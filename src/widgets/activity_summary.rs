use crate::views::instances::state::{ActivitySummary, ActivitySummaryMode};
use chrono::Utc;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Padding, Paragraph, Widget, Wrap},
};

const DIALOG_WIDTH_THRESHOLD: u16 = 100;
const DIALOG_WIDTH_SMALL: u16 = 70;
const DIALOG_WIDTH_NORMAL: u16 = 90;
const DIALOG_HEIGHT_PERCENT: u16 = 80;
const HEADER_HEIGHT: u16 = 3;

pub struct ActivitySummaryWidget<'a> {
    summary: &'a ActivitySummary,
    activity_summary_mode: ActivitySummaryMode,
    scroll_offset: usize,
}

impl<'a> ActivitySummaryWidget<'a> {
    #[must_use]
    pub const fn new(summary: &'a ActivitySummary, mode: ActivitySummaryMode) -> Self {
        Self {
            summary,
            activity_summary_mode: mode,
            scroll_offset: 0,
        }
    }

    #[must_use]
    pub const fn scroll_offset(mut self, offset: usize) -> Self {
        self.scroll_offset = offset;
        self
    }
}

impl Widget for ActivitySummaryWidget<'_> {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        let dialog_width = if area.width < DIALOG_WIDTH_THRESHOLD {
            DIALOG_WIDTH_SMALL
        } else {
            DIALOG_WIDTH_NORMAL
        }
        .min(area.width.saturating_sub(2));

        let dialog_height =
            (area.height * DIALOG_HEIGHT_PERCENT / 100).min(area.height.saturating_sub(2));
        let popup_area = centered_rect(dialog_width, dialog_height, area);

        Clear.render(popup_area, buffer);

        let title = match self.activity_summary_mode {
            ActivitySummaryMode::SinceLastViewed => " Activity Summary ",
            ActivitySummaryMode::FullHistory => " Full Activity History ",
        };

        let block = Block::default()
            .title(title)
            .title_style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .padding(Padding::uniform(1));

        let inner_area = block.inner(popup_area);
        block.render(popup_area, buffer);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(HEADER_HEIGHT), Constraint::Min(0)])
            .split(inner_area);

        render_header(self.summary, self.activity_summary_mode, chunks[0], buffer);
        render_events(self.summary, chunks[1], buffer, self.scroll_offset);
    }
}

fn render_header(
    summary: &ActivitySummary,
    mode: ActivitySummaryMode,
    area: Rect,
    buffer: &mut Buffer,
) {
    let header_lines = vec![
        render_time_line(summary, mode),
        Line::from(vec![
            Span::styled("Activity: ", Style::default().fg(Color::Gray)),
            Span::styled(
                summary.format_as_summary_line(),
                Style::default().fg(Color::Cyan),
            ),
            Span::styled("  (f: toggle mode)", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(""),
    ];

    Paragraph::new(header_lines).render(area, buffer);
}

fn render_time_line(summary: &ActivitySummary, mode: ActivitySummaryMode) -> Line<'static> {
    let timestamp_style = Style::default()
        .fg(Color::White)
        .add_modifier(Modifier::BOLD);

    match mode {
        ActivitySummaryMode::SinceLastViewed => {
            let elapsed_minutes = summary.elapsed_seconds / 60;
            let elapsed_seconds = summary.elapsed_seconds % 60;
            let time_display = if elapsed_minutes > 0 {
                format!("{elapsed_minutes}m {elapsed_seconds}s ago")
            } else {
                format!("{elapsed_seconds}s ago")
            };

            Line::from(vec![
                Span::styled("Since: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    summary.since.format("%H:%M:%S").to_string(),
                    timestamp_style,
                ),
                Span::styled(
                    format!(" ({time_display})"),
                    Style::default().fg(Color::DarkGray),
                ),
            ])
        }
        ActivitySummaryMode::FullHistory => Line::from(vec![
            Span::styled("Range: ", Style::default().fg(Color::Gray)),
            Span::styled(
                summary.since.format("%H:%M:%S").to_string(),
                timestamp_style,
            ),
            Span::styled(" → ", Style::default().fg(Color::DarkGray)),
            Span::styled(Utc::now().format("%H:%M:%S").to_string(), timestamp_style),
        ]),
    }
}

fn render_events(summary: &ActivitySummary, area: Rect, buffer: &mut Buffer, scroll_offset: usize) {
    let mut items = Vec::new();

    add_commands_section(&mut items, summary);
    add_files_section(&mut items, summary);
    add_tasks_section(&mut items, summary);
    add_errors_section(&mut items, summary);
    add_notifications_section(&mut items, summary);

    if items.is_empty() {
        Paragraph::new("No activity recorded")
            .style(Style::default().fg(Color::DarkGray))
            .wrap(Wrap { trim: false })
            .render(area, buffer);
        return;
    }

    render_activity_list(items, area, buffer, scroll_offset);
}

fn add_commands_section<'a>(items: &mut Vec<ListItem<'a>>, summary: &'a ActivitySummary) {
    if summary.commands_executed.is_empty() {
        return;
    }

    items.push(create_section_header(
        "Commands Executed",
        summary.commands_executed.len(),
        Color::Green,
    ));

    for command in &summary.commands_executed {
        items.push(create_list_item(command, Color::White));
    }
    items.push(ListItem::new(Line::from("")));
}

fn add_files_section<'a>(items: &mut Vec<ListItem<'a>>, summary: &'a ActivitySummary) {
    if summary.files_changed.is_empty() {
        return;
    }

    items.push(create_section_header(
        "Files Changed",
        summary.files_changed.len(),
        Color::Blue,
    ));

    for file in &summary.files_changed {
        items.push(create_list_item(file, Color::White));
    }
    items.push(ListItem::new(Line::from("")));
}

