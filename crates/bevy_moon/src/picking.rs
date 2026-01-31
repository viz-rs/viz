use bevy_app::{App, Plugin, PreUpdate};
use bevy_camera::{Camera, Projection, RenderTarget, visibility::InheritedVisibility};
use bevy_ecs::{
    change_detection::Res, entity::Entity, message::MessageWriter, query::With,
    schedule::IntoScheduleConfigs, system::Query,
};
use bevy_math::{FloatExt, Vec2, Vec3, Vec3Swizzles};
use bevy_picking::{
    Pickable, PickingSystems,
    backend::{HitData, PointerHits, ray::RayMap},
    pointer::{PointerId, PointerLocation},
};
use bevy_transform::components::GlobalTransform;
use bevy_window::PrimaryWindow;

use crate::{computed::ComputedNode, elements::node::Node, stack::UiStackMap};

/// A plugin that adds picking support for UI nodes.
#[derive(Clone)]
pub struct UiPickingPlugin;

impl Plugin for UiPickingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreUpdate, ui_picking.in_set(PickingSystems::Backend));
    }
}

fn ui_picking(
    ray_map: Res<RayMap>,
    ui_stack_map: Res<UiStackMap>,
    pointers: Query<(&PointerId, &PointerLocation)>,
    primary_window: Query<Entity, With<PrimaryWindow>>,
    cameras: Query<(
        Entity,
        &Camera,
        &GlobalTransform,
        &Projection,
        &RenderTarget,
    )>,
    node_query: Query<
        (
            Entity,
            &GlobalTransform,
            &InheritedVisibility,
            Option<&Pickable>,
            &ComputedNode,
        ),
        With<Node>,
    >,
    mut output: MessageWriter<PointerHits>,
) {
    let primary_window = primary_window.single().ok();

    let pick_sets =
        ray_map.iter().filter_map(|(ray_id, ray)| {
            let (
                Some(ui_stack),
                Ok((camera_entity, camera, camera_transform, projection, render_target)),
            ) = (ui_stack_map.get(&ray_id.camera), cameras.get(ray_id.camera))
            else {
                return None;
            };

            let location = pointers.iter().find_map(|(id, loc)| {
                if *id == ray_id.pointer {
                    return loc.location.as_ref();
                }
                None
            })?;

            let viewport_pos = location.position;
            if let Some(viewport) = camera.logical_viewport_rect()
                && !viewport.contains(viewport_pos)
            {
                // The pointer is outside the viewport, skip it
                return None;
            }

            if render_target
                .normalize(primary_window)
                .is_none_or(|x| x != location.target)
            {
                return None;
            }

            let ray_options = {
                let (_scale, far, near) = match projection {
                    Projection::Perspective(p) => (1.0, p.far, p.near),
                    Projection::Orthographic(p) => (p.scale, p.far, p.near),
                    _ => return None,
                };

                let cursor_ray_len = far - near;
                let cursor_ray_end = ray.origin + ray.direction * cursor_ray_len;

                (near, ray.origin, cursor_ray_end)
            };

            let mut picks = Vec::<(Entity, HitData)>::new();
            let mut blocked = false;

            // From front to back
            for node in ui_stack.ranges.iter().rev().flat_map(|range| {
                node_query.iter_many(ui_stack.entities[range.clone()].iter().rev())
            }) {
                if blocked {
                    break;
                }

                let Some(picked) = pick(&ray_options, camera_entity, camera_transform, node) else {
                    continue;
                };

                blocked = picked.0;

                picks.push((node.0, picked.1));
            }

            if picks.is_empty() {
                return None;
            }

            // bevy ui: 0.5
            // bevy egui: 0.6
            Some((ray_id.pointer, picks, camera.order as f32 + 0.7))
        });

    pick_sets.for_each(|(pointer, picks, order)| {
        output.write(PointerHits::new(pointer, picks, order));
    });
}

fn pick(
    ray_options: &(f32, Vec3, Vec3),
    camera_entity: Entity,
    camera_transform: &GlobalTransform,
    node: (
        Entity,
        &GlobalTransform,
        &InheritedVisibility,
        Option<&Pickable>,
        &ComputedNode,
    ),
) -> Option<(bool, HitData)> {
    let (_entity, &transform, inherited_visibility, pickable, computed_node) = node;

    if !inherited_visibility.get() {
        return None;
    }

    if computed_node.is_empty() {
        return None;
    }

    let Some(local_point) = normalize_point((ray_options.1, ray_options.2), transform) else {
        return None;
    };

    let corner_radii = computed_node.corner_radii;

    if !contains_point(local_point, computed_node.size, corner_radii) {
        return None;
    }

    let world_point = transform.transform_point(local_point.extend(0.0));
    // Transform point from world to camera space to get the Z distance
    let camera_point = camera_transform
        .affine()
        .inverse()
        .transform_point3(world_point);

    // HitData requires a depth as calculated from the camera's near clipping plane
    let depth = -ray_options.0 - camera_point.z;

    let hitdata = HitData::new(
        camera_entity,
        depth,
        Some(world_point),
        Some(transform.back().as_vec3()),
    );

    let blocked = pickable
        // If an entity has a `Pickable` component, we will use that as the source of truth.
        .map(|pickable| pickable.should_block_lower)
        // If the `Pickable` component doesn't exist, default behavior is to block.
        .unwrap_or(true);

    Some((blocked, hitdata))
}

// Transform cursor line segment to node coordinate system.
fn normalize_point(
    (ray_origin, ray_end): (Vec3, Vec3),
    transform: GlobalTransform,
) -> Option<Vec2> {
    let world_to_node = transform.affine().inverse();
    let cursor_start_node = world_to_node.transform_point3(ray_origin);
    let cursor_end_node = world_to_node.transform_point3(ray_end);

    // Find where the cursor segment intersects the plane Z=0 (which is the node's
    // plane in node-local space). It may not intersect if, for example, we're
    // viewing the node side-on
    if cursor_start_node.z == cursor_end_node.z {
        // Cursor ray is parallel to the node and misses it
        return None;
    }
    let lerp_factor = f32::inverse_lerp(cursor_start_node.z, cursor_end_node.z, 0.0);
    // @TODO(fundon): check it in 3D
    // if !(0.0..=1.0).contains(&lerp_factor) {
    //     // Lerp factor is out of range, meaning that while an infinite line cast by
    //     // the cursor would intersect the node, the node is not between the
    //     // camera's near and far planes
    //     return None;
    // }
    // Otherwise we can interpolate the xy of the start and end positions by the
    // lerp factor to get the cursor position in node-local space!
    Some(cursor_start_node.lerp(cursor_end_node, lerp_factor).xy())
}

fn contains_point(
    local_point: Vec2,
    size: Vec2,
    [top_left, top_right, bottom_right, bottom_left]: [f32; 4],
) -> bool {
    let [top, bottom] = (local_point.x < 0.0)
        .then_some([top_left, bottom_left])
        .unwrap_or([top_right, bottom_right]);

    let r = (local_point.y < 0.0).then_some(top).unwrap_or(bottom);
    let corner_to_point = local_point.abs() - 0.5 * size;
    let q = corner_to_point + r;
    let l = q.max(Vec2::ZERO).length();
    let m = q.max_element().min(0.);
    l + m - r < 0.
}
