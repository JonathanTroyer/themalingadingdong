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

    // Background OKLCH changes
    BackgroundLChanged(f32),
    BackgroundCChanged(f32),
    BackgroundHChanged(f32),

    // Foreground OKLCH changes
    ForegroundLChanged(f32),
    ForegroundCChanged(f32),
    ForegroundHChanged(f32),

    // Numeric parameter changes
    TargetContrastChanged(f64),
    ExtendedContrastChanged(f64),
    AccentChromaChanged(f32),
    ExtendedChromaChanged(f32),

    // Selection changes
    VariantChanged(VariantArg),
    LightnessCurveTypeChanged(CurveType),
    LightnessCurveStrengthChanged(f32),
    ChromaCurveTypeChanged(CurveType),
    HueCurveTypeChanged(CurveType),

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
