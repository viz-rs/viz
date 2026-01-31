use std::ops::Range;

use bevy_asset::AssetId;
use bevy_ecs::{
    component::Component,
    query::ROQueryItem,
    resource::Resource,
    system::{
        SystemParamItem,
        lifetimeless::{Read, SRes},
    },
};
use bevy_image::Image;
use bevy_platform::collections::HashMap;
use bevy_render::{
    render_phase::{
        PhaseItem, RenderCommand, RenderCommandResult, SetItemPipeline, TrackedRenderPass,
    },
    render_resource::{BindGroup, BufferUsages, IndexFormat, RawBufferVec},
    view::ViewUniformOffset,
};
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable, Default)]
pub(crate) struct UiVertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
    pub color: [f32; 4],
    /// Shader flags to determine how to render the UI node.
    /// See [`shader_flags`] for possible values.
    pub flags: u32,
    /// Border radius of the UI node.
    /// Ordering: top left, top right, bottom right, bottom left.
    pub radius: [f32; 4],
    /// Border thickness of the UI node.
    /// Ordering: left, top, right, bottom.
    pub border: [f32; 4],
    /// Size of the UI node.
    pub size: [f32; 2],
    /// Position relative to the center of the UI node.
    pub point: [f32; 2],
}

#[derive(Resource)]
pub struct UiMeta {
    pub(crate) vertices: RawBufferVec<UiVertex>,
    pub(crate) indices: RawBufferVec<u32>,
    pub(crate) view_bind_group: Option<BindGroup>,
}

impl Default for UiMeta {
    fn default() -> Self {
        Self {
            vertices: RawBufferVec::new(BufferUsages::VERTEX),
            indices: RawBufferVec::new(BufferUsages::INDEX),
            view_bind_group: None,
        }
    }
}

#[derive(Resource, Default)]
pub struct ImageNodeBindGroups {
    pub values: HashMap<AssetId<Image>, BindGroup>,
}

#[derive(Component)]
pub struct UiBatch {
    pub range: Range<u32>,
    pub image: AssetId<Image>,
}

pub struct SetUiViewBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetUiViewBindGroup<I> {
    type Param = SRes<UiMeta>;
    type ViewQuery = Read<ViewUniformOffset>;
    type ItemQuery = ();

    #[inline]
    fn render<'w>(
        _item: &P,
        view_uniform: ROQueryItem<'w, '_, Self::ViewQuery>,
        _entity: Option<ROQueryItem<'w, '_, Self::ItemQuery>>,
        ui_meta: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let ui_meta = ui_meta.into_inner();
        let Some(view_bind_group) = ui_meta.view_bind_group.as_ref() else {
            return RenderCommandResult::Failure("view_bind_group not available");
        };

        pass.set_bind_group(I, view_bind_group, &[view_uniform.offset]);
        RenderCommandResult::Success
    }
}

pub struct SetUiTextureBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetUiTextureBindGroup<I> {
    type Param = SRes<ImageNodeBindGroups>;
    type ViewQuery = ();
    type ItemQuery = Read<UiBatch>;

    #[inline]
    fn render<'w>(
        _item: &P,
        _view: ROQueryItem<'w, '_, Self::ViewQuery>,
        batch: Option<ROQueryItem<'w, '_, Self::ItemQuery>>,
        image_bind_groups: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(batch) = batch else {
            return RenderCommandResult::Skip;
        };
        let image_bind_groups = image_bind_groups.into_inner();
        let Some(image) = image_bind_groups.values.get(&batch.image) else {
            return RenderCommandResult::Failure("missing texture to draw ui");
        };

        pass.set_bind_group(I, image, &[]);
        RenderCommandResult::Success
    }
}

pub struct DrawUiNode;
impl<P: PhaseItem> RenderCommand<P> for DrawUiNode {
    type Param = SRes<UiMeta>;
    type ViewQuery = ();
    type ItemQuery = Read<UiBatch>;

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

pub type DrawUi = (
    SetItemPipeline,
    SetUiViewBindGroup<0>,
    SetUiTextureBindGroup<1>,
    DrawUiNode,
);
