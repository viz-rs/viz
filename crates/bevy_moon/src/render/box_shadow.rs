//! Box shadows rendering

use core::ops::Range;

use bevy_app::{App, Plugin};
use bevy_asset::{AssetServer, Handle, embedded_asset, load_embedded_asset};
use bevy_camera::visibility::InheritedVisibility;
use bevy_color::{Alpha, ColorToComponents, LinearRgba};
use bevy_ecs::{
    entity::Entity,
    prelude::Component,
    query::{ROQueryItem, With},
    reflect::ReflectComponent,
    resource::Resource,
    schedule::IntoScheduleConfigs,
    system::{
        Commands, Local, Query, Res, ResMut, SystemParamItem,
        lifetimeless::{Read, SRes},
    },
};
use bevy_image::BevyDefault;
use bevy_math::{Affine3A, FloatOrd, Vec2, Vec4};
use bevy_mesh::{VertexBufferLayout, VertexFormat};
use bevy_reflect::{Reflect, prelude::ReflectDefault};
use bevy_render::{
    Extract, ExtractSchedule, Render, RenderApp, RenderStartup, RenderSystems,
    render_phase::{
        AddRenderCommand, DrawFunctions, PhaseItem, PhaseItemExtraIndex, RenderCommand,
        RenderCommandResult, SetItemPipeline, TrackedRenderPass, ViewSortedRenderPhases,
    },
    render_resource::{
        BindGroup, BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries, BlendState,
        BufferUsages, ColorTargetState, ColorWrites, FragmentState, IndexFormat, MultisampleState,
        PipelineCache, RawBufferVec, RenderPipelineDescriptor, ShaderStages,
        SpecializedRenderPipeline, SpecializedRenderPipelines, TextureFormat, VertexState,
        VertexStepMode, binding_types::uniform_buffer,
    },
    renderer::{RenderDevice, RenderQueue},
    sync_world::{MainEntity, RenderEntity},
    view::{ExtractedView, ViewTarget, ViewUniform, ViewUniformOffset, ViewUniforms},
};
use bevy_shader::{Shader, ShaderDefVal};
use bevy_sprite_render::Mesh2dPipelineKey;
use bevy_transform::components::GlobalTransform;
use bevy_utils::default;

use bytemuck::{Pod, Zeroable};

use crate::{
    computed::ComputedNode,
    elements::node::Node,
    render::{
        flags::StackZOffsets,
        quad,
        systems::RenderUiSystems,
        transparent::TransparentUi,
        view::{MoonUiCameraView, MoonUiOptions, MoonUiViewTarget},
    },
    stack::UiStackMap,
    style::{BoxShadow, Style},
};

/// Number of shadow samples.
/// A larger value will result in higher quality shadows.
/// Default is 4, values higher than ~10 offer diminishing returns.
/// ```
#[derive(Component, Clone, Copy, Debug, Reflect, Eq, PartialEq)]
#[reflect(Component, Default, PartialEq, Clone)]
pub struct BoxShadowSamples(pub u32);

impl Default for BoxShadowSamples {
    fn default() -> Self {
        Self(4)
    }
}

/// A plugin that enables the rendering of box shadows.
pub struct BoxShadowPlugin;

impl Plugin for BoxShadowPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "box_shadow.wgsl");

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<BoxShadowMeta>()
            .init_resource::<ExtractedBoxShadows>()
            .allow_ambiguous_resource::<ExtractedBoxShadows>()
            .init_resource::<SpecializedRenderPipelines<BoxShadowPipeline>>()
            .add_render_command::<TransparentUi, DrawBoxShadows>()
            .add_systems(
                ExtractSchedule,
                extract_shadows.in_set(RenderUiSystems::ExtractBoxShadows),
            )
            .add_systems(RenderStartup, init_box_shadow_pipeline)
            .add_systems(
                Render,
                (
                    queue_shadows.in_set(RenderSystems::Queue),
                    prepare_shadows.in_set(RenderSystems::PrepareBindGroups),
                ),
            );
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable, Default)]
struct BoxShadowVertex {
    position: [f32; 3],
    uvs: [f32; 2],
    color: [f32; 4],
    size: [f32; 2],
    radius: [f32; 4],
    blur_radius: f32,
    bounds: [f32; 2],
}

#[derive(Component)]
pub struct UiShadowsBatch {
    pub range: Range<u32>,
}

