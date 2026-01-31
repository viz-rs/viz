pub mod elements;
pub mod properties;

mod geometry;
pub mod style;

#[cfg(feature = "picking")]
mod picking;

pub mod computed;
mod layout;
pub mod measure;
pub mod render;
mod stack;
mod systems;
mod utils;

use bevy_camera::{
    CameraUpdateSystems,
    visibility::{Visibility, VisibilityClass, add_visibility_class},
};
use bevy_ecs::schedule::{IntoScheduleConfigs, SystemSet};
use bevy_render::{extract_resource::ExtractResourcePlugin, sync_world::SyncToRenderWorld};
use bevy_transform::TransformSystems;
#[cfg(feature = "picking")]
use picking::UiPickingPlugin;

use bevy_app::{AnimationSystems, App, Plugin, PostUpdate};

use render::plugin::UiRenderPlugin;

use crate::{
    elements::{image, node::Node, text},
    layout::{UiLayoutEngine, ui_layout_system, ui_target_info_system},
    stack::{UiStackMap, ui_stack_system},
    systems::ui_clipping_system,
};

// Marks systems that can be ambiguous with [`widget::text_system`] if the `bevy_text` feature is enabled.
// See https://github.com/bevyengine/bevy/pull/11391 for more details.
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
struct AmbiguousWithText;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
struct AmbiguousWithUpdateText2dLayout;

/// The label enum labeling the types of systems in the Bevy UI
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum UiSystems {
    /// After this label, input interactions with UI entities have been updated for this frame.
    ///
    /// Runs in [`PreUpdate`].
    Focus,
    /// All UI systems in [`PostUpdate`] will run in or after this label.
    Prepare,
    // /// Propagate UI component values needed by layout.
    // Propagate,
    /// Update content requirements before layout.
    Content,
    /// After this label, the ui layout state has been updated.
    ///
    /// Runs in [`PostUpdate`].
    Layout,
    /// UI systems ordered after [`UiSystems::Layout`].
    ///
    /// Runs in [`PostUpdate`].
    PostLayout,
    /// After this label, the [`UiStack`] resource has been updated.
    ///
    /// Runs in [`PostUpdate`].
    Stack,
}

pub struct MoonPlugin;

impl Plugin for MoonPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "picking")]
        app.add_plugins(UiPickingPlugin);

        app.register_required_components::<Node, Visibility>()
            .register_required_components::<Node, VisibilityClass>()
            .register_required_components::<Node, SyncToRenderWorld>();
        app.world_mut()
            .register_component_hooks::<Node>()
            .on_add(add_visibility_class::<Node>);

        app.init_resource::<UiLayoutEngine>()
            .init_resource::<UiStackMap>()
            .add_plugins(ExtractResourcePlugin::<UiStackMap>::default());

        app.configure_sets(
            PostUpdate,
            (
                CameraUpdateSystems,
                UiSystems::Prepare.after(AnimationSystems),
                UiSystems::Content,
                UiSystems::Layout,
                UiSystems::PostLayout,
            )
                .chain(),
        );

        let ui_layout_system_config = ui_layout_system
            .in_set(UiSystems::Layout)
            .before(TransformSystems::Propagate);

        let ui_layout_system_config = ui_layout_system_config
            // Text and Text2D operate on disjoint sets of entities
            .ambiguous_with(bevy_sprite::update_text2d_layout)
            .ambiguous_with(bevy_text::detect_text_needs_rerender::<bevy_sprite::Text2d>);

        app.add_systems(
            PostUpdate,
            (
                ui_target_info_system.in_set(UiSystems::Prepare),
                ui_stack_system.in_set(UiSystems::Stack),
                ui_layout_system_config
                    .in_set(UiSystems::Layout)
                    .ambiguous_with(text::measure_text_system)
                    .ambiguous_with(ui_clipping_system)
                    .ambiguous_with(ui_layout_system)
                    // .ambiguous_with(widget::update_viewport_render_target_size)
                    .in_set(AmbiguousWithText),
                ui_clipping_system.after(TransformSystems::Propagate),
                image::update_image_content_size_system
                    .in_set(UiSystems::Content)
                    .in_set(AmbiguousWithText)
                    .in_set(AmbiguousWithUpdateText2dLayout),
            ),
        );

        app.add_systems(
            PostUpdate,
            (
                (
                    bevy_text::detect_text_needs_rerender::<text::Text>,
                    text::measure_text_system,
                )
                    .chain()
                    .in_set(UiSystems::Content)
                    // Text and Text2d are independent.
                    .ambiguous_with(bevy_text::detect_text_needs_rerender::<bevy_sprite::Text2d>)
                    // Potential conflict: `Assets<Image>`
                    // Since both systems will only ever insert new [`Image`] assets,
                    // they will never observe each other's effects.
                    .ambiguous_with(bevy_sprite::update_text2d_layout)
                    // We assume Text is on disjoint UI entities to ImageNode and UiTextureAtlasImage
                    // FIXME: Add an archetype invariant for this https://github.com/bevyengine/bevy/issues/1481.
                    .ambiguous_with(image::update_image_content_size_system),
                text::text_system
                    .in_set(UiSystems::PostLayout)
                    .after(bevy_text::free_unused_font_atlases_system)
                    .before(bevy_asset::AssetEventSystems)
                    // Text2d and bevy_ui text are entirely on separate entities
                    .ambiguous_with(bevy_text::detect_text_needs_rerender::<bevy_sprite::Text2d>)
                    .ambiguous_with(bevy_sprite::update_text2d_layout)
                    .ambiguous_with(bevy_sprite::calculate_bounds_text2d),
            ),
        );

        app.configure_sets(
            PostUpdate,
            AmbiguousWithText.ambiguous_with(text::text_system),
        );

        app.configure_sets(
            PostUpdate,
            AmbiguousWithUpdateText2dLayout.ambiguous_with(bevy_sprite::update_text2d_layout),
        );

        app.add_plugins(UiRenderPlugin);
    }
}
