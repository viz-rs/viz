use bevy_ecs::{component::Component, prelude::ReflectComponent};
use bevy_reflect::{Reflect, prelude::ReflectDefault};
use bevy_transform::components::Transform;

use crate::computed::ComputedNode;
use crate::style::Style;

/// A [`Node`] element, the all-in-one element for building complex UIs in bevy.
#[derive(Component, Clone, Debug, Default, PartialEq, Reflect)]
#[require(Transform, Style, ComputedNode)]
#[reflect(Component, Clone, Debug, Default, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct Node;

/// UI node entities with this component will ignore any clipping rect they inherit,
/// the node will not be clipped regardless of its ancestors' `Overflow` setting.
#[derive(Component)]
pub struct OverrideClip;
