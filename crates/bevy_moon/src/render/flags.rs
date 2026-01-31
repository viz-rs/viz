bitflags::bitflags! {
    #[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    /// The values here should match the values for the constants in `ui.wgsl`
    pub struct ShaderFlags: u32 {
        const UNTEXTURED                        = 0;
        const TEXTURED                          = 1 << 0;
        const CORNER_TOP_RIGHT                  = 1 << 1;
        const CORNER_TOP_LEFT                   = 1 << 2;
        const CORNER_BOTTOM_LEFT                = 1 << 3;
        const CORNER_BOTTOM_RIGHT               = 1 << 4;
        const RADIAL                            = 1 << 5;
        const FILL_START                        = 1 << 6;
        const FILL_END                          = 1 << 7;
        const CONIC                             = 1 << 8;
        const BORDER_LEFT                       = 1 << 9;
        const BORDER_TOP                        = 1 << 10;
        const BORDER_RIGHT                      = 1 << 11;
        const BORDER_BOTTOM                     = 1 << 12;
        const BORDER_ALL                        = Self::BORDER_LEFT.bits()
          | Self::BORDER_TOP.bits()
          | Self::BORDER_RIGHT.bits()
          | Self::BORDER_BOTTOM.bits();
        const CORNER_ALL                        = Self::CORNER_TOP_LEFT.bits()
          | Self::CORNER_TOP_RIGHT.bits()
          | Self::CORNER_BOTTOM_RIGHT.bits()
          | Self::CORNER_BOTTOM_LEFT.bits();
    }
}

impl ShaderFlags {
    pub const CORNERS: [Self; 4] = [
        Self::CORNER_TOP_RIGHT,
        Self::CORNER_TOP_LEFT,
        Self::CORNER_BOTTOM_LEFT,
        Self::CORNER_BOTTOM_RIGHT,
    ];

    pub const BORDERS: [Self; 4] = [
        Self::BORDER_LEFT,
        Self::BORDER_TOP,
        Self::BORDER_RIGHT,
        Self::BORDER_BOTTOM,
    ];
}

/// Local Z offsets of "extracted nodes" for a given entity. These exist to allow rendering multiple "extracted nodes"
/// for a given source entity (ex: render both a background color _and_ a custom material for a given node).
///
/// When possible these offsets should be defined in _this_ module to ensure z-index coordination across contexts.
/// When this is _not_ possible, pick a suitably unique index unlikely to clash with other things (ex: `0.1826823` not `0.1`).
///
/// Offsets should be unique for a given node entity to avoid z fighting.
/// These should pretty much _always_ be larger than -0.5 and smaller than 0.5 to avoid clipping into nodes
/// above / below the current node in the stack.
///
/// A z-index of 0.0 is the baseline, which is used as the primary "background color" of the node.
///
/// Note that nodes "stack" on each other, so a negative offset on the node above could clip _into_
/// a positive offset on a node below.
#[derive(Copy, Clone)]
#[repr(i8)]
pub enum StackZOffsets {
    BoxShadow = -10,
    BackgroundColor = 0,
    Border = 1,
    // Gradient = 2,
    // BorderGradient = 3,
    Image = 4,
    // Material = 5,
    Text = 6,
    // TextStrikeThrough = 7,
}

impl StackZOffsets {
    pub fn to_percent(&self) -> f32 {
        ((*self as i8) as f32) / 100.0
    }
}