/// Contains the vertices and bind groups to be sent to the GPU
#[derive(Resource)]
pub struct BoxShadowMeta {
    vertices: RawBufferVec<BoxShadowVertex>,
    indices: RawBufferVec<u32>,
    view_bind_group: Option<BindGroup>,
}

impl Default for BoxShadowMeta {
    fn default() -> Self {
        Self {
            vertices: RawBufferVec::new(BufferUsages::VERTEX),
            indices: RawBufferVec::new(BufferUsages::INDEX),
            view_bind_group: None,
        }
    }
}

#[derive(Resource)]
pub struct BoxShadowPipeline {
    pub view_layout: BindGroupLayoutDescriptor,
    pub shader: Handle<Shader>,
}

pub fn init_box_shadow_pipeline(mut commands: Commands, asset_server: Res<AssetServer>) {
    let view_layout = BindGroupLayoutDescriptor::new(
        "box_shadow_view_layout",
        &BindGroupLayoutEntries::single(
            ShaderStages::VERTEX_FRAGMENT,
            uniform_buffer::<ViewUniform>(true),
        ),
    );

    commands.insert_resource(BoxShadowPipeline {
        view_layout,
        shader: load_embedded_asset!(asset_server.as_ref(), "box_shadow.wgsl"),
    });
}

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub struct BoxShadowPipelineKey {
    pub mesh_key: Mesh2dPipelineKey,
    /// Number of samples, a higher value results in better quality shadows.
    pub samples: u32,
}

impl SpecializedRenderPipeline for BoxShadowPipeline {
    type Key = BoxShadowPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let shader_defs = vec![ShaderDefVal::UInt(
            "SHADOW_SAMPLES".to_string(),
            key.samples,
        )];

        let mesh_key = key.mesh_key;

        let format = match mesh_key.contains(Mesh2dPipelineKey::HDR) {
            true => ViewTarget::TEXTURE_FORMAT_HDR,
            false => TextureFormat::bevy_default(),
        };
        let count = mesh_key.msaa_samples();

        let layout = vec![self.view_layout.clone()];

        let vertex_layout = VertexBufferLayout::from_vertex_formats(
            VertexStepMode::Vertex,
            vec![
                // position
                VertexFormat::Float32x3,
                // uv
                VertexFormat::Float32x2,
                // color
                VertexFormat::Float32x4,
                // target rect size
                VertexFormat::Float32x2,
                // corner radius values (top left, top right, bottom right, bottom left)
                VertexFormat::Float32x4,
                // blur radius
                VertexFormat::Float32,
                // outer size
                VertexFormat::Float32x2,
            ],
        );

