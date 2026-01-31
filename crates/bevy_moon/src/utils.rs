use bevy_ecs::entity::Entity;
use bevy_transform::components::GlobalTransform;

/// Sort entities by their z-order.
///
/// Sorted by `translation.z` and entity id.
///
/// * Sorted from back To front: true.
/// * Sorted from front To back: false.
pub fn sort_entities_by_z_order<const BTF: bool>(
    entity: Entity,
    transform: GlobalTransform,
) -> (f32, u32) {
    sort_entities_by_z_order_with::<BTF>(transform.translation().z, entity.index_u32())
}

pub const SORT_ENTITIES_FROM_BACK_TO_FRONT: fn(Entity, GlobalTransform) -> (f32, u32) =
    sort_entities_by_z_order::<true>;

pub const SORT_ENTITIES_FROM_FRONT_TO_BACK: fn(Entity, GlobalTransform) -> (f32, u32) =
    sort_entities_by_z_order::<false>;

#[inline]
const fn sort_entities_by_z_order_with<const BTF: bool>(z: f32, index: u32) -> (f32, u32) {
    if BTF {
        (z, index)
    } else {
        (-z, u32::MAX - index)
    }
}
