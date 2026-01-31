use bevy_app::{App, Plugin};
use bevy_asset::{AssetEvent, AssetId};
use bevy_color::ColorToComponents;
use bevy_ecs::{
    change_detection::Res,
    entity::Entity,
    query::With,
    schedule::IntoScheduleConfigs,
    system::{Commands, Local, Query, ResMut},
};
use bevy_math::{FloatOrd, Rect, Vec2};
use bevy_render::{
    ExtractSchedule, Render, RenderApp, RenderStartup, RenderSystems,
    render_asset::RenderAssets,
    render_phase::{
        AddRenderCommand, DrawFunctions, PhaseItem, PhaseItemExtraIndex, ViewSortedRenderPhases,
        sort_phase_system,
    },
    render_resource::{BindGroupEntries, PipelineCache, SpecializedRenderPipelines},
    renderer::{RenderDevice, RenderQueue},
    sync_world::MainEntity,
    texture::GpuImage,
    view::{ExtractedView, ViewUniforms},
};
use bevy_shader::load_shader_library;
use bevy_sprite_render::SpriteAssetEvents;
use bevy_utils::default;

use crate::{
    geometry::VEC2_FLIP_Y,
    render::{
        box_shadow::BoxShadowPlugin,
        extract::{
            ExtractedGlyph, ExtractedUiItem, ExtractedUiNodes, extract_images, extract_node_styles,
            extract_texts,
        },
        flags::ShaderFlags,
        graph::add_moon_ui_subgraph,
        pipeline::{UiPipeline, UiPipelineKey, init_ui_pipeline},
        quad,
        systems::RenderUiSystems,
        transparent::TransparentUi,
        ui::{DrawUi, ImageNodeBindGroups, UiBatch, UiMeta, UiVertex},
        view::{MoonUiCameraView, MoonUiOptions, MoonUiViewTarget, extract_camera_views},
    },
    stack::UiStackMap,
};

#[derive(Default)]
pub struct UiRenderPlugin;

impl Plugin for UiRenderPlugin {
    fn build(&self, app: &mut App) {
        load_shader_library!(app, "ui.wgsl");

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<UiMeta>()
            .init_resource::<ImageNodeBindGroups>()
            .init_resource::<ExtractedUiNodes>()
            .allow_ambiguous_resource::<ExtractedUiNodes>()
            .init_resource::<SpecializedRenderPipelines<UiPipeline>>()
            .init_resource::<DrawFunctions<TransparentUi>>()
            .init_resource::<ViewSortedRenderPhases<TransparentUi>>()
            .add_render_command::<TransparentUi, DrawUi>()
            .configure_sets(
                ExtractSchedule,
                (
                    RenderUiSystems::ExtractCameraViews,
                    RenderUiSystems::ExtractBoxShadows,
                    RenderUiSystems::ExtractNodeStyles,
                    RenderUiSystems::ExtractImages,
                    RenderUiSystems::ExtractTexts,
                )
                    .chain(),
            )
            .add_systems(RenderStartup, init_ui_pipeline)
            .add_systems(
                ExtractSchedule,
                (
                    extract_camera_views.in_set(RenderUiSystems::ExtractCameraViews),
                    extract_node_styles.in_set(RenderUiSystems::ExtractNodeStyles),
                    extract_images.in_set(RenderUiSystems::ExtractImages),
                    extract_texts.in_set(RenderUiSystems::ExtractTexts),
                ),
            )
            .add_systems(
                Render,
                (
                    queue_nodes.in_set(RenderSystems::Queue),
                    sort_phase_system::<TransparentUi>.in_set(RenderSystems::PhaseSort),
                    prepare_nodes.in_set(RenderSystems::PrepareBindGroups),
                ),
            );

        add_moon_ui_subgraph(render_app);

        app.add_plugins(BoxShadowPlugin);
    }
}

