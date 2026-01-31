use std::ops::Range;

use bevy_asset::{AssetId, Assets};
use bevy_camera::visibility::InheritedVisibility;
use bevy_color::{Alpha, LinearRgba};
use bevy_ecs::{
    entity::Entity,
    query::With,
    resource::Resource,
    system::{Commands, Query, Res, ResMut},
};
use bevy_image::{Image, TextureAtlasLayout};
use bevy_math::{Affine3A, Rect, UVec2, Vec2, Vec4};
use bevy_render::{
    Extract,
    sync_world::{MainEntity, RenderEntity, TemporaryRenderEntity},
};
use bevy_text::{ComputedTextBlock, GlyphAtlasInfo, PositionedGlyph, TextColor, TextLayoutInfo};
use bevy_transform::components::GlobalTransform;

use crate::{
    computed::ComputedNode,
    elements::{
        image::{ImageNode, ImageNodeSize},
        node::Node,
        text::Text,
    },
    geometry::{VEC2_FLIP_X, VEC2_FLIP_Y},
    render::flags::{ShaderFlags, StackZOffsets},
    stack::UiStackMap,
    style::Style,
};

pub enum ExtractedUiItem {
    Node {
        size: Vec2,
        color: LinearRgba,
        atlas_scaling: Option<Vec2>,
        flip_x: bool,
        flip_y: bool,
        /// Corner radius of the UI node.
        /// Ordering: top left, top right, bottom right, bottom left.
        corner_radii: Vec4,
        /// Border thickness of the UI node.
        /// Ordering: left, top, right, bottom.
        border: Vec4,
        flags: ShaderFlags,
    },
    /// A contiguous sequence of text glyphs from the same section
    Glyphs {
        /// Indices into [`ExtractedUiNodes::glyphs`]
        range: Range<usize>,
    },
}

pub struct ExtractedGlyph {
    pub color: LinearRgba,
    pub translation: Vec2,
    pub rect: Rect,
}

pub struct ExtractedNode {
    pub z_order: f32,
    pub image: AssetId<Image>,
    pub clip: Option<Rect>,
    pub transform: Affine3A,
    pub item: ExtractedUiItem,
    pub main_entity: MainEntity,
    pub render_entity: Entity,
    pub camera_entity: Entity,
}

#[derive(Resource, Default)]
pub struct ExtractedUiNodes {
    pub nodes: Vec<ExtractedNode>,
    pub glyphs: Vec<ExtractedGlyph>,
}

impl ExtractedUiNodes {
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.glyphs.clear();
    }
}

pub fn extract_node_styles(
    mut extracted_ui_nodes: ResMut<ExtractedUiNodes>,
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
            extract_node_style(&mut extracted_ui_nodes, camera_entity, node);
        }
    }
}