        RenderPipelineDescriptor {
            vertex: VertexState {
                shader: self.shader.clone(),
                shader_defs: shader_defs.clone(),
                buffers: vec![vertex_layout],
                ..default()
            },
            fragment: Some(FragmentState {
                shader: self.shader.clone(),
                shader_defs,
                targets: vec![Some(ColorTargetState {
                    format,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
                ..default()
            }),
            multisample: MultisampleState {
                count,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            layout,
            label: Some("box_shadow_pipeline".into()),
            ..default()
        }
    }
}

/// Description of a shadow to be sorted and queued for rendering
pub struct ExtractedBoxShadow {
    pub stack_index: f32,
    pub transform: Affine3A,
    pub bounds: Vec2,
    // pub clip: Option<Rect>,
    pub color: LinearRgba,
    pub radius: Vec4,
    pub blur_radius: f32,
    pub size: Vec2,
    pub main_entity: MainEntity,
    pub render_entity: Entity,
    pub camera_entity: Entity,
}

/// List of extracted shadows to be sorted and queued for rendering
#[derive(Resource, Default)]
pub struct ExtractedBoxShadows {
    pub box_shadows: Vec<ExtractedBoxShadow>,
}

pub fn extract_shadows(
    mut extracted_box_nodes: ResMut<ExtractedBoxShadows>,
    ui_stack_map: Extract<Res<UiStackMap>>,
    node_query: Extract<
        Query<
            (
                Entity,
                RenderEntity,
                &GlobalTransform,
                &InheritedVisibility,
                (&Style, &ComputedNode),
            ),
            With<Node>,
        >,
    >,
) {
    for (&camera_entity, ui_stack) in ui_stack_map.iter() {
        for node in ui_stack
            .ranges
            .iter()
            .flat_map(|range| node_query.iter_many(&ui_stack.entities[range.clone()]))
        {
            extract_node_style(&mut extracted_box_nodes, camera_entity, node);
        }
    }
}

fn extract_node_style(
    extracted_box_nodes: &mut ExtractedBoxShadows,
    camera_entity: Entity,
    (entity, render_entity, transform, inherited_visibility, (style, computed_node)): (
        Entity,
        Entity,
        &GlobalTransform,
        &InheritedVisibility,
        (&Style, &ComputedNode),
    ),
) {
    if !inherited_visibility.get() {
        return;
    }
    if computed_node.is_empty() {
        return;
    }
    let Some(box_shadow) = style.box_shadow else {
        return;
    };

    let BoxShadow {
        color,
        offset,
        blur_radius,
        spread_radius,
    } = box_shadow;

    if color.is_fully_transparent() {
        return;
    }

    let index = computed_node.stack_index as f32;
    let main_entity = MainEntity::from(entity);
    let transform = transform.affine();
    let size = computed_node.size;

    let spread_ratio = size.y / size.x;
    let spread = Vec2::new(spread_radius, spread_radius * spread_ratio);
    let bounds = size + spread;

    if bounds.cmple(Vec2::ZERO).any() {
        return;
    }

    let radius = computed_node.corner_radii.map(|n| n * spread_ratio);

    extracted_box_nodes.box_shadows.push(ExtractedBoxShadow {
        stack_index: index,
        color: color.into(),
        bounds: bounds + 6. * blur_radius,
        // clip: None,
        radius: radius.into(),
        blur_radius,
        size,
        transform: transform * Affine3A::from_translation(offset.extend(0.0)),
        main_entity,
        render_entity,
        camera_entity,
    });
}

pub fn queue_shadows(
    render_targets: Query<(
        MainEntity,
        &MoonUiCameraView,
        &MoonUiOptions,
        Option<&BoxShadowSamples>,
    )>,
    render_views: Query<&ExtractedView, With<MoonUiViewTarget>>,
    extracted_box_shadows: Res<ExtractedBoxShadows>,
    ui_stack_map: Res<UiStackMap>,
    box_shadow_pipeline: Res<BoxShadowPipeline>,
    pipeline_cache: Res<PipelineCache>,
    draw_functions: Res<DrawFunctions<TransparentUi>>,
    mut pipelines: ResMut<SpecializedRenderPipelines<BoxShadowPipeline>>,
    mut render_phases: ResMut<ViewSortedRenderPhases<TransparentUi>>,
) {
    let draw_function = draw_functions.read().id::<DrawBoxShadows>();

    for (extracted_index, node) in extracted_box_shadows.box_shadows.iter().enumerate() {
        let Some(ui_stack) = ui_stack_map.get(&node.camera_entity) else {
            continue;
        };
        let Some((
            _camera_entity,
            &MoonUiCameraView(ui_camera_view),
            &MoonUiOptions(mesh_key),
            shadow_samples,
        )) = render_targets.iter().find(|r| r.0 == node.camera_entity)
        else {
            continue;
        };
        let Some(extracted_view) = render_views.get(ui_camera_view).ok() else {
            continue;
        };
        let Some(render_phase) = render_phases.get_mut(&extracted_view.retained_view_entity) else {
            continue;
        };

        let samples = shadow_samples
            .map(|samples| samples.0)
            .unwrap_or(mesh_key.msaa_samples());

        let pipeline = pipelines.specialize(
            &pipeline_cache,
            &box_shadow_pipeline,
            BoxShadowPipelineKey { mesh_key, samples },
        );

        let view_index = node.main_entity.index_u32() as usize;
        if !ui_stack.bitset.contains(view_index) {
            continue;
        }

        let index = node.stack_index + StackZOffsets::BoxShadow.to_percent();

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

pub fn prepare_shadows(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    pipeline_cache: Res<PipelineCache>,
    view_uniforms: Res<ViewUniforms>,
    box_shadow_pipeline: Res<BoxShadowPipeline>,
    mut commands: Commands,
    mut ui_meta: ResMut<BoxShadowMeta>,
    mut extracted_shadows: ResMut<ExtractedBoxShadows>,
    mut phases: ResMut<ViewSortedRenderPhases<TransparentUi>>,
    mut previous_len: Local<usize>,
) {
    let Some(view_binding) = view_uniforms.uniforms.binding() else {
        return;
    };

    ui_meta.vertices.clear();
    ui_meta.indices.clear();
    ui_meta.view_bind_group = Some(render_device.create_bind_group(
        "box_shadow_view_bind_group",
        &pipeline_cache.get_bind_group_layout(&box_shadow_pipeline.view_layout),
        &BindGroupEntries::single(view_binding),
    ));

    let mut batches: Vec<(Entity, UiShadowsBatch)> = Vec::with_capacity(*previous_len);

    // Buffer indexes
    let mut vertices_index = 0;
    let mut indices_index = 0;

    for ui_phase in phases.values_mut() {
        for (item_index, item) in ui_phase.items.iter_mut().enumerate() {
            let Some(box_shadow) = extracted_shadows
                .box_shadows
                .get(item.extracted_index)
                .filter(|n| item.entity() == n.render_entity)
            else {
                continue;
            };

            let rect_size = box_shadow.bounds;

            let transform = box_shadow.transform;

            let points = quad::VERTEX_POSITIONS.map(|pos| pos * rect_size);

            // Specify the corners of the node
            let positions = points.map(|pos| transform.transform_point3(pos.extend(0.0)));

            let uvs = quad::UVS;

            let vertex = BoxShadowVertex {
                bounds: rect_size.into(),
                size: box_shadow.size.into(),
                radius: box_shadow.radius.into(),
                blur_radius: box_shadow.blur_radius,
                color: box_shadow.color.to_f32_array(),
                ..default()
            };

            for i in 0..4 {
                ui_meta.vertices.push(BoxShadowVertex {
                    uvs: uvs[i].into(),
                    position: positions[i].into(),
                    ..vertex
                });
            }

            for i in quad::INDICES {
                ui_meta.indices.push(indices_index + i);
            }

            batches.push((
                item.entity(),
                UiShadowsBatch {
                    range: vertices_index..vertices_index + 6,
                },
            ));

            vertices_index += 6;
            indices_index += 4;

            // shadows are sent to the gpu non-batched
            *item.batch_range_mut() = item_index as u32..item_index as u32 + 1;
        }
    }
    ui_meta.vertices.write_buffer(&render_device, &render_queue);
    ui_meta.indices.write_buffer(&render_device, &render_queue);
    *previous_len = batches.len();
    commands.try_insert_batch(batches);

    extracted_shadows.box_shadows.clear();
}

pub type DrawBoxShadows = (SetItemPipeline, SetBoxShadowViewBindGroup<0>, DrawBoxShadow);

pub struct SetBoxShadowViewBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetBoxShadowViewBindGroup<I> {
    type Param = SRes<BoxShadowMeta>;
    type ViewQuery = Read<ViewUniformOffset>;
    type ItemQuery = ();

    fn render<'w>(
        _item: &P,
        view_uniform: ROQueryItem<'w, '_, Self::ViewQuery>,
        _entity: Option<ROQueryItem<'w, '_, Self::ItemQuery>>,
        ui_meta: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(view_bind_group) = ui_meta.into_inner().view_bind_group.as_ref() else {
            return RenderCommandResult::Failure("view_bind_group not available");
        };

        pass.set_bind_group(I, view_bind_group, &[view_uniform.offset]);
        RenderCommandResult::Success
    }
}

pub struct DrawBoxShadow;
impl<P: PhaseItem> RenderCommand<P> for DrawBoxShadow {
    type Param = SRes<BoxShadowMeta>;
    type ViewQuery = ();
    type ItemQuery = Read<UiShadowsBatch>;

    #[inline]
    fn render<'w>(
        _item: &P,
        _view: ROQueryItem<'w, '_, Self::ViewQuery>,
        batch: Option<ROQueryItem<'w, '_, Self::ItemQuery>>,
        ui_meta: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(batch) = batch else {
            return RenderCommandResult::Skip;
        };
        let ui_meta = ui_meta.into_inner();
        let Some(vertices) = ui_meta.vertices.buffer() else {
            return RenderCommandResult::Failure("missing vertices to draw ui");
        };
        let Some(indices) = ui_meta.indices.buffer() else {
            return RenderCommandResult::Failure("missing indices to draw ui");
        };

        // Store the vertices
        pass.set_vertex_buffer(0, vertices.slice(..));
        // Define how to "connect" the vertices
        pass.set_index_buffer(indices.slice(..), IndexFormat::Uint32);
        // Draw the vertices
        pass.draw_indexed(batch.range.clone(), 0, 0..1);
        RenderCommandResult::Success
    }
}
