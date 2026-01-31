use bevy_ecs::schedule::SystemSet;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum RenderUiSystems {
    ExtractCameraViews,
    ExtractBoxShadows,
    ExtractNodeStyles,
    ExtractImages,
    ExtractTexts,
    // ExtractBackgrounds,
    // ExtractTextureSlice,
    // ExtractBorders,
    // ExtractViewportNodes,
    // ExtractTextBackgrounds,
    // ExtractTextShadows,
    // ExtractText,
    // ExtractDebug,
    // ExtractGradient,
}