fn extract_node_style(
    extracted_ui_nodes: &mut ExtractedUiNodes,
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
    if style.is_hidden() {
        return;
    }

    let index = computed_node.stack_index as f32;
    let main_entity = MainEntity::from(entity);
    let transform = transform.affine();
    let size = computed_node.size;
    let clip = style.clip_rect;

    // Style

    let corner_radii = computed_node.corner_radii;

    let mut border_width = [0.0f32; 4];

    // Background color
    if let Some(color) = style.background
        && !color.is_fully_transparent()
    {
        extracted_ui_nodes.nodes.push(ExtractedNode {
            z_order: index + StackZOffsets::BackgroundColor.to_percent(),
            image: AssetId::default(),
            clip,
            item: ExtractedUiItem::Node {
                color: color.into(),
                size,
                atlas_scaling: None,
                flip_x: false,
                flip_y: false,
                border: border_width.into(),
                corner_radii: corner_radii.into(),
                flags: ShaderFlags::UNTEXTURED,
            },
            transform,
            main_entity,
            render_entity,
            camera_entity,
        });
    }

    // Border
    let mut border_color = [LinearRgba::NONE; 4];
    if let Some(bc) = &style.border_color {
        for (i, (&w, c)) in computed_node
            .border
            .iter()
            .zip(bc.into_array().iter())
            .enumerate()
        {
            border_width[i] = w;
            border_color[i] = c.to_linear();
        }
    }

    if !(border_width.iter().all(|w| w == &0.0)
        || border_color.iter().all(LinearRgba::is_fully_transparent))
    {
        let color = border_color[0];
        let has_same_color = border_color[1..].iter().all(|c| *c == color);
        if has_same_color {
            extracted_ui_nodes.nodes.push(ExtractedNode {
                z_order: index + StackZOffsets::Border.to_percent(),
                image: AssetId::default(),
                clip,
                item: ExtractedUiItem::Node {
                    color,
                    size,
                    atlas_scaling: None,
                    flip_x: false,
                    flip_y: false,
                    border: border_width.into(),
                    corner_radii: corner_radii.into(),
                    flags: ShaderFlags::BORDER_ALL,
                },
                transform,
                main_entity,
                render_entity,
                camera_entity,
            });
        } else {
            for (color, flags) in border_color.iter().copied().zip(ShaderFlags::BORDERS) {
                extracted_ui_nodes.nodes.push(ExtractedNode {
                    z_order: index + StackZOffsets::Border.to_percent(),
                    image: AssetId::default(),
                    clip,
                    item: ExtractedUiItem::Node {
                        color,
                        size,
                        atlas_scaling: None,
                        flip_x: false,
                        flip_y: false,
                        border: border_width.into(),
                        corner_radii: corner_radii.into(),
                        flags,
                    },
                    transform,
                    main_entity,
                    render_entity,
                    camera_entity,
                });
            }
        }
    }

    // Outline
    if computed_node.outline[0] <= 0.0 {
        return;
    }

    let Some(color) = style
        .outline
        .map(|o| o.color)
        .filter(|c| !c.is_fully_transparent())
    else {
        return;
    };

    let color = color.into();
    let size = computed_node.outline_size();
    let border_width = computed_node.outline_border_width();
    let corner_radii = computed_node.outline_corner_radii();

    extracted_ui_nodes.nodes.push(ExtractedNode {
        z_order: index + StackZOffsets::Border.to_percent(),
        image: AssetId::default(),
        clip,
        item: ExtractedUiItem::Node {
            color,
            size,
            atlas_scaling: None,
            flip_x: false,
            flip_y: false,
            border: border_width.into(),
            corner_radii: corner_radii.into(),
            flags: ShaderFlags::BORDER_ALL,
        },
        transform,
        main_entity,
        render_entity,
        camera_entity,
    });
}

pub fn extract_images(
    mut extracted_nodes: ResMut<ExtractedUiNodes>,
    texture_atlases: Extract<Res<Assets<TextureAtlasLayout>>>,
    image_query: Extract<
        Query<
            (
                Entity,
                RenderEntity,
                &GlobalTransform,
                &InheritedVisibility,
                (&Style, &ImageNode, &ImageNodeSize, &ComputedNode),
            ),
            With<Node>,
        >,
    >,
    ui_stack_map: Extract<Res<UiStackMap>>,
) {
    for (&camera_entity, ui_stack) in ui_stack_map.iter() {
        for (
            entity,
            render_entity,
            transform,
            inherited_visibility,
            (style, image, image_size, computed_node),
        ) in ui_stack
            .ranges
            .iter()
            .flat_map(|range| image_query.iter_many(&ui_stack.entities[range.clone()]))
        {
            if !inherited_visibility.get() {
                continue;
            }
            if computed_node.is_empty() {
                continue;
            }
            if image.is_empty() {
                continue;
            }

            let atlas_rect = if image_size.size().cmpne(UVec2::ZERO).all() {
                Some(Rect::from_corners(Vec2::ZERO, image_size.size().as_vec2()))
            } else {
                image
                    .texture_atlas
                    .as_ref()
                    .and_then(|s| s.texture_rect(&texture_atlases))
                    .map(|r| r.as_rect())
            };

            let mut rect = match (atlas_rect, image.rect) {
                (None, None) => Rect {
                    min: Vec2::ZERO,
                    max: computed_node.size,
                },
                (None, Some(image_rect)) => image_rect,
                (Some(atlas_rect), None) => atlas_rect,
                (Some(atlas_rect), Some(mut image_rect)) => {
                    image_rect.min += atlas_rect.min;
                    image_rect.max += atlas_rect.min;
                    image_rect
                }
            };

            let atlas_scaling = if atlas_rect.is_some() || image.rect.is_some() {
                let atlas_scaling = computed_node.size / rect.size();
                rect.min *= atlas_scaling;
                rect.max *= atlas_scaling;
                Some(atlas_scaling)
            } else {
                None
            };

            let clip = style.clip_rect;
            let index = computed_node.stack_index as f32;
            let transform = transform.affine();
            let main_entity = MainEntity::from(entity);

            extracted_nodes.nodes.push(ExtractedNode {
                z_order: index + StackZOffsets::Image.to_percent(),
                image: image.image.id(),
                clip,
                item: ExtractedUiItem::Node {
                    color: image.color.into(),
                    size: rect.size(),
                    atlas_scaling,
                    flip_x: image.flip_x,
                    flip_y: image.flip_y,
                    border: computed_node.border.into(),
                    corner_radii: computed_node.corner_radii.into(),
                    flags: ShaderFlags::TEXTURED,
                },
                transform,
                main_entity,
                render_entity,
                camera_entity,
            });
        }
    }
}