fn queue_nodes(
    render_targets: Query<(MainEntity, &MoonUiCameraView, &MoonUiOptions)>,
    render_views: Query<&ExtractedView, With<MoonUiViewTarget>>,
    extracted_ui_nodes: Res<ExtractedUiNodes>,
    ui_stack_map: Res<UiStackMap>,
    ui_pipeline: Res<UiPipeline>,
    pipeline_cache: Res<PipelineCache>,
    draw_functions: Res<DrawFunctions<TransparentUi>>,
    mut pipelines: ResMut<SpecializedRenderPipelines<UiPipeline>>,
    mut render_phases: ResMut<ViewSortedRenderPhases<TransparentUi>>,
) {
    let draw_function = draw_functions.read().id::<DrawUi>();

    for (extracted_index, node) in extracted_ui_nodes.nodes.iter().enumerate() {
        let Some(ui_stack) = ui_stack_map.get(&node.camera_entity) else {
            return;
        };
        let Some((_camera_entity, &MoonUiCameraView(ui_camera_view), &MoonUiOptions(mesh_key))) =
            render_targets.iter().find(|r| r.0 == node.camera_entity)
        else {
            continue;
        };
        let Some(extracted_view) = render_views.get(ui_camera_view).ok() else {
            continue;
        };
        let Some(render_phase) = render_phases.get_mut(&extracted_view.retained_view_entity) else {
            continue;
        };

        let pipeline = pipelines.specialize(
            &pipeline_cache,
            &ui_pipeline,
            UiPipelineKey {
                mesh_key,
                // @TODO(fundon): add an `UiAntiAlias` option
                anti_alias: true,
            },
        );

        let view_index = node.main_entity.index_u32() as usize;
        if !ui_stack.bitset.contains(view_index) {
            continue;
        }

        let index = node.z_order;

        render_phase.add(TransparentUi {
            pipeline,
            draw_function,
            extracted_index,
            entity: (node.render_entity, node.main_entity),
            sort_key: FloatOrd(index),
            batch_range: 0..0,
            extra_index: PhaseItemExtraIndex::None,
            indexed: true,
        });
    }
}

