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
    AccentChroma,
    ExtendedChroma,
    LightnessCurve,
    LightnessStrength,
    ChromaCurve,
    HueCurve,
    HueOverrides,

    // Scrollable panel
    Validation,
}
