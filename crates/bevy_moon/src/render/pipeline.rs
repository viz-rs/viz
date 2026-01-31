use bevy_asset::{AssetServer, Handle, load_embedded_asset};
use bevy_ecs::{change_detection::Res, resource::Resource, system::Commands};
use bevy_image::BevyDefault;
use bevy_mesh::VertexBufferLayout;
use bevy_render::{
    render_resource::{
        BindGroupLayoutDescriptor, BindGroupLayoutEntries, BlendState, ColorTargetState,
        ColorWrites, FragmentState, MultisampleState, RenderPipelineDescriptor, SamplerBindingType,
        ShaderStages, SpecializedRenderPipeline, TextureFormat, TextureSampleType, VertexFormat,
        VertexState, VertexStepMode,
        binding_types::{sampler, texture_2d, uniform_buffer},
    },
    view::{ViewTarget, ViewUniform},
};
use bevy_shader::Shader;
use bevy_sprite_render::Mesh2dPipelineKey;
use bevy_utils::default;

#[derive(Resource, Clone)]
pub struct UiPipeline {
    pub view_layout: BindGroupLayoutDescriptor,
    pub image_layout: BindGroupLayoutDescriptor,
    pub shader: Handle<Shader>,
    // @TODO(fundon): likes Mesh2dPipeline
    // pub per_object_buffer_batch_size: Option<u32>,
}

pub fn init_ui_pipeline(mut commands: Commands, asset_server: Res<AssetServer>) {
    let view_layout = BindGroupLayoutDescriptor::new(
        "moon_ui_view_layout",
        &BindGroupLayoutEntries::single(
            ShaderStages::VERTEX_FRAGMENT,
            uniform_buffer::<ViewUniform>(true),
        ),
    );

    let image_layout = BindGroupLayoutDescriptor::new(
        "moon_ui_image_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
            ),
        ),
    );

    commands.insert_resource(UiPipeline {
        view_layout,
        image_layout,
        shader: load_embedded_asset!(asset_server.as_ref(), "ui.wgsl"),
    });
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct UiPipelineKey {
    pub mesh_key: Mesh2dPipelineKey,
    pub anti_alias: bool,
}

impl SpecializedRenderPipeline for UiPipeline {
    type Key = UiPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let shader_defs = key
            .anti_alias
            .then_some(vec!["ANTI_ALIAS".into()])
            .unwrap_or_default();

        let mesh_key = key.mesh_key;

        let format = match mesh_key.contains(Mesh2dPipelineKey::HDR) {
            true => ViewTarget::TEXTURE_FORMAT_HDR,
            false => TextureFormat::bevy_default(),
        };
        let count = mesh_key.msaa_samples();

        let layout = vec![self.view_layout.clone(), self.image_layout.clone()];

        let vertex_layout = VertexBufferLayout::from_vertex_formats(
            VertexStepMode::Vertex,
            vec![
                // position
                VertexFormat::Float32x3,
                // uv
                VertexFormat::Float32x2,
                // color
                VertexFormat::Float32x4,
                // mode
                VertexFormat::Uint32,
                // border radius
                VertexFormat::Float32x4,
                // border thickness
                VertexFormat::Float32x4,
                // border size
                VertexFormat::Float32x2,
                // position relative to the center
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
            label: Some("moon_ui_pipeline".into()),
            ..default()
        }
    }
}
