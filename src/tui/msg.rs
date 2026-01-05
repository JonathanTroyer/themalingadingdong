//! Application messages for the TUI.

use crate::cli::VariantArg;
use crate::curves::CurveType;

/// All possible messages that can be sent in the TUI application.
#[derive(Debug, Clone, PartialEq)]
pub enum Msg {
    // Application control
    Quit,
    ShowHelp,
    HideHelp,
    ShowExport,
    HideExport,

    // Focus/Navigation
    FocusNext,
    FocusPrev,
    FocusUp,
    FocusDown,

    // Background HellwigJmh changes (J', M, h)
    BackgroundJChanged(f32),
    BackgroundMChanged(f32),
    BackgroundHChanged(f32),

    // Foreground HellwigJmh changes (J', M, h)
    ForegroundJChanged(f32),
    ForegroundMChanged(f32),
    ForegroundHChanged(f32),

    // Numeric parameter changes
    MinContrastChanged(f64),
    ExtendedMinContrastChanged(f64),
    AccentColorfulnessChanged(f32),
    ExtendedColorfulnessChanged(f32),

    // Selection changes
    VariantChanged(VariantArg),
    LightnessCurveTypeChanged(CurveType),
    LightnessCurveStrengthChanged(f32),
    ChromaCurveTypeChanged(CurveType),
    ChromaCurveStrengthChanged(f32),
    HueCurveTypeChanged(CurveType),
    HueCurveStrengthChanged(f32),

    // Hue override changes (index 0-7)
    HueOverrideChanged(u8, Option<f32>),

    // Metadata changes
    NameChanged(String),
    AuthorChanged(String),

    // Palette regeneration (chained after parameter changes)
    Regenerate,

    // Export flow
    ExportPathChanged(String),
    DoExport,
    ExportSuccess(String),
    ExportError(String),

    // Validation scroll
    ValidationScrollUp,
    ValidationScrollDown,

    // No-op (for unhandled events)
    None,
}
