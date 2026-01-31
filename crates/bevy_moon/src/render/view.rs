use bevy_camera::{Camera, Camera2d, Camera3d};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::{Has, Or, With},
    system::{Commands, Local, Query, ResMut},
};
use bevy_math::{URect, UVec4};
use bevy_platform::collections::HashSet;
use bevy_render::{
    Extract,
    render_phase::ViewSortedRenderPhases,
    sync_world::{RenderEntity, TemporaryRenderEntity},
    view::{ExtractedView, Hdr, Msaa, RetainedViewEntity},
};
use bevy_sprite_render::Mesh2dPipelineKey;
use bevy_transform::components::GlobalTransform;

use crate::render::transparent::TransparentUi;

/// A render-world component that lives on the main render target view and
/// specifies the corresponding moon ui view.
///
/// For example, if moon ui is being rendered to a 3D camera, this component lives on
/// the 3D camera and contains the entity corresponding to the moon ui view.
///
/// Entity id of the temporary render entity with the corresponding extracted moon ui view.
#[derive(Component, Debug)]
pub struct MoonUiCameraView(pub Entity);

/// A render-world component that lives on the moon ui view and specifies the
/// corresponding main render target view.
///
/// For example, if moon ui is being rendered to a 3D camera, this component
/// lives on the moon ui view and contains the entity corresponding to the 3D camera.
///
/// This is the inverse of [`MoonUiCameraView`].
#[derive(Component, Debug)]
pub struct MoonUiViewTarget(pub Entity);

/// Caches the mesh key for the moon ui view.
#[derive(Component, Debug)]
pub struct MoonUiOptions(pub Mesh2dPipelineKey);

/// Extracts all moon ui with a camera into the render world.
pub fn extract_camera_views(
    cameras: Extract<
        Query<
            (
                Entity,
                RenderEntity,
                &GlobalTransform,
                &Camera,
                // options
                (Has<Hdr>, &Msaa),
            ),
            Or<(With<Camera2d>, With<Camera3d>)>,
        >,
    >,
    mut commands: Commands,
    mut render_phases: ResMut<ViewSortedRenderPhases<TransparentUi>>,
    mut live_entities: Local<HashSet<RetainedViewEntity>>,
) {
    live_entities.clear();

    for (main_entity, render_entity, &transform, camera, (hdr, msaa)) in &cameras {
        // Ignore inactive cameras.
        if !camera.is_active {
            commands
                .get_entity(render_entity)
                .expect("Camera entity wasn't synced.")
                .remove::<MoonUiCameraView>();
            continue;
        }

        let (
            Some(URect {
                min: viewport_origin,
                ..
            }),
            Some(viewport_size),
        ) = (
            camera.physical_viewport_rect(),
            camera.physical_viewport_size(),
        )
        else {
            continue;
        };

        // @TODO(fundon): should be a safe value
        const MOON_UI_CAMERA_SUBVIEW: u32 = u32::MAX;

        // We use `MOON_UI_CAMERA_SUBVIEW` here so as not to conflict with other ui views.
        // *. Main 3D or 2D camera, which will have subview index 0.
        // *. Bevy UI, which will have subview index `UI_CAMERA_SUBVIEW = 1`.
        // *. Bevy egui, which will have subview index `EGUI_CAMERA_SUBVIEW = 2095931312`.
        let retained_view_entity =
            RetainedViewEntity::new(main_entity.into(), None, MOON_UI_CAMERA_SUBVIEW);

        let mesh_key =
            Mesh2dPipelineKey::from_msaa_samples(msaa.samples()) | Mesh2dPipelineKey::from_hdr(hdr);

        // Creates the UI view.
        let ui_camera_view = commands
            .spawn((
                ExtractedView {
                    retained_view_entity,
                    clip_from_view: camera.clip_from_view(),
                    world_from_view: transform,
                    clip_from_world: None,
                    viewport: UVec4::new(
                        viewport_origin.x,
                        viewport_origin.y,
                        viewport_size.x,
                        viewport_size.y,
                    ),
                    hdr,
                    invert_culling: false,
                    color_grading: Default::default(),
                },
                // Link to the main camera view.
                MoonUiViewTarget(render_entity),
                TemporaryRenderEntity,
            ))
            .id();

        commands
            .get_entity(render_entity)
            .expect("Camera entity wasn't synced.")
            // Link from the main 2D/3D camera view to the moon ui view.
            .insert(MoonUiCameraView(ui_camera_view))
            .insert(MoonUiOptions(mesh_key));

        render_phases.insert_or_clear(retained_view_entity);
        live_entities.insert(retained_view_entity);
    }

    render_phases.retain(|camera_entity, _| live_entities.contains(camera_entity));
}