fn prepare_nodes(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    pipeline_cache: Res<PipelineCache>,
    view_uniforms: Res<ViewUniforms>,
    ui_pipeline: Res<UiPipeline>,
    gpu_images: Res<RenderAssets<GpuImage>>,
    events: Res<SpriteAssetEvents>,
    mut commands: Commands,
    mut ui_meta: ResMut<UiMeta>,
    mut extracted_ui_nodes: ResMut<ExtractedUiNodes>,
    mut image_bind_groups: ResMut<ImageNodeBindGroups>,
    mut render_phases: ResMut<ViewSortedRenderPhases<TransparentUi>>,
    mut ui_stack_map: ResMut<UiStackMap>,
    mut previous_len: Local<usize>,
) {
    // If an image has changed, the GpuImage has (probably) changed
    for event in &events.images {
        match event {
          AssetEvent::Added { .. } |
          AssetEvent::Unused { .. } |
          // Images don't have dependencies
          AssetEvent::LoadedWithDependencies { .. } => {}
          AssetEvent::Modified { id } | AssetEvent::Removed { id } => {
              image_bind_groups.values.remove(id);
          }
      };
    }

    let Some(view_binding) = view_uniforms.uniforms.binding() else {
        return;
    };

    ui_meta.vertices.clear();
    ui_meta.indices.clear();
    ui_meta.view_bind_group = Some(render_device.create_bind_group(
        "ui_view_bind_group",
        &pipeline_cache.get_bind_group_layout(&ui_pipeline.view_layout),
        &BindGroupEntries::single(view_binding),
    ));

    let mut batches: Vec<(Entity, UiBatch)> = Vec::with_capacity(*previous_len);

    // Buffer indexes
    let mut vertices_index = 0;
    let mut indices_index = 0;

    for render_phase in render_phases.values_mut() {
        // let mut batch_item_index = 0;
        let mut batch_image_handle = AssetId::invalid();

        for (item_index, item) in render_phase.items.iter_mut().enumerate() {
            let Some(extracted_ui_node) = extracted_ui_nodes
                .nodes
                .get(item.extracted_index)
                .filter(|n| item.entity() == n.render_entity)
            else {
                batch_image_handle = AssetId::invalid();
                continue;
            };

            let mut should_batch = false;
            let mut existing_batch = batches.last_mut();

            if batch_image_handle == AssetId::invalid()
                || existing_batch.is_none()
                || (batch_image_handle != AssetId::default()
                    && extracted_ui_node.image != AssetId::default()
                    && batch_image_handle != extracted_ui_node.image)
            {
                if let Some(gpu_image) = gpu_images.get(extracted_ui_node.image) {
                    // batch_item_index = item_index;
                    should_batch = true;
                    batch_image_handle = extracted_ui_node.image;

                    let new_batch = UiBatch {
                        range: vertices_index..vertices_index,
                        image: extracted_ui_node.image,
                    };

                    batches.push((item.entity(), new_batch));

                    image_bind_groups
                        .values
                        .entry(batch_image_handle)
                        .or_insert_with(|| {
                            render_device.create_bind_group(
                                "ui_material_bind_group",
                                &pipeline_cache.get_bind_group_layout(&ui_pipeline.image_layout),
                                &BindGroupEntries::sequential((
                                    &gpu_image.texture_view,
                                    &gpu_image.sampler,
                                )),
                            )
                        });

                    existing_batch = batches.last_mut();
                } else {
                    continue;
                }
            } else if batch_image_handle == AssetId::default()
                && extracted_ui_node.image != AssetId::default()
            {
                if let Some(ref mut existing_batch) = existing_batch
                    && let Some(gpu_image) = gpu_images.get(extracted_ui_node.image)
                {
                    batch_image_handle = extracted_ui_node.image;
                    existing_batch.1.image = extracted_ui_node.image;

                    image_bind_groups
                        .values
                        .entry(batch_image_handle)
                        .or_insert_with(|| {
                            render_device.create_bind_group(
                                "ui_material_bind_group",
                                &pipeline_cache.get_bind_group_layout(&ui_pipeline.image_layout),
                                &BindGroupEntries::sequential((
                                    &gpu_image.texture_view,
                                    &gpu_image.sampler,
                                )),
                            )
                        });
                } else {
                    continue;
                }
            }

            let clip = extracted_ui_node.clip;

            match &extracted_ui_node.item {
                &ExtractedUiItem::Node {
                    color,
                    size,
                    atlas_scaling,
                    flip_x,
                    flip_y,
                    corner_radii,
                    border,
                    flags,
                } => {
                    let transform = extracted_ui_node.transform;

                    // uv coordinates
                    let mut uvs = quad::UVS;

                    // local positions
                    let mut points = quad::VERTEX_POSITIONS.map(|pos| pos * size);

                    // world positions
                    let mut positions =
                        points.map(|pos| transform.transform_point3(pos.extend(0.0)));

                    let mut positions_diff = [Vec2::ZERO; 4];

                    // creates a rectangle from top-right to bottom-left
                    let rect = Rect::from_corners(positions[0].truncate(), positions[2].truncate());

                    let mut clipped_rect = Option::<Rect>::None;

                    // needs to consider rotation and scale cases
                    // needs to calculate multiple points: n = 3 or n > 4
                    if let Some(clip_rect) = clip
                        && size.cmpne(Vec2::ZERO).all()
                    {
                        let intersected = clip_rect.intersect(rect);

                        // out of bounds
                        if intersected.is_empty() {
                            continue;
                        }

                        // intersects
                        if intersected.size() != rect.size() {
                            // counter-clockwise order: [top-right, top-left, bottom-left, bottom-right]
                            let corners = [
                                intersected.max,
                                Vec2::new(intersected.min.x, intersected.max.y),
                                intersected.min,
                                Vec2::new(intersected.max.x, intersected.min.y),
                            ];

                            for i in 0..4 {
                                positions_diff[i] = corners[i] - positions[i].truncate();
                                positions[i] = corners[i].extend(0.0);
                            }

                            clipped_rect = Some(intersected);
                        }
                    }

                    // swaps top-right <-> top-left, bottom-right <-> bottom-left
                    if flip_x {
                        positions.swap(0, 1);
                        positions.swap(2, 3);

                        positions_diff.swap(0, 1);
                        positions_diff.swap(2, 3);

                        for i in 0..4 {
                            positions_diff[i].x *= -1.0;
                        }
                    }
                    // swaps top-right <-> bottom-right, top-left <-> bottom-left
                    if flip_y {
                        positions.swap(0, 3);
                        positions.swap(1, 2);

                        positions_diff.swap(0, 3);
                        positions_diff.swap(1, 2);

                        for i in 0..4 {
                            positions_diff[i].y *= -1.0;
                        }
                    }

                    for i in 0..4 {
                        points[i] += positions_diff[i];
                    }

                    if clipped_rect.is_some() && flags == ShaderFlags::TEXTURED {
                        let image = gpu_images
                            .get(extracted_ui_node.image)
                            .expect("Image was checked during batching and should still exist");
                        // Rescale atlases. This is done here because we need texture data that might not be available in Extract.
                        let atlas_extent = atlas_scaling
                            .map(|scaling| image.size_2d().as_vec2() * scaling)
                            .unwrap_or(size);

                        let scale_factor = transform.to_scale_rotation_translation().0.truncate();

                        let image_rect = Rect::from_corners(Vec2::ZERO, size);

                        for i in 0..4 {
                            // flip Y-axis
                            positions_diff[i].y *= -1.0;
                            // inverse scale factor
                            positions_diff[i] /= scale_factor;
                        }

                        let top_right =
                            Vec2::new(image_rect.max.x, image_rect.min.y) + positions_diff[0];
                        let top_left = image_rect.min + positions_diff[1];
                        let bottom_left =
                            Vec2::new(image_rect.min.x, image_rect.max.y) + positions_diff[2];
                        let bottom_right = image_rect.max + positions_diff[3];

                        uvs = [top_right, top_left, bottom_left, bottom_right]
                            .map(|pos| pos / atlas_extent);
                    }

                    let vertex = UiVertex {
                        size: size.into(),
                        flags: flags.bits(),
                        color: color.to_f32_array(),
                        border: border.to_array(),
                        radius: corner_radii.to_array(),
                        ..Default::default()
                    };

                    for i in 0..4 {
                        ui_meta.vertices.push(UiVertex {
                            uv: uvs[i].into(),
                            point: points[i].into(),
                            position: positions[i].into(),
                            flags: vertex.flags | ShaderFlags::CORNERS[i].bits(),
                            ..vertex
                        });
                    }

                    for i in quad::INDICES {
                        ui_meta.indices.push(indices_index + i);
                    }

                    vertices_index += 6;
                    indices_index += 4;
                }
                ExtractedUiItem::Glyphs { range } => {
                    let image = gpu_images
                        .get(extracted_ui_node.image)
                        .expect("Image was checked during batching and should still exist");

                    let atlas_extent = image.size_2d().as_vec2();

                    let transform = extracted_ui_node.transform;

                    for &ExtractedGlyph {
                        color,
                        translation,
                        rect: glyph_rect,
                    } in &extracted_ui_nodes.glyphs[range.clone()]
                    {
                        let size = glyph_rect.size();

                        // local positions
                        let points = quad::VERTEX_POSITIONS.map(|pos| pos * size * VEC2_FLIP_Y);

                        // world positions
                        let mut positions = points
                            .map(|pos| transform.transform_point3((translation + pos).extend(0.0)));

                        let mut positions_diff = [Vec2::ZERO; 4];

                        // creates a rectangle from top-right to bottom-left
                        let rect =
                            Rect::from_corners(positions[0].truncate(), positions[2].truncate());

                        let mut clipped_rect = Option::<Rect>::None;

                        if let Some(clip_rect) = clip
                            && size.cmpne(Vec2::ZERO).all()
                        {
                            let intersected = clip_rect.intersect(rect);

                            // out of bounds
                            if intersected.is_empty() {
                                continue;
                            }

                            // intersects
                            if intersected.size() != rect.size() {
                                // counter-clockwise order: [top-right, top-left, bottom-left, bottom-right]
                                let corners = [
                                    intersected.max,
                                    Vec2::new(intersected.min.x, intersected.max.y),
                                    intersected.min,
                                    Vec2::new(intersected.max.x, intersected.min.y),
                                ];

                                for i in 0..4 {
                                    positions_diff[i] = corners[i] - positions[i].truncate();
                                    positions[i] = corners[i].extend(0.0);
                                }

                                clipped_rect = Some(intersected);
                            }
                        }

                        if clipped_rect.is_some() {
                            let scale_factor =
                                transform.to_scale_rotation_translation().0.truncate();

                            for i in 0..4 {
                                // flip Y-axis
                                positions_diff[i].y *= -1.0;
                                // inverse scale factor
                                positions_diff[i] /= scale_factor;
                            }
                        }

                        let top_right =
                            Vec2::new(glyph_rect.max.x, glyph_rect.min.y) + positions_diff[0];
                        let top_left = glyph_rect.min + positions_diff[1];
                        let bottom_left =
                            Vec2::new(glyph_rect.min.x, glyph_rect.max.y) + positions_diff[2];
                        let bottom_right = glyph_rect.max + positions_diff[3];

                        // ignores rotation cases
                        // should add a clip rect to shader
                        if clipped_rect.is_some()
                            && (top_right.x != bottom_right.x
                                || top_right.y != top_left.y
                                || bottom_left.y != bottom_right.y
                                || top_left.x != bottom_left.x)
                        {
                            continue;
                        }

                        let uvs = [top_right, top_left, bottom_left, bottom_right]
                            .map(|pos| pos / atlas_extent);

                        let vertex = UiVertex {
                            size: size.into(),
                            color: color.to_f32_array(),
                            flags: ShaderFlags::TEXTURED.bits(),
                            ..default()
                        };

                        for i in 0..4 {
                            ui_meta.vertices.push(UiVertex {
                                uv: uvs[i].into(),
                                position: positions[i].into(),
                                flags: vertex.flags | ShaderFlags::CORNERS[i].bits(),
                                ..vertex
                            });
                        }

                        for i in quad::INDICES {
                            ui_meta.indices.push(indices_index + i as u32);
                        }

                        vertices_index += 6;
                        indices_index += 4;
                    }
                }
            }

            if let Some(batch) = existing_batch {
                batch.1.range.end = vertices_index;
            }
            if should_batch {
                item.batch_range_mut().end += 1;
            }
            // render_phase.items[batch_item_index].batch_range_mut().end += 1;
        }
    }

    ui_meta.vertices.write_buffer(&render_device, &render_queue);
    ui_meta.indices.write_buffer(&render_device, &render_queue);
    *previous_len = batches.len();
    commands.try_insert_batch(batches);

    extracted_ui_nodes.clear();

    ui_stack_map.clear();
}
