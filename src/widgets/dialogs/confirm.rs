use super::style::DialogStyle;
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Padding, Paragraph, Widget, Wrap},
};

const DIALOG_WIDTH_THRESHOLD: u16 = 80;
const DIALOG_WIDTH_SMALL: u16 = 80;
const DIALOG_WIDTH_NORMAL: u16 = 60;
const DIALOG_MIN_HEIGHT: u16 = 7;
const DIALOG_MAX_HEIGHT: u16 = 15;
const DIALOG_BORDER_AND_PADDING: u16 = 4;
const DIALOG_CONTENT_ROWS_ABOVE_MESSAGE: u16 = 1;

pub struct ConfirmDialog<'a> {
    title: &'a str,
    message: &'a str,
    style: DialogStyle,
}

impl<'a> ConfirmDialog<'a> {
    #[must_use]
    pub fn new(title: &'a str, message: &'a str) -> Self {
        Self {
            title,
            message,
            style: DialogStyle::default(),
        }
    }

    #[must_use]
    pub const fn style(mut self, style: DialogStyle) -> Self {
        self.style = style;
        self
    }
}

impl Widget for ConfirmDialog<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let dialog_width = if area.width < DIALOG_WIDTH_THRESHOLD {
            DIALOG_WIDTH_SMALL
        } else {
            DIALOG_WIDTH_NORMAL
        };

        let content_width = dialog_width.saturating_sub(DIALOG_BORDER_AND_PADDING);
        let wrapped_lines = calculate_wrapped_line_count(self.message, content_width);
        let dialog_height =
            (wrapped_lines + DIALOG_CONTENT_ROWS_ABOVE_MESSAGE + DIALOG_BORDER_AND_PADDING)
                .clamp(DIALOG_MIN_HEIGHT, DIALOG_MAX_HEIGHT)
                .min(area.height.saturating_sub(2));

        let popup_area = centered_rect(dialog_width, dialog_height, area);
        let border_color = self.style.color();

        Clear.render(popup_area, buf);

        let block = Block::default()
            .title(self.title)
            .title_style(
                Style::default()
                    .fg(border_color)
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .padding(Padding::uniform(1));

        let inner_area = block.inner(popup_area);
        block.render(popup_area, buf);

        let text = Paragraph::new(vec![Line::from(""), Line::from(Span::raw(self.message))])
            .wrap(Wrap { trim: true })
            .alignment(Alignment::Center);

        text.render(inner_area, buf);
    }
}

fn calculate_wrapped_line_count(text: &str, max_width: u16) -> u16 {
    if text.is_empty() || max_width == 0 {
        return 1;
    }

    let max_width = max_width as usize;
    let mut line_count: u16 = 0;

    for line in text.split('\n') {
        if line.is_empty() {
            line_count += 1;
            continue;
        }

        line_count += count_wrapped_lines_for_string(line, max_width);
    }

    line_count.max(1)
}

fn count_wrapped_lines_for_string(text: &str, max_width: usize) -> u16 {
    let mut line_count: u16 = 0;
    let mut current_line_length = 0;

    for word in text.split_whitespace() {
        let word_length = word.chars().count();

        if current_line_length == 0 {
            current_line_length = word_length;
        } else if current_line_length + 1 + word_length <= max_width {
            current_line_length += 1 + word_length;
        } else {
            line_count += 1;
            current_line_length = word_length;
        }
    }

    if current_line_length > 0 {
        line_count += 1;
    }

    line_count
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
