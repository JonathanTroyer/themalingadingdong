//! Activity-based architecture for the TUI.
//!
//! Each screen in the TUI is an Activity with its own Application instance,
//! component IDs, and message types. The ActivityManager orchestrates transitions.

use std::io::Stdout;

use color_eyre::eyre::Result;
use ratatui::{Terminal, prelude::CrosstermBackend};

use super::Model;
use super::activities::{CodePreviewActivity, MainActivity};

/// Shared context passed between activities.
pub struct Context {
    pub model: Model,
}

/// Exit reasons for activity transitions.
#[derive(Debug, Clone, PartialEq)]
pub enum ExitReason {
    Quit,
    SwitchToMain,
    SwitchToCodePreview,
}

/// Activity lifecycle trait.
///
/// Each activity owns its own tui-realm Application and handles its own events.
pub trait Activity {
    /// Initialize the activity with context from the manager.
    fn on_create(&mut self, context: Context);

    /// Draw the UI and handle one tick of events.
    fn on_draw(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()>;

    /// Check if activity wants to exit. Returns Some(reason) to exit, None to continue.
    fn will_umount(&self) -> Option<&ExitReason>;

    /// Clean up and return the context to the manager.
    fn on_destroy(&mut self) -> Option<Context>;
}

/// Activity types available in the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityType {
    Main,
    CodePreview,
}

/// Manages activity lifecycle and transitions.
pub struct ActivityManager {
    context: Option<Context>,
    current: ActivityType,
}

impl ActivityManager {
    pub fn new(context: Context) -> Self {
        Self {
            context: Some(context),
            current: ActivityType::Main,
        }
    }

    pub fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
        loop {
            let mut activity: Box<dyn Activity> = match self.current {
                ActivityType::Main => Box::<MainActivity>::default(),
                ActivityType::CodePreview => Box::<CodePreviewActivity>::default(),
            };

            activity.on_create(self.context.take().expect("context should be available"));

            loop {
                activity.on_draw(terminal)?;

                if let Some(reason) = activity.will_umount() {
                    match reason {
                        ExitReason::Quit => {
                            activity.on_destroy();
                            return Ok(());
                        }
                        ExitReason::SwitchToMain => {
                            self.context = activity.on_destroy();
                            self.current = ActivityType::Main;
                            break;
                        }
                        ExitReason::SwitchToCodePreview => {
                            self.context = activity.on_destroy();
                            self.current = ActivityType::CodePreview;
                            break;
                        }
                    }
                }
            }
        }
    }
}
