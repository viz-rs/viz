use bevy_asset::Assets;
use bevy_camera::Camera;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    change_detection::DetectChanges,
    component::Component,
    entity::Entity,
    query::With,
    reflect::ReflectComponent,
    system::{Query, Res, ResMut},
    world::{Mut, Ref},
};
use bevy_image::{Image, TextureAtlasLayout};
use bevy_math::Vec2;
use bevy_reflect::{Reflect, prelude::ReflectDefault};
use bevy_text::{
    ComputedTextBlock, CosmicFontSystem, Font, FontAtlasSet, FontHinting, LineBreak, LineHeight,
    SwashCache, TextBounds, TextColor, TextError, TextFont, TextLayout, TextLayoutInfo,
    TextMeasureInfo, TextPipeline, TextReader, TextRoot, TextSpanAccess,
};

use crate::{
    computed::{ComputedNode, ComputedTargetInfo},
    elements::node::Node,
    measure::{ContentSize, FixedMeasure, Measure, MeasureArgs},
    stack::UiStackMap,
};

/// UI text system flags.
///
/// Used internally by [`measure_text_system`] and [`text_system`] to schedule text for processing.
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component, Default, Debug, Clone)]
pub struct TextNodeFlags {
    /// If set then a new measure function for the text node will be created.
    needs_measure_fn: bool,
    /// If set then the text will be recomputed.
    needs_recompute: bool,
}

impl Default for TextNodeFlags {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl TextNodeFlags {
    pub const DEFAULT: Self = Self {
        needs_measure_fn: true,
        needs_recompute: true,
    };
}

#[derive(Component, Debug, Default, Clone, Deref, DerefMut, Reflect, PartialEq)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
#[require(
    Node,
    TextLayout,
    TextFont,
    TextColor,
    LineHeight,
    TextNodeFlags,
    ContentSize,
    // Disable hinting.
    // UI text is normally pixel-aligned, but with hinting enabled sometimes the text bounds are miscalculated slightly.
    FontHinting::Disabled,
)]
pub struct Text(pub String);

impl Text {
    /// Makes a new text component.
    pub fn new(text: impl Into<String>) -> Self {
        Self(text.into())
    }
}

impl TextRoot for Text {}

impl TextSpanAccess for Text {
    fn read_span(&self) -> &str {
        self.as_str()
    }
    fn write_span(&mut self) -> &mut String {
        &mut *self
    }
}

impl From<&str> for Text {
    fn from(value: &str) -> Self {
        Self(String::from(value))
    }
}

impl From<String> for Text {
    fn from(value: String) -> Self {
        Self(value)
    }
}

/// Text measurement for UI layout. See [`NodeMeasure`].
pub struct TextMeasure {
    // All values are scaled in text measure info.
    pub info: TextMeasureInfo,
    pub zoom_factor: f32,
}

impl TextMeasure {
    /// Checks if the cosmic text buffer is needed for measuring the text.
    #[inline]
    pub const fn needs_buffer(height: Option<f32>, available_width: taffy::AvailableSpace) -> bool {
        height.is_none() && matches!(available_width, taffy::AvailableSpace::Definite(_))
    }
}