fn add_tasks_section<'a>(items: &mut Vec<ListItem<'a>>, summary: &'a ActivitySummary) {
    if summary.tasks_completed == 0 {
        return;
    }

    items.push(create_section_header(
        "Tasks Completed",
        summary.tasks_completed,
        Color::Magenta,
    ));
    items.push(ListItem::new(Line::from("")));
}

fn add_errors_section<'a>(items: &mut Vec<ListItem<'a>>, summary: &'a ActivitySummary) {
    if summary.errors.is_empty() {
        return;
    }

    items.push(create_section_header(
        "Errors",
        summary.errors.len(),
        Color::Red,
    ));

    for error in &summary.errors {
        items.push(create_list_item(error, Color::Red));
    }
    items.push(ListItem::new(Line::from("")));
}

fn add_notifications_section<'a>(items: &mut Vec<ListItem<'a>>, summary: &'a ActivitySummary) {
    if summary.notifications.is_empty() {
        return;
    }

    items.push(create_section_header(
        "Provider Notifications",
        summary.notifications.len(),
        Color::Yellow,
    ));

    for notification in &summary.notifications {
        items.push(create_list_item(notification, Color::Yellow));
    }
}

fn create_section_header(title: &str, count: usize, color: Color) -> ListItem<'_> {
    ListItem::new(Line::from(vec![
        Span::styled(
            format!("{title} "),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!("({count})"), Style::default().fg(Color::DarkGray)),
    ]))
}

fn create_list_item(text: &str, color: Color) -> ListItem<'_> {
    ListItem::new(Line::from(vec![
        Span::raw("  ● "),
        Span::styled(text, Style::default().fg(color)),
    ]))
}

fn render_activity_list(
    items: Vec<ListItem>,
    area: Rect,
    buffer: &mut Buffer,
    scroll_offset: usize,
) {
    let maximum_scroll_offset = items.len().saturating_sub(area.height as usize);
    let visible_scroll_offset = scroll_offset.min(maximum_scroll_offset);
    let visible_items: Vec<ListItem> = items
        .into_iter()
        .skip(visible_scroll_offset)
        .take(area.height as usize)
        .collect();

    List::new(visible_items).render(area, buffer);
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let vertical_margin = area.height.saturating_sub(height) / 2;
    let horizontal_margin = area.width.saturating_sub(width) / 2;

    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(vertical_margin),
            Constraint::Length(height),
            Constraint::Min(0),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(horizontal_margin),
            Constraint::Length(width),
            Constraint::Min(0),
        ])
        .split(popup_layout[1])[1]
}
