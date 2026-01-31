use std::{
    any::TypeId,
    ops::{Deref, DerefMut, Range},
};

use bevy_camera::{Camera, visibility::VisibleEntities};
use bevy_ecs::{
    change_detection::DetectChangesMut,
    entity::{Entity, EntityHashMap},
    hierarchy::{ChildOf, Children},
    query::{With, Without},
    resource::Resource,
    system::{Local, Query, ResMut},
};
use bevy_render::extract_resource::ExtractResource;
use bevy_transform::components::GlobalTransform;
use fixedbitset::FixedBitSet;
use smallvec::SmallVec;

use crate::{
    computed::ComputedNode, elements::node::Node, utils::SORT_ENTITIES_FROM_BACK_TO_FRONT,
};

/// The current UI stack, which contains all UI nodes ordered by their depth (back-to-front).
///
/// The first entry is the furthest node from the camera and is the first one to get rendered
/// while the last entry is the first node to receive interactions.
#[derive(Debug, Clone, Default)]
pub struct UiStack {
    /// Stores the ranges of UI nodes ordered.
    pub ranges: SmallVec<[Range<usize>; 24]>,

    /// Lists the entities of UI nodes ordered.
    pub entities: SmallVec<[Entity; 32]>,

    /// Lists the roots of UI nodes ordered.
    pub roots: SmallVec<[Entity; 16]>,

    /// Stores the bitset.
    pub bitset: FixedBitSet,
}

#[derive(Debug, Clone, Resource, ExtractResource, Default)]
pub struct UiStackMap(EntityHashMap<UiStack>);

impl AsMut<EntityHashMap<UiStack>> for UiStackMap {
    fn as_mut(&mut self) -> &mut EntityHashMap<UiStack> {
        &mut self.0
    }
}

impl Deref for UiStackMap {
    type Target = EntityHashMap<UiStack>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for UiStackMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub fn ui_stack_system(
    render_targets: Query<(Entity, &VisibleEntities), With<Camera>>,
    root_node_query: Query<
        (Entity, &GlobalTransform, Option<&Children>),
        (With<Node>, Without<ChildOf>),
    >,
    node_query: Query<(Entity, &GlobalTransform, Option<&Children>), (With<Node>, With<ChildOf>)>,
    mut update_query: Query<&mut ComputedNode>,
    mut ui_stack_map: ResMut<UiStackMap>,
    mut view_entities: Local<FixedBitSet>,
) {
    ui_stack_map.clear();

    for (camera_entity, visiable_entities) in render_targets {
        view_entities.clear();
        view_entities.extend(
            visiable_entities
                .get(TypeId::of::<Node>())
                .iter()
                .map(|e| e.index_u32() as usize),
        );

        if view_entities.is_clear() {
            continue;
        }

        let ui_stack = ui_stack_map.as_mut().entry(camera_entity).or_default();

        ui_stack.bitset.union_with(&view_entities);

        // Only filter root nodes.
        let nodes = root_node_query
            .iter()
            .filter(|entity| ui_stack.bitset.contains(entity.0.index_u32() as usize))
            .collect::<Vec<_>>();

        ui_stack.roots.extend(nodes.iter().map(|e| e.0));

        // Make sure ui transparency phases' `sort_key` is correct.
        let mut depth = 0;

        update_ui_stack_recursive(
            &node_query,
            &mut update_query,
            ui_stack,
            &mut depth,
            nodes,
            camera_entity,
        );
    }
}

fn update_ui_stack_recursive(
    node_query: &Query<(Entity, &GlobalTransform, Option<&Children>), (With<Node>, With<ChildOf>)>,
    update_query: &mut Query<&mut ComputedNode>,
    ui_stack: &mut UiStack,
    depth: &mut usize,
    mut sorted_nodes: Vec<(Entity, &GlobalTransform, Option<&Children>)>,
    camera_entity: Entity,
) {
    if sorted_nodes.is_empty() {
        return;
    }

    radsort::sort_by_key(&mut sorted_nodes, |e| {
        SORT_ENTITIES_FROM_BACK_TO_FRONT(e.0, *e.1)
    });

    tracing::debug!(
        "camera: {} {} {:?}",
        camera_entity,
        &depth,
        sorted_nodes
            .iter()
            .map(|e| (e.0, e.1.translation().z))
            .collect::<Vec<_>>()
    );

    let start = ui_stack
        .ranges
        .last()
        .map(|range| range.end)
        .unwrap_or_default();
    let end = start + sorted_nodes.len();
    ui_stack.ranges.push(start..end);

    for (entity, _transform, children) in sorted_nodes {
        if let Ok(mut computed_node) = update_query.get_mut(entity) {
            computed_node.bypass_change_detection().stack_index = *depth;
        }

        ui_stack.entities.push(entity);

        *depth += 1;

        let Some(children) = children.filter(|c| !c.is_empty()) else {
            continue;
        };

        let nodes = node_query.iter_many(children).collect::<Vec<_>>();

        update_ui_stack_recursive(
            &node_query,
            update_query,
            ui_stack,
            depth,
            nodes,
            camera_entity,
        );
    }
}
