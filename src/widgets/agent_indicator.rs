use crate::views::instances::AgentState;
use ratatui::style::Color;
use std::time::{SystemTime, UNIX_EPOCH};

const BLINK_DURATION_MS: u128 = 500;
const BLINK_PHASES: u128 = 2;

pub struct AgentIndicator {
    state: AgentState,
}

impl AgentIndicator {
    #[must_use]
    pub const fn new(state: AgentState) -> Self {
        Self { state }
    }

    fn should_blink() -> bool {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(true, |duration| {
                (duration.as_millis() / BLINK_DURATION_MS).is_multiple_of(BLINK_PHASES)
            })
    }

    #[must_use]
    pub fn label(&self) -> (&'static str, Color) {
        let should_blink = Self::should_blink();

        match self.state {
            AgentState::Idle => ("Idle", Color::Gray),
            AgentState::Running if should_blink => ("Running", Color::Rgb(255, 165, 0)),
            AgentState::Running => ("Running", Color::Rgb(255, 165, 0)),
            AgentState::NeedsPermissions => ("Needs Permission", Color::Rgb(138, 43, 226)),
            AgentState::Done => ("Done", Color::Green),
        }
    }

    #[must_use]
    pub fn dot(&self) -> (&'static str, Color) {
        let should_blink = Self::should_blink();

        match self.state {
            AgentState::Idle => (" ", Color::Gray),
            AgentState::Running if should_blink => ("●", Color::Rgb(255, 165, 0)),
            AgentState::Running => (" ", Color::Rgb(255, 165, 0)),
            AgentState::NeedsPermissions => ("●", Color::Rgb(138, 43, 226)),
            AgentState::Done => ("●", Color::Green),
        }
    }

    #[must_use]
    pub fn dot_visible(&self) -> (&'static str, Color) {
        let should_blink = Self::should_blink();

        match self.state {
            AgentState::Idle => ("", Color::Gray),
            AgentState::Running if should_blink => ("●", Color::Rgb(255, 165, 0)),
            AgentState::Running => ("", Color::Rgb(255, 165, 0)),
            AgentState::NeedsPermissions => ("●", Color::Rgb(138, 43, 226)),
            AgentState::Done => ("●", Color::Green),
        }
    }
}

#[must_use]
pub fn label(state: AgentState) -> (&'static str, Color) {
    AgentIndicator::new(state).label()
}

#[must_use]
pub fn dot(state: AgentState) -> (&'static str, Color) {
    AgentIndicator::new(state).dot()
}

#[must_use]
pub fn dot_visible(state: AgentState) -> (&'static str, Color) {
    AgentIndicator::new(state).dot_visible()
}