pub fn extract_texts(
    mut commands: Commands,
    mut extracted_ui_nodes: ResMut<ExtractedUiNodes>,
    texture_atlases: Extract<Res<Assets<TextureAtlasLayout>>>,
    text_query: Extract<
        Query<
            (
                Entity,
                // RenderEntity,
                &GlobalTransform,
                &InheritedVisibility,
                &ComputedTextBlock,
                &Text,
                &TextColor,
                &TextLayoutInfo,
                (&Style, &ComputedNode),
            ),
            With<Node>,
        >,
    >,
    text_styles: Extract<Query<&TextColor>>,
    ui_stack_map: Extract<Res<UiStackMap>>,
) {
    let mut start = extracted_ui_nodes.glyphs.len();
    let mut end = start + 1;

    for (&camera_entity, ui_stack) in ui_stack_map.iter() {
        for (
            entity,
            // render_entity,
            transform,
            inherited_visibility,
            computed_block,
            _text,
            text_color,
            text_layout_info,
            (style, computed_node),
        ) in ui_stack
            .ranges
            .iter()
            .flat_map(|range| text_query.iter_many(&ui_stack.entities[range.clone()]))
        {
            if !inherited_visibility.get() {
                continue;
            }
            if computed_node.is_empty() {
                continue;
            }

            let clip = style.clip_rect;
            let index = computed_node.stack_index as f32;
            let main_entity = MainEntity::from(entity);

            let scale_factor = text_layout_info.scale_factor;
            let offset = 0.5 * computed_node.size * VEC2_FLIP_X;
            let scale_and_flip = scale_factor.recip() * VEC2_FLIP_Y;
            let transform = transform.affine()
                * Affine3A::from_translation(offset.extend(0.0))
                * Affine3A::from_scale(scale_and_flip.extend(0.0));

            let mut color = text_color.0.to_linear();

            let mut current_span = 0;

            for (
                i,
                &PositionedGlyph {
                    position,
                    span_index,
                    atlas_info:
                        GlyphAtlasInfo {
                            texture,
                            texture_atlas,
                            location,
                        },
                    ..
                },
            ) in text_layout_info.glyphs.iter().enumerate()
            {
                if span_index != current_span {
                    color = text_styles
                        .get(
                            computed_block
                                .entities()
                                .get(span_index)
                                .map(|t| t.entity)
                                .unwrap_or(Entity::PLACEHOLDER),
                        )
                        .map(|text_color| LinearRgba::from(text_color.0))
                        .unwrap_or_default();
                    current_span = span_index;
                }
                let rect = texture_atlases.get(texture_atlas).unwrap().textures
                    [location.glyph_index]
                    .as_rect();
                extracted_ui_nodes.glyphs.push(ExtractedGlyph {
                    color,
                    rect,
                    translation: position,
                });

                if text_layout_info.glyphs.get(i + 1).is_none_or(|info| {
                    info.span_index != current_span || info.atlas_info.texture != texture
                }) {
                    extracted_ui_nodes.nodes.push(ExtractedNode {
                        z_order: index + StackZOffsets::Text.to_percent(),
                        image: texture,
                        clip,
                        item: ExtractedUiItem::Glyphs { range: start..end },
                        transform,
                        main_entity,
                        // render_entity,
                        // Fixes missing some glyphs when the scale factor is too large, likes 8.
                        render_entity: commands.spawn(TemporaryRenderEntity).id(),
                        camera_entity,
                    });
                    start = end;
                }

                end += 1;
            }
        }
    }
}
