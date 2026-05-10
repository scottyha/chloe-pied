use crate::activity::types::{ActivityEvent, ActivityEventType};
use chrono::{DateTime, Utc};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

const MAXIMUM_DETAIL_EVENTS: usize = 3;
const DETAIL_DESCRIPTION_MAXIMUM_CHARACTERS: usize = 72;
const DETAIL_DESCRIPTION_TRUNCATED_CHARACTERS: usize = 69;
const ACTIVITY_BULLET: &str = "  ● ";
const DETAIL_INDENT: &str = "    ";

#[must_use]
pub fn format_activity_digest<'a>(
    events: &'a [&'a ActivityEvent],
    since: Option<DateTime<Utc>>,
) -> Vec<Line<'a>> {
    let visible_events = filter_events_since(events, since);
    let counts = ActivityDigestCounts::from_events(&visible_events);

    let mut lines = vec![activity_header_line()];

    if let Some(since_time) = since {
        lines.push(Line::from(Span::styled(
            format!("Since {}", since_time.format("%H:%M")),
            Style::default().fg(Color::Gray),
        )));
    }

    if visible_events.is_empty() {
        lines.push(empty_activity_line());
        return lines;
    }

    append_category_lines(&mut lines, &counts);
    append_recent_event_lines(&mut lines, &visible_events);

    lines
}

fn filter_events_since<'a>(
    events: &'a [&'a ActivityEvent],
    since: Option<DateTime<Utc>>,
) -> Vec<&'a ActivityEvent> {
    let Some(since_time) = since else {
        return events.to_vec();
    };

    events
        .iter()
        .copied()
        .filter(|event| event.timestamp >= since_time)
        .collect()
}

fn activity_header_line() -> Line<'static> {
    Line::from(Span::styled(
        "Activity",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    ))
}

fn empty_activity_line() -> Line<'static> {
    Line::from(Span::styled(
        "  No activity recorded",
        Style::default().fg(Color::DarkGray),
    ))
}

fn append_category_lines(lines: &mut Vec<Line<'static>>, counts: &ActivityDigestCounts) {
    append_category_line(lines, counts.commands, "commands", Color::Green);
    append_category_line(lines, counts.files_changed, "files changed", Color::Blue);
    append_category_line(
        lines,
        counts.tasks_completed,
        "tasks completed",
        Color::Magenta,
    );
    append_category_line(lines, counts.errors, "errors", Color::Red);
    append_category_line(lines, counts.notifications, "notifications", Color::Yellow);
}

fn append_category_line(
    lines: &mut Vec<Line<'static>>,
    count: usize,
    label: &'static str,
    color: Color,
) {
    if count == 0 {
        return;
    }

    lines.push(Line::from(vec![
        Span::raw(ACTIVITY_BULLET),
        Span::styled(format!("{count} {label}"), Style::default().fg(color)),
    ]));
}

fn append_recent_event_lines(lines: &mut Vec<Line<'static>>, events: &[&ActivityEvent]) {
    for event in events.iter().rev().take(MAXIMUM_DETAIL_EVENTS) {
        lines.push(Line::from(vec![
            Span::raw(DETAIL_INDENT),
            Span::styled(
                format!("{} ", event.timestamp.format("%H:%M")),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                truncate_description(event.description.lines().next().unwrap_or("")),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::DIM),
            ),
        ]));
    }
}

fn truncate_description(description: &str) -> String {
    if description.chars().count() <= DETAIL_DESCRIPTION_MAXIMUM_CHARACTERS {
        return description.to_string();
    }

    let mut truncated: String = description
        .chars()
        .take(DETAIL_DESCRIPTION_TRUNCATED_CHARACTERS)
        .collect();
    truncated.push('…');
    truncated
}

#[derive(Default)]
struct ActivityDigestCounts {
    commands: usize,
    files_changed: usize,
    tasks_completed: usize,
    errors: usize,
    notifications: usize,
}

impl ActivityDigestCounts {
    fn from_events(events: &[&ActivityEvent]) -> Self {
        let mut counts = Self::default();

        for event in events {
            match event.event_type {
                ActivityEventType::CommandExecuted => counts.commands += 1,
                ActivityEventType::FileChanged => counts.files_changed += 1,
                ActivityEventType::TaskCompleted => counts.tasks_completed += 1,
                ActivityEventType::ErrorOccurred => counts.errors += 1,
                ActivityEventType::ProviderNotification => counts.notifications += 1,
            }
        }

        counts
    }
}
