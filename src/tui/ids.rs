//! Component identifiers for the TUI.

/// Unique identifiers for all components in the TUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Id {
    // Display panels (read-only)
    Palette,
    Preview,

    // Parameter groups (editable)
    BackgroundPicker,
    ForegroundPicker,
    TargetContrast,
    ExtendedContrast,
    AccentColorfulness,
    ExtendedColorfulness,
    LightnessCurve,
    LightnessStrength,
    ChromaCurve,
    ChromaStrength,
    HueCurve,
    HueStrength,
    HueOverrides,

    // Scrollable panel
    Validation,
}