impl Measure for TextMeasure {
    /// Returns the size of the text.
    fn measure(&mut self, args: MeasureArgs, _style: &taffy::Style) -> Vec2 {
        use crate::geometry::map_fn;

        // The measure arguments have been scaled by the target scale factor.
        let MeasureArgs {
            known_dimensions: taffy::Size { width, height },
            available_space:
                taffy::Size {
                    width: available_width,
                    ..
                },
            font_system,
            text_buffer,
        } = args;

        let scale_fn = map_fn(|v, s| v * s, self.zoom_factor);

        let width = width.map(scale_fn);
        let height = height.map(scale_fn);

        // Text info has been scaled.
        let min = self.info.min;
        let max = self.info.max;

        let x = match width {
            Some(x) => x,
            None => match available_width {
                taffy::AvailableSpace::MinContent => min.x,
                taffy::AvailableSpace::MaxContent => max.x,
                taffy::AvailableSpace::Definite(x) => {
                    // It is possible for the "min content width" to be larger than
                    // the "max content width" when soft-wrapping right-aligned text
                    // and possibly other situations.

                    scale_fn(x).max(min.x).min(max.x)
                }
            },
        };

        let size = match height {
            Some(y) => Vec2::new(x, y),
            None => match available_width {
                taffy::AvailableSpace::MinContent => Vec2::new(x, min.y),
                taffy::AvailableSpace::MaxContent => Vec2::new(x, max.y),
                taffy::AvailableSpace::Definite(_) => match text_buffer {
                    Some(buffer) => {
                        self.info
                            .compute_size(TextBounds::new_horizontal(x), buffer, font_system)
                    }
                    None => {
                        tracing::error!("text measure failed, buffer is missing");
                        Vec2::ZERO
                    }
                },
            },
        };

        (size / self.zoom_factor).ceil()
    }

    fn get_text_buffer<'a>(
        &mut self,
        query: &'a mut Query<&mut ComputedTextBlock>,
    ) -> Option<&'a mut ComputedTextBlock> {
        query.get_mut(self.info.entity).map(Mut::into_inner).ok()
    }
}

pub fn measure_text_system(
    fonts: Res<Assets<Font>>,
    ui_stack_map: Res<UiStackMap>,
    camera_query: Query<Ref<ComputedTargetInfo>, With<Camera>>,
    mut text_query: Query<
        (
            Entity,
            Ref<TextLayout>,
            Ref<FontHinting>,
            &mut ComputedTextBlock,
            &mut TextNodeFlags,
            &mut ContentSize,
        ),
        With<Node>,
    >,
    mut text_reader: TextReader<Text>,
    mut text_pipeline: ResMut<TextPipeline>,
    mut font_system: ResMut<CosmicFontSystem>,
) {
    for (entity, text_layout, hinting, mut computed_text_block, mut text_flags, mut content_size) in
        text_query.iter_mut()
    {
        let target_info = ui_stack_map
            .iter()
            .find_map(|stack| {
                stack
                    .1
                    .bitset
                    .contains(entity.index_u32() as usize)
                    .then_some(*stack.0)
            })
            .iter()
            .find_map(|&camera_entity| camera_query.get(camera_entity).ok());

        let (scale_factor, zoom_factor, is_changed) = extract_values(target_info);
        let applied_scale_factor = normalize_scale_factor(scale_factor, zoom_factor);

        if is_changed {
            text_flags.needs_measure_fn = true;
        }

        let should_measure = computed_text_block.needs_rerender()
            || text_flags.needs_measure_fn
            || content_size.is_added()
            || hinting.is_changed();

        if !should_measure {
            continue;
        }

        match text_pipeline.create_text_measure(
            entity,
            &fonts,
            text_reader.iter(entity),
            applied_scale_factor as f64,
            &text_layout,
            &mut computed_text_block,
            &mut font_system,
            *hinting,
        ) {
            Ok(measure) => {
                if text_layout.linebreak == LineBreak::NoWrap {
                    content_size.set(FixedMeasure { size: measure.max });
                } else {
                    content_size.set(TextMeasure {
                        info: measure,
                        // In measure, they have been scaled by the target `scale_factor`,
                        // so just need to scale them with `zoom_factor`.
                        zoom_factor: applied_scale_factor / scale_factor,
                    });
                }

                // Text measure func created successfully, so set `TextNodeFlags` to schedule a recompute
                text_flags.needs_measure_fn = false;
                text_flags.needs_recompute = true;
            }
            Err(TextError::NoSuchFont) => {
                // Try again next frame
                text_flags.needs_measure_fn = true;
            }
            Err(
                e @ (TextError::FailedToAddGlyph(_)
                | TextError::FailedToGetGlyphImage(_)
                | TextError::MissingAtlasLayout
                | TextError::MissingAtlasTexture
                | TextError::InconsistentAtlasState),
            ) => {
                panic!("Fatal error when processing text: {e}.");
            }
        };
    }
}

