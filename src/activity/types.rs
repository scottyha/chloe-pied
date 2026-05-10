use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ActivitySummaryMode {
    #[default]
    SinceLastViewed,
    FullHistory,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActivityEventType {
    CommandExecuted,
    FileChanged,
    TaskCompleted,
    ErrorOccurred,
    ProviderNotification,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivityEvent {
    #[serde(default)]
    pub pane_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub event_type: ActivityEventType,
    pub description: String,
    pub metadata: Option<String>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ActivitySummary {
    pub since: DateTime<Utc>,
    pub elapsed_seconds: i64,
    pub commands_executed: Vec<String>,
    pub files_changed: Vec<String>,
    pub errors: Vec<String>,
    pub notifications: Vec<String>,
    pub tasks_completed: usize,
}

impl ActivitySummary {
    #[must_use]
    pub fn from_events(since: DateTime<Utc>, events: &[&ActivityEvent]) -> Option<Self> {
        if events.is_empty() {
            return None;
        }

        let mut commands_executed = Vec::new();
        let mut files_changed = Vec::new();
        let mut errors = Vec::new();
        let mut notifications = Vec::new();
        let mut tasks_completed = 0;

        for event in events {
            match event.event_type {
                ActivityEventType::CommandExecuted => {
                    commands_executed.push(event.description.clone());
                }
                ActivityEventType::FileChanged => {
                    files_changed.push(event.description.clone());
                }
                ActivityEventType::ErrorOccurred => {
                    errors.push(event.description.clone());
                }
                ActivityEventType::ProviderNotification => {
                    notifications.push(event.description.clone());
                }
                ActivityEventType::TaskCompleted => {
                    tasks_completed += 1;
                }
            }
        }

        let elapsed = Utc::now().signed_duration_since(since);

        Some(Self {
            since,
            elapsed_seconds: elapsed.num_seconds(),
            commands_executed,
            files_changed,
            errors,
            notifications,
            tasks_completed,
        })
    }

    #[must_use]
    #[allow(dead_code)]
    pub fn format_as_text(&self) -> String {
        use std::fmt::Write;
        let mut output = String::new();

        output.push_str("Activity Summary\n");
        let _ = write!(
            output,
            "Since: {} ({} seconds ago)\n\n",
            self.since.format("%H:%M:%S"),
            self.elapsed_seconds
        );

        if !self.commands_executed.is_empty() {
            let _ = writeln!(
                output,
                "Commands executed ({}):",
                self.commands_executed.len()
            );
            for command in &self.commands_executed {
                let _ = writeln!(output, "  • {command}");
            }
            output.push('\n');
        }

        if !self.files_changed.is_empty() {
            let _ = writeln!(output, "Files changed ({}):", self.files_changed.len());
            for file in &self.files_changed {
                let _ = writeln!(output, "  • {file}");
            }
            output.push('\n');
        }

        if self.tasks_completed > 0 {
            let _ = write!(output, "Tasks completed: {}\n\n", self.tasks_completed);
        }

        if !self.errors.is_empty() {
            let _ = writeln!(output, "Errors ({}):", self.errors.len());
            for error in &self.errors {
                let _ = writeln!(output, "  • {error}");
            }
            output.push('\n');
        }

        if !self.notifications.is_empty() {
            let _ = writeln!(output, "Notifications ({}):", self.notifications.len());
            for notification in &self.notifications {
                let _ = writeln!(output, "  • {notification}");
            }
        }

        output
    }

    #[must_use]
    #[allow(dead_code)]
    pub fn format_as_summary_line(&self) -> String {
        let mut parts = Vec::new();

        if !self.commands_executed.is_empty() {
            parts.push(format!("{} commands", self.commands_executed.len()));
        }

        if !self.files_changed.is_empty() {
            parts.push(format!("{} files changed", self.files_changed.len()));
        }

        if self.tasks_completed > 0 {
            parts.push(format!("{} tasks done", self.tasks_completed));
        }

        if !self.errors.is_empty() {
            parts.push(format!("{} errors", self.errors.len()));
        }

        if parts.is_empty() {
            "No significant activity".to_string()
        } else {
            parts.join(", ")
        }
    }
}
