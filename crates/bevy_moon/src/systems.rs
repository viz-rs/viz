use bevy_ecs::{
    change_detection::DetectChangesMut,
    entity::Entity,
    hierarchy::Children,
    query::Has,
    system::{Query, Res},
};
use bevy_math::Rect;
use bevy_transform::components::GlobalTransform;

use crate::{
    computed::ComputedNode, elements::node::OverrideClip, stack::UiStackMap, style::Style,
};

pub fn ui_clipping_system(
    ui_stack_map: Res<UiStackMap>,
    node_query: Query<(
        Entity,
        &ComputedNode,
        &GlobalTransform,
        Option<&Children>,
        Has<OverrideClip>,
    )>,
    mut update_style: Query<&mut Style>,
) {
    // root nodes
    let entities = ui_stack_map
        .iter()
        .flat_map(|(_, ui_stack)| ui_stack.roots.as_ref());

    for node in node_query.iter_many(entities) {
        ui_clipping_recursive(&node_query, &mut update_style, node, None);
    }
}

fn ui_clipping_recursive(
    node_query: &Query<(
        Entity,
        &ComputedNode,
        &GlobalTransform,
        Option<&Children>,
        Has<OverrideClip>,
    )>,
    update_style: &mut Query<&mut Style>,
    (entity, computed_node, transform, children, has_override_clip): (
        Entity,
        &ComputedNode,
        &GlobalTransform,
        Option<&Children>,
        bool,
    ),
    mut maybe_inherited_clip_rect: Option<Rect>,
) {
    let Ok(mut style) = update_style.get_mut(entity) else {
        return;
    };

    // If the UI node entity has an `OverrideClip` component, discard any inherited clip rect
    if has_override_clip {
        maybe_inherited_clip_rect = None;
    }

    // If `display` is None, clip the entire node and all its descendants by replacing the inherited clip with a default rect (which is empty)
    if style.is_hidden() {
        maybe_inherited_clip_rect = Some(Rect::default());
    }

    match (maybe_inherited_clip_rect, style.clip_rect) {
        (Some(inherited_clip_rect), Some(clip_rect)) => {
            // Replace the previous calculated clip with the inherited clipping rect
            if clip_rect != inherited_clip_rect {
                style.bypass_change_detection().clip_rect = Some(inherited_clip_rect);
            }
        }
        (Some(inherited_clip_rect), None) => {
            // Update this node's clip rect
            style.bypass_change_detection().clip_rect = Some(inherited_clip_rect);
        }
        (None, clip_rect) => {
            if clip_rect.is_some() {
                style.bypass_change_detection().clip_rect = None;
            }
        }
    }

    if let Some(clip_rect) = computed_node.clip_rect(
        transform.translation().truncate(),
        style.overflow_is_visible(),
        style.overflow_clip_margin,
    ) {
        maybe_inherited_clip_rect = maybe_inherited_clip_rect
            .map(|c| c.intersect(clip_rect))
            .or(Some(clip_rect));
    }

    if let Some(children) = children {
        for node in node_query.iter_many(children) {
            ui_clipping_recursive(node_query, update_style, node, maybe_inherited_clip_rect);
        }
    }
}