pub fn text_system(
    camera_query: Query<Ref<ComputedTargetInfo>, With<Camera>>,
    mut text_query: Query<(
        Entity,
        Ref<ComputedNode>,
        &TextLayout,
        &mut TextLayoutInfo,
        &mut ComputedTextBlock,
        &mut TextNodeFlags,
    )>,
    text_font_query: Query<&TextFont>,
    ui_stack_map: Res<UiStackMap>,
    mut textures: ResMut<Assets<Image>>,
    mut texture_atlases: ResMut<Assets<TextureAtlasLayout>>,
    mut font_atlas_set: ResMut<FontAtlasSet>,
    mut text_pipeline: ResMut<TextPipeline>,
    mut font_system: ResMut<CosmicFontSystem>,
    mut swash_cache: ResMut<SwashCache>,
) {
    for (
        entity,
        computed_node,
        text_layout,
        mut text_layout_info,
        mut computed_text_block,
        mut text_flags,
    ) in text_query.iter_mut()
    {
        let target_info = ui_stack_map
            .iter()
            .find_map(|stack| {
                stack
                    .1
                    .bitset
                    .contains(entity.index_u32() as usize)
                    .then_some(*stack.0)
            })
            .iter()
            .find_map(|&camera_entity| camera_query.get(camera_entity).ok());

        let (scale_factor, zoom_factor, is_changed) = extract_values(target_info);
        let applied_scale_factor = normalize_scale_factor(scale_factor, zoom_factor);

        if is_changed {
            text_flags.needs_recompute = true;
        }

        let should_update = computed_node.is_changed()
            || text_flags.needs_recompute
            || applied_scale_factor != text_layout_info.scale_factor;

        if !should_update {
            continue;
        }

        // Skip the text node if it is waiting for a new measure func
        if text_flags.needs_measure_fn {
            continue;
        }

        let physical_node_size = if text_layout.linebreak == LineBreak::NoWrap {
            TextBounds::UNBOUNDED
        } else {
            // The computed is in logical pixels, so we need to scale it up by the applied scale factor.
            TextBounds::from(computed_node.size * applied_scale_factor)
        };

        match text_pipeline.update_text_layout_info(
            &mut text_layout_info,
            text_font_query,
            applied_scale_factor as f64,
            &mut font_atlas_set,
            &mut texture_atlases,
            &mut textures,
            &mut computed_text_block,
            &mut font_system,
            &mut swash_cache,
            physical_node_size,
            text_layout.justify,
        ) {
            Err(TextError::NoSuchFont) => {
                // There was an error processing the text layout, try again next frame
                text_flags.needs_recompute = true;
            }
            Err(
                e @ (TextError::FailedToAddGlyph(_)
                | TextError::FailedToGetGlyphImage(_)
                | TextError::MissingAtlasLayout
                | TextError::MissingAtlasTexture
                | TextError::InconsistentAtlasState),
            ) => {
                panic!("Fatal error when processing text: {e}.");
            }
            Ok(()) => {
                text_layout_info.scale_factor = applied_scale_factor;
                text_layout_info.size *= applied_scale_factor.recip();

                text_flags.needs_recompute = false;
            }
        }
    }
}

/// Extracts the scale factor, zoom factor, and change flag from the target info.
fn extract_values(target_info: Option<Ref<ComputedTargetInfo>>) -> (f32, f32, bool) {
    match target_info {
        Some(t) => (t.scale_factor, t.zoom_factor, t.is_changed()),
        None => (1.0, 1.0, false),
    }
}

// @TODO(fundon): should be an configurable value and enabled on native platforms.
// There is a memory leak on web.
fn normalize_scale_factor(scale_factor: f32, zoom_factor: f32) -> f32 {
    let base = (scale_factor * zoom_factor.recip().round()).max(scale_factor);

    #[cfg(not(target_arch = "wasm32"))]
    let min = 8.0;

    #[cfg(target_arch = "wasm32")]
    let min = scale_factor;

    base.min(min)
}
