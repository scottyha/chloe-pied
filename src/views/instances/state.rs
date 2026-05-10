pub use crate::activity::types::{ActivityEvent, ActivityEventType, ActivitySummary};
use crate::events::AppEvent;
use crate::types::AgentProvider;
use alacritty_terminal::grid::Dimensions;
use chrono::{DateTime, Utc};
use ratatui::layout::Rect;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::PathBuf;
use tokio::sync::mpsc;
use uuid::Uuid;

const MAX_ACTIVITY_EVENTS: usize = 500;
const ACTIVITY_RETENTION_DAYS: i64 = 7;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SplitDirection {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PaneNode {
    Leaf(Box<InstancePane>),
    Split {
        direction: SplitDirection,
        ratio: f32,
        first: Box<Self>,
        second: Box<Self>,
    },
}

impl PaneNode {
    #[must_use]
    pub fn collect_panes(&self) -> Vec<&InstancePane> {
        match self {
            Self::Leaf(pane) => vec![pane],
            Self::Split { first, second, .. } => {
                let mut panes = first.collect_panes();
                panes.extend(second.collect_panes());
                panes
            }
        }
    }

    #[must_use]
    pub fn find_pane(&self, id: Uuid) -> Option<&InstancePane> {
        match self {
            Self::Leaf(pane) if pane.id == id => Some(pane),
            Self::Leaf(_) => None,
            Self::Split { first, second, .. } => {
                first.find_pane(id).or_else(|| second.find_pane(id))
            }
        }
    }

    pub fn find_pane_mut(&mut self, id: Uuid) -> Option<&mut InstancePane> {
        match self {
            Self::Leaf(pane) if pane.id == id => Some(pane),
            Self::Leaf(_) => None,
            Self::Split { first, second, .. } => {
                first.find_pane_mut(id).or_else(|| second.find_pane_mut(id))
            }
        }
    }

    #[must_use]
    pub fn first_pane_id(&self) -> Uuid {
        match self {
            Self::Leaf(pane) => pane.id,
            Self::Split { first, .. } => first.first_pane_id(),
        }
    }

    #[must_use]
    pub fn pane_count(&self) -> usize {
        match self {
            Self::Leaf(_) => 1,
            Self::Split { first, second, .. } => first.pane_count() + second.pane_count(),
        }
    }

    pub fn for_each_pane_mut<F>(&mut self, function: &mut F)
    where
        F: FnMut(&mut InstancePane),
    {
        match self {
            Self::Leaf(pane) => function(pane),
            Self::Split { first, second, .. } => {
                first.for_each_pane_mut(function);
                second.for_each_pane_mut(function);
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceState {
    pub root: Option<PaneNode>,
    pub selected_pane_id: Option<Uuid>,
    pub mode: InstanceMode,
    #[serde(skip)]
    pub last_render_area: Option<Rect>,
    #[serde(skip, default)]
    pub pane_areas: Vec<(Uuid, Rect)>,
    #[serde(skip, default)]
    pub activity_summary_scroll_offset: usize,
    #[serde(skip)]
    event_sender: Option<mpsc::UnboundedSender<AppEvent>>,
}

impl InstanceState {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            root: None,
            selected_pane_id: None,
            mode: InstanceMode::Normal,
            last_render_area: None,
            pane_areas: Vec::new(),
            activity_summary_scroll_offset: 0,
            event_sender: None,
        }
    }

    pub fn set_event_sender(&mut self, sender: mpsc::UnboundedSender<AppEvent>) {
        self.event_sender = Some(sender);
    }

    #[must_use]
    pub fn event_sender(&self) -> Option<mpsc::UnboundedSender<AppEvent>> {
        self.event_sender.clone()
    }

    pub fn process_pty_output(&mut self, pane_id: Uuid, data: &[u8]) {
        let Some(pane) = self.find_pane_mut(pane_id) else {
            return;
        };

        if let Ok(text) = String::from_utf8(data.to_vec()) {
            super::activity::detect_and_log_activity(pane, &text);
        }
    }

    pub fn handle_pty_exit(&mut self, pane_id: Uuid) {
        let Some(pane) = self.find_pane_mut(pane_id) else {
            return;
        };

        pane.agent_state = AgentState::Done;
    }

    pub fn selected_pane_mut(&mut self) -> Option<&mut InstancePane> {
        let id = self.selected_pane_id?;
        self.root.as_mut()?.find_pane_mut(id)
    }

    #[must_use]
    pub fn pane_count(&self) -> usize {
        self.root.as_ref().map_or(0, PaneNode::pane_count)
    }

    #[must_use]
    pub fn collect_panes(&self) -> Vec<&InstancePane> {
        self.root
            .as_ref()
            .map_or_else(Vec::new, PaneNode::collect_panes)
    }

    #[must_use]
    pub fn find_pane(&self, id: Uuid) -> Option<&InstancePane> {
        self.root.as_ref()?.find_pane(id)
    }

    pub fn find_pane_mut(&mut self, id: Uuid) -> Option<&mut InstancePane> {
        self.root.as_mut()?.find_pane_mut(id)
    }

    #[must_use]
    pub fn get_pane_area(&self, id: Uuid) -> Option<Rect> {
        self.pane_areas
            .iter()
            .find(|(pane_id, _)| *pane_id == id)
            .map(|(_, area)| *area)
    }

    pub fn prune_all_activity_events(&mut self) {
        if let Some(root) = &mut self.root {
            root.for_each_pane_mut(&mut |pane| {
                pane.prune_old_activity_events();
            });
        }
    }
}

impl Default for InstanceState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AgentState {
    #[default]
    Idle,
    Running,
    NeedsPermissions,
    Done,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstancePane {
    pub id: Uuid,
    pub name: Option<String>,
    pub working_directory: PathBuf,
    #[serde(default)]
    pub provider: AgentProvider,
    pub rows: u16,
    pub columns: u16,
    #[serde(skip)]
    pub pty_session: Option<super::pty::PtySession>,
    #[serde(skip, default)]
    pub pty_spawn_error: Option<String>,
    #[serde(default)]
    pub agent_state: AgentState,
    #[serde(skip, default)]
    pub scroll_offset: usize,
    pub last_viewed_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub activity_events: VecDeque<ActivityEvent>,
}

impl InstancePane {
    #[must_use]
    pub fn new(working_directory: PathBuf, rows: u16, columns: u16) -> Self {
        Self::with_provider(working_directory, rows, columns, AgentProvider::default())
    }

    #[must_use]
    pub fn with_provider(
        working_directory: PathBuf,
        rows: u16,
        columns: u16,
        provider: AgentProvider,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: None,
            working_directory,
            provider,
            rows,
            columns,
            pty_session: None,
            pty_spawn_error: None,
            agent_state: AgentState::Idle,
            scroll_offset: 0,
            last_viewed_at: Some(Utc::now()),
            activity_events: VecDeque::new(),
        }
    }

    pub fn scroll_up(&mut self, lines: usize, max_scrollback: usize) {
        self.scroll_offset = (self.scroll_offset + lines).min(max_scrollback);
    }

    pub const fn scroll_down(&mut self, lines: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(lines);
    }

    pub const fn scroll_to_bottom(&mut self) {
        self.scroll_offset = 0;
    }

    #[must_use]
    pub fn scrollback_len(&self) -> usize {
        let Some(session) = &self.pty_session else {
            return 0;
        };
        let term_mutex = session.term();
        let Ok(term) = term_mutex.lock() else {
            return 0;
        };
        term.grid().history_size()
    }

    pub fn mark_viewed(&mut self) {
        self.last_viewed_at = Some(Utc::now());
    }

    pub fn add_activity_event(
        &mut self,
        event_type: ActivityEventType,
        description: String,
        metadata: Option<String>,
    ) {
        let event = ActivityEvent {
            timestamp: Utc::now(),
            event_type,
            description,
            metadata,
        };

        self.activity_events.push_back(event);
        self.prune_old_activity_events();
    }

    pub fn prune_old_activity_events(&mut self) {
        let cutoff_time = Utc::now() - chrono::Duration::days(ACTIVITY_RETENTION_DAYS);

        self.activity_events
            .retain(|event| event.timestamp > cutoff_time);

        while self.activity_events.len() > MAX_ACTIVITY_EVENTS {
            self.activity_events.pop_front();
        }
    }

    #[must_use]
    #[allow(dead_code)]
    pub fn get_events_since(&self, since: DateTime<Utc>) -> Vec<&ActivityEvent> {
        self.activity_events
            .iter()
            .filter(|event| event.timestamp > since)
            .collect()
    }

    #[must_use]
    #[allow(dead_code)]
    pub fn generate_activity_summary(&self) -> Option<ActivitySummary> {
        let since = self.last_viewed_at?;
        let events = self.get_events_since(since);

        ActivitySummary::from_events(since, &events)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InstanceMode {
    Normal,
    Focused,
    Scroll,
    ActivitySummary,
}
