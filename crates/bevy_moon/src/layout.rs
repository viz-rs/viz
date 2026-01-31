use std::{
    fmt,
    ops::{Deref, DerefMut},
};

use bevy_camera::Camera;
use bevy_ecs::{
    change_detection::{DetectChanges, DetectChangesMut},
    entity::{Entity, EntityHashMap},
    error::Result,
    hierarchy::Children,
    lifecycle::RemovedComponents,
    query::{Added, Changed, With},
    resource::Resource,
    system::{Commands, Local, Query, Res, ResMut},
    world::Ref,
};
use bevy_math::{UVec2, Vec2};
use bevy_platform::collections::hash_map::Entry;
use bevy_text::{ComputedTextBlock, CosmicFontSystem};
use bevy_transform::components::Transform;
use smallvec::SmallVec;
use stacksafe::stacksafe;

use crate::{
    computed::{ComputedNode, ComputedTargetInfo},
    elements::{node::Node, text::TextMeasure},
    geometry::VEC2_FLIP_Y,
    measure::{ContentSize, Measure, MeasureArgs, NodeContext},
    stack::UiStackMap,
    style::Style,
};

#[derive(Debug)]
pub struct UiTree(taffy::TaffyTree<NodeContext>);

impl UiTree {
    pub fn new() -> Self {
        let mut tree = taffy::TaffyTree::new();
        tree.enable_rounding();
        Self(tree)
    }
}

impl Deref for UiTree {
    type Target = taffy::TaffyTree<NodeContext>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for UiTree {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Resource)]
pub struct UiLayoutEngine {
    tree: UiTree,
    // entity <-> taffy node
    layouts: EntityHashMap<taffy::NodeId>,
}

impl fmt::Debug for UiLayoutEngine {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("UiLayoutEngine")
            .field("tree", &self.tree)
            .field("layouts", &self.layouts)
            .finish()
    }
}

impl Default for UiLayoutEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[expect(unsafe_code, reason = "TaffyTree is safe as long as calc is not used")]
/// SAFETY: Taffy Tree becomes thread unsafe when you use the calc feature, which we do not implement
unsafe impl Send for UiTree {}

#[expect(unsafe_code, reason = "TaffyTree is safe as long as calc is not used")]
/// SAFETY: Taffy Tree becomes thread unsafe when you use the calc feature, which we do not implement
unsafe impl Sync for UiTree {}

fn _assert_send_sync_ui_surface_impl_safe() {
    fn _assert_send_sync<T: Send + Sync>() {}
    _assert_send_sync::<EntityHashMap<taffy::NodeId>>();
    _assert_send_sync::<UiTree>();
    _assert_send_sync::<UiLayoutEngine>();
}

const EXPECT_MESSAGE: &str = "we should avoid taffy layout errors by construction if possible";

