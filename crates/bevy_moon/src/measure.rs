use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
};

use bevy_ecs::{component::Component, reflect::ReflectComponent, system::Query};
use bevy_math::Vec2;
use bevy_reflect::{Reflect, prelude::ReflectDefault};
use bevy_text::{ComputedTextBlock, CosmicFontSystem};
use stacksafe::StackSafe;

pub struct MeasureArgs<'a> {
    pub known_dimensions: taffy::Size<Option<f32>>,
    pub available_space: taffy::Size<taffy::AvailableSpace>,
    pub font_system: &'a mut CosmicFontSystem,
    pub text_buffer: Option<&'a mut ComputedTextBlock>,
}

/// A `Measure` is used to compute the size of a ui node
/// when the size of that node is based on its content.
pub trait Measure: Send + Sync + 'static {
    /// Calculate the size of the node given the constraints.
    fn measure(&mut self, args: MeasureArgs<'_>, style: &taffy::Style) -> Vec2;

    /// Calculate the text buffer for the text node.
    fn get_text_buffer<'a>(
        &mut self,
        _: &'a mut Query<&mut ComputedTextBlock>,
    ) -> Option<&'a mut ComputedTextBlock> {
        None
    }
}

pub struct NodeContext(StackSafe<Box<dyn Measure>>);

impl Deref for NodeContext {
    type Target = StackSafe<Box<dyn Measure>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for NodeContext {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Debug for NodeContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeContxt").finish()
    }
}

impl NodeContext {
    pub fn new<M>(measure: M) -> Self
    where
        M: Measure + Send + Sync + 'static,
    {
        Self(StackSafe::new(Box::new(measure)))
    }
}

/// A `FixedMeasure` is a `Measure` that ignores all constraints and
/// always returns the same size.
#[derive(Default, Clone)]
pub struct FixedMeasure {
    pub size: Vec2,
}

impl Measure for FixedMeasure {
    fn measure(&mut self, _: MeasureArgs, _: &taffy::Style) -> Vec2 {
        self.size
    }
}

/// A node with a `ContentSize` component is a node where its size
/// is based on its content.
#[derive(Component, Reflect, Default)]
#[reflect(Component, Default)]
pub struct ContentSize {
    /// The `Measure` used to compute the intrinsic size
    #[reflect(ignore)]
    pub(crate) measure: Option<NodeContext>,
}

impl Deref for ContentSize {
    type Target = Option<NodeContext>;

    fn deref(&self) -> &Self::Target {
        &self.measure
    }
}

impl DerefMut for ContentSize {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.measure
    }
}

impl ContentSize {
    /// Set a `Measure` for the UI node entity with this component
    pub fn set<M>(&mut self, measure: M)
    where
        M: Measure + Send + Sync + 'static,
    {
        self.measure = Some(NodeContext::new(measure));
    }

    /// Creates a `ContentSize` with a `Measure` that always returns given `size` argument, regardless of the UI layout's constraints.
    pub fn fixed_size(size: Vec2) -> Self {
        Self {
            measure: Some(NodeContext::new(FixedMeasure { size })),
        }
    }

    /// Take the `Measure` from the `ContentSize` component.
    pub fn take(&mut self) -> Option<NodeContext> {
        self.measure.take()
    }
}
