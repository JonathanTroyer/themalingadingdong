//! Event handling utilities for tui-realm integration.

use std::sync::LazyLock;

use crossterm_actions::{EditingMode, TuiRealmDispatcher};

/// Custom user events (currently unused, but required by tui-realm).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserEvent {}

/// Global dispatcher instance - shared by all components.
/// Using LazyLock for zero-cost lazy initialization.
pub static DISPATCHER: LazyLock<TuiRealmDispatcher> =
    LazyLock::new(|| TuiRealmDispatcher::with_defaults(EditingMode::Emacs));

/// Convenience function for components to access the dispatcher.
pub fn dispatcher() -> &'static TuiRealmDispatcher {
    &DISPATCHER
}