impl UiLayoutEngine {
    pub fn new() -> Self {
        let mut tree = UiTree::new();
        tree.enable_rounding();
        UiLayoutEngine {
            tree,
            layouts: EntityHashMap::default(),
        }
    }

    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.tree.clear();
        self.layouts.clear();
    }

    pub fn upsert_node(
        &mut self,
        entity: Entity,
        style: &Style,
        scale_factor: f32,
        node_context: Option<NodeContext>,
    ) -> taffy::NodeId {
        let taffy_style = style.into_taffy(scale_factor);

        let node_id = match self.layouts.entry(entity) {
            Entry::Occupied(entry) => {
                let node_id = *entry.get();
                self.update_node_style(node_id, taffy_style);
                self.update_node_context(node_id, node_context);
                node_id
            }
            Entry::Vacant(entry) => {
                let node_id = Self::_request_layout(&mut self.tree, taffy_style, node_context);
                entry.insert(node_id);
                node_id
            }
        };

        node_id
    }

    pub fn remove_nodes(&mut self, entities: impl Iterator<Item = Entity>) {
        for entity in entities {
            self.remove_node(entity);
        }
    }

    pub fn remove_node(&mut self, entity: Entity) {
        let Some(node_id) = self.layouts.remove(&entity) else {
            return;
        };
        self.tree.remove(node_id).expect(EXPECT_MESSAGE);
    }

    pub fn remove_node_children(&mut self, entity: Entity) {
        let Some(&node_id) = self.layouts.get(&entity) else {
            return;
        };
        self.update_node_children(node_id, &[]);
    }

    #[allow(dead_code)]
    pub fn remove_node_context(&mut self, entity: Entity) {
        let Some(&node_id) = self.layouts.get(&entity) else {
            return;
        };
        self.tree
            .set_node_context(node_id, None)
            .expect(EXPECT_MESSAGE);
    }

    pub fn update_node_style(&mut self, node_id: taffy::NodeId, taffy_style: taffy::Style) {
        self.tree
            .set_style(node_id, taffy_style)
            .expect(EXPECT_MESSAGE)
    }

    pub fn update_node_context(
        &mut self,
        node_id: taffy::NodeId,
        node_context: Option<NodeContext>,
    ) {
        self.tree
            .set_node_context(node_id, node_context)
            .expect(EXPECT_MESSAGE)
    }

    pub fn update_node_children(&mut self, node_id: taffy::NodeId, children: &[taffy::NodeId]) {
        self.tree
            .set_children(node_id, children)
            .expect(EXPECT_MESSAGE);
    }

    fn _request_layout(
        taffy: &mut taffy::TaffyTree<NodeContext>,
        taffy_style: taffy::Style,
        node_context: Option<NodeContext>,
    ) -> taffy::NodeId {
        if let Some(context) = node_context {
            taffy.new_leaf_with_context(taffy_style, context)
        } else {
            taffy.new_leaf(taffy_style)
        }
        .expect(EXPECT_MESSAGE)
    }

    #[allow(dead_code)]
    pub fn request_layout(
        &mut self,
        taffy_style: taffy::Style,
        node_context: Option<NodeContext>,
    ) -> taffy::NodeId {
        Self::_request_layout(&mut self.tree, taffy_style, node_context)
    }

    #[allow(dead_code)]
    pub fn request_layout_with_children(
        &mut self,
        taffy_style: taffy::Style,
        children: &[taffy::NodeId],
    ) -> taffy::NodeId {
        self.tree
            .new_with_children(taffy_style, children)
            .expect(EXPECT_MESSAGE)
    }

    #[allow(dead_code)]
    pub fn request_measured_layout(
        &mut self,
        taffy_style: taffy::Style,
        measure: impl Measure,
    ) -> taffy::NodeId {
        self.tree
            .new_leaf_with_context(taffy_style, NodeContext::new(measure))
            .expect(EXPECT_MESSAGE)
    }

    #[stacksafe]
    pub fn compute_layout<'a>(
        &mut self,
        root_node_entity: Entity,
        scale_factor: f32,
        physical_size: UVec2,
        text_block_query: &'a mut Query<&mut ComputedTextBlock>,
        font_system: &'a mut CosmicFontSystem,
    ) {
        let node_id = *self.layouts.entry(root_node_entity).or_insert_with(|| {
            let node_id = self
                .tree
                .new_leaf(taffy::Style::DEFAULT)
                .expect(EXPECT_MESSAGE);
            node_id
        });

        let physical_size = physical_size.as_vec2() * scale_factor;

        let available_space = taffy::Size {
            width: taffy::AvailableSpace::Definite(physical_size.x),
            height: taffy::AvailableSpace::Definite(physical_size.y),
        };

        self.tree
            .compute_layout_with_measure(
                node_id,
                available_space,
                |known_dimensions, available_space, _id, node_context, style| {
                    let Some(node_context) = node_context else {
                        return taffy::Size::ZERO;
                    };

                    let text_buffer =
                        TextMeasure::needs_buffer(known_dimensions.height, available_space.width)
                            .then(|| node_context.get_text_buffer(text_block_query))
                            .flatten();

                    let args = MeasureArgs {
                        known_dimensions,
                        available_space,
                        font_system,
                        text_buffer,
                    };

                    let Vec2 { x, y } = node_context.measure(args, style);
                    taffy::Size {
                        width: x,
                        height: y,
                    }
                },
            )
            .expect(EXPECT_MESSAGE);
    }

    pub fn get_layout(&self, entity: Entity, scale_factor: f32) -> Result<taffy::Layout> {
        use crate::geometry::Scale;

        let Some(&node_id) = self.layouts.get(&entity) else {
            return Err("Invalid hierarchy".into());
        };

        let layout = self.tree.layout(node_id)?;

        let inverse_scale_factor = scale_factor.recip();

        Ok(layout.scale(inverse_scale_factor))
    }
}

// Used to understand performance
#[allow(dead_code)]
impl UiLayoutEngine {
    fn count_all_children(&self, parent: taffy::NodeId) -> Result<u32> {
        let mut count = 0;

        for child in self.tree.children(parent)? {
            // Count this child.
            count += 1;

            // Count all of this child's children.
            count += self.count_all_children(child)?
        }

        Ok(count)
    }

    fn max_depth(&self, depth: u32, parent: taffy::NodeId) -> Result<u32> {
        use taffy::TraversePartialTree;

        println!(
            "{parent:?} at depth {depth} has {} children",
            self.tree.child_count(parent)
        );

        let mut max_child_depth = 0;

        for child in self.tree.children(parent)? {
            max_child_depth = std::cmp::max(max_child_depth, self.max_depth(0, child)?);
        }

        Ok(depth + 1 + max_child_depth)
    }

    fn get_edges(&self, parent: taffy::NodeId) -> Result<Vec<(taffy::NodeId, taffy::NodeId)>> {
        let mut edges = Vec::new();

        for child in self.tree.children(parent)? {
            edges.push((parent, child));

            edges.extend(self.get_edges(child)?);
        }

        Ok(edges)
    }

    fn print_tree(&mut self) {
        for (entity, &node_id) in &self.layouts {
            println!("Entity: {entity}");
            self.tree.print_tree(node_id);
            // self.get_edges(node_id);
        }
    }
}

pub fn ui_target_info_system(
    mut commands: Commands,
    #[cfg(feature = "pan")] mut camera_query: Query<(
        &Camera,
        &bevy_camera_controller::pan_camera::PanCamera,
        Option<&mut ComputedTargetInfo>,
    )>,
    #[cfg(not(feature = "pan"))] mut camera_query: Query<(
        &Camera,
        &bevy_camera::Projection,
        Option<&mut ComputedTargetInfo>,
    )>,
    ui_stack_map: Res<UiStackMap>,
) {
    for &camera_entity in ui_stack_map.keys() {
        let Ok((camera, p, target_info)) = camera_query.get_mut(camera_entity) else {
            continue;
        };

        let scale_factor = camera.target_scaling_factor().unwrap_or(1.0);
        let physical_size = camera.physical_viewport_size().unwrap_or(UVec2::ZERO);

        #[cfg(feature = "pan")]
        let zoom_factor = p.zoom_factor;

        #[cfg(not(feature = "pan"))]
        let zoom_factor = match p {
            bevy_camera::Projection::Orthographic(projection) => projection.scale,
            _ => 1.0,
        };

        if let Some(mut target_info) = target_info {
            if target_info.zoom_factor != zoom_factor {
                target_info.zoom_factor = zoom_factor;
            }
        } else {
            commands.entity(camera_entity).insert(ComputedTargetInfo {
                scale_factor,
                physical_size,
                zoom_factor,
            });
        }
    }
}

pub fn ui_layout_system(
    camera_query: Query<&ComputedTargetInfo, With<Camera>>,
    node_query: Query<(Entity, Ref<Style>, Option<&Children>), With<Node>>,
    ui_stack_map: Res<UiStackMap>,
    mut ui_layout_engine: ResMut<UiLayoutEngine>,
    mut layouts: Local<SmallVec<[taffy::NodeId; 8]>>,
    mut removed_children: RemovedComponents<Children>,
    mut removed_nodes: RemovedComponents<Node>,

    _added_node_query: Query<(), Added<Node>>,
    _changed_children_query: Query<Entity, (Changed<Children>, With<Node>)>,

    mut update_node_query: Query<(&mut Transform, &mut ComputedNode), With<Node>>,
    mut content_size_query: Query<Option<&mut ContentSize>>,
    mut text_block_query: Query<&mut ComputedTextBlock>,
    mut font_system: ResMut<CosmicFontSystem>,
) {
    // Adds all nodes to the layout engine.
    for (&camera_entity, ui_stack) in ui_stack_map.iter() {
        let Ok(targt_info) = camera_query.get(camera_entity) else {
            continue;
        };

        let &ComputedTargetInfo { scale_factor, .. } = targt_info;

        for node in node_query.iter_many(&ui_stack.roots) {
            update_ui_layout_recursive(
                &node_query,
                &mut content_size_query,
                &mut ui_layout_engine,
                &mut layouts,
                node,
                scale_factor,
            );
        }

        layouts.clear();
    }

    // Updates and remove children.
    for entity in removed_children.read() {
        ui_layout_engine.remove_node_children(entity);
    }

    // Cleans up removed nodes after syncing children.
    ui_layout_engine.remove_nodes(
        removed_nodes
            .read()
            .filter(|&entity| !node_query.contains(entity)),
    );

    // Computes ui layout by UI root entity.
    for (&camera_entity, ui_stack) in ui_stack_map.iter() {
        let Ok(targt_info) = camera_query.get(camera_entity) else {
            continue;
        };

        let &ComputedTargetInfo {
            scale_factor,
            physical_size,
            ..
        } = targt_info;

        for node in node_query.iter_many(&ui_stack.roots) {
            ui_layout_engine.compute_layout(
                node.0,
                scale_factor,
                physical_size,
                &mut text_block_query,
                &mut font_system,
            );

            update_ui_geometry_recursive(
                &node_query,
                &mut update_node_query,
                &mut text_block_query,
                &mut font_system,
                &mut ui_layout_engine,
                None,
                node,
                scale_factor,
            );
        }
    }

    // Debugs
    // #[cfg(debug_assertions)]
    // ui_layout_engine.print_tree();
}

fn update_ui_layout_recursive(
    node_query: &Query<(Entity, Ref<Style>, Option<&Children>), With<Node>>,
    content_size_query: &mut Query<Option<&mut ContentSize>>,
    ui_layout_engine: &mut UiLayoutEngine,
    layouts: &mut SmallVec<[taffy::NodeId; 8]>,
    (entity, style, children): (Entity, Ref<Style>, Option<&Children>),
    scale_factor: f32,
) {
    // Stores current node's layout id and index.
    let mut node = Option::<(taffy::NodeId, usize)>::None;

    let content_size = content_size_query.get_mut(entity).ok().flatten();

    let is_changed = style.is_changed()
        || content_size
            .as_ref()
            .is_some_and(|c| c.is_changed() && c.measure.is_some())
        || !ui_layout_engine.layouts.contains_key(&entity);

    if is_changed {
        let node_id = ui_layout_engine.upsert_node(
            entity,
            &style,
            scale_factor,
            content_size.and_then(|mut c| {
                let content_size = c.bypass_change_detection().take();
                content_size
            }),
        );

        layouts.push(node_id);

        node = Some((node_id, layouts.len()));
    }

    if let Some(children) = children {
        for node in node_query.iter_many(children) {
            update_ui_layout_recursive(
                node_query,
                content_size_query,
                ui_layout_engine,
                layouts,
                node,
                scale_factor,
            );
        }
    }

    // The current node is created, so we can update its children.
    let Some((node_id, node_children)) = node
        .map(|(node_id, index)| (node_id, layouts.drain(index..)))
        .filter(|(_, node_children)| node_children.len() > 0)
    else {
        return;
    };

    ui_layout_engine.update_node_children(node_id, &node_children.collect::<Vec<_>>());
}

fn update_ui_geometry_recursive(
    node_query: &Query<(Entity, Ref<Style>, Option<&Children>), With<Node>>,
    update_node_query: &mut Query<(&mut Transform, &mut ComputedNode), With<Node>>,
    text_block_query: &mut Query<&mut ComputedTextBlock>,
    font_system: &mut CosmicFontSystem,
    ui_layout_engine: &mut UiLayoutEngine,
    mut maybe_inherited_node: Option<(Transform, ComputedNode)>,
    (entity, style, children): (Entity, Ref<Style>, Option<&Children>),
    scale_factor: f32,
) {
    let (Ok(layout), Ok((mut transform, mut computed_node))) = (
        ui_layout_engine.get_layout(entity, scale_factor),
        update_node_query.get_mut(entity),
    ) else {
        return;
    };

    {
        let bypass_computed_node = computed_node.bypass_change_detection();
        let prev_location = bypass_computed_node.location;
        let prev_size = bypass_computed_node.size;

        bypass_computed_node.set_layout(layout);
        bypass_computed_node.set_corner_radii(style.corner_radii);

        if let Some(outline) = style.outline {
            bypass_computed_node.set_outline(outline);
        }

        if prev_location != computed_node.location || prev_size != computed_node.size {
            computed_node.set_changed();
        }
    }

    if let Some((_parent_transform, parent_computed_node)) = maybe_inherited_node {
        // @TODO(fundon): scrolling
        let local_center =
            computed_node.location + 0.5 * (computed_node.size - parent_computed_node.size);
        let local_center_flipped = local_center * VEC2_FLIP_Y;

        let mut local_affine = computed_node.affine;
        if local_center_flipped != local_affine.translation.truncate() {
            // extracts transform without layout translation
            let base_affine = transform.compute_affine() * local_affine.inverse();

            // updates layout translation
            local_affine.translation.x = local_center_flipped.x;
            local_affine.translation.y = local_center_flipped.y;

            // applies new layout translation
            let new_affine = base_affine * local_affine;

            transform.translation.x = new_affine.translation.x;
            transform.translation.y = new_affine.translation.y;

            computed_node.affine = local_affine;
        }
    }

    if let Some(children) = children {
        // Updates its children.
        maybe_inherited_node = Some((*transform, *computed_node));

        for node in node_query.iter_many(children) {
            update_ui_geometry_recursive(
                &node_query,
                update_node_query,
                text_block_query,
                font_system,
                ui_layout_engine,
                maybe_inherited_node,
                node,
                scale_factor,
            );
        }
    }
}
