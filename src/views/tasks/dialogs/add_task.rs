use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Padding, Paragraph, Wrap},
};

use super::render_popup_background;

const PERCENTAGE_FULL: u16 = 100;
const POPUP_WIDTH_PERCENT: u16 = 70;
const POPUP_HEIGHT_MIN_PERCENT: u16 = 45;
const POPUP_HEIGHT_MAX_PERCENT: u16 = 80;
const TIP_BLOCK_HEIGHT: u16 = 7;
const VERTICAL_GAP: u16 = 1;
const PROMPT_LINE_COUNT: u16 = 2;
const BORDER_AND_PADDING: u16 = 4;
const CURSOR_CHARACTER_WIDTH: u16 = 1;

pub struct AddTaskDialogState<'a> {
    pub input: &'a str,
    pub prompt: &'a str,
}

pub fn render_add_task_dialog(frame: &mut Frame, state: &AddTaskDialogState<'_>, area: Rect) {
    let popup_width = percentage_of(area.width, POPUP_WIDTH_PERCENT);
    let content_width = popup_width.saturating_sub(BORDER_AND_PADDING);
    let input_text_width = content_width.saturating_sub(CURSOR_CHARACTER_WIDTH);
    let wrapped_line_count = calculate_wrapped_line_count(state.input, input_text_width);

    let popup_layout = calculate_popup_layout(area, wrapped_line_count);
    let popup_area = centered_area(popup_width, popup_layout.height, area);
    render_popup_background(frame, popup_area);

    let outer_block = Block::default()
        .title(" Add Task ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .padding(Padding::uniform(1));

    let inner_area = outer_block.inner(popup_area);
    frame.render_widget(outer_block, popup_area);

    if !popup_layout.show_tip_block {
        render_input_area(frame, inner_area, state, popup_layout.scroll_offset);
        return;
    }

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(VERTICAL_GAP),
            Constraint::Length(TIP_BLOCK_HEIGHT),
        ])
        .split(inner_area);

    render_input_area(frame, layout[0], state, popup_layout.scroll_offset);
    render_tip_block(frame, layout[2]);
}

struct AddTaskPopupLayout {
    height: u16,
    scroll_offset: u16,
    show_tip_block: bool,
}

const fn calculate_popup_layout(area: Rect, wrapped_line_count: u16) -> AddTaskPopupLayout {
    let minimum_height = percentage_of(area.height, POPUP_HEIGHT_MIN_PERCENT);
    let maximum_height = percentage_of(area.height, POPUP_HEIGHT_MAX_PERCENT);
    let required_input_height = PROMPT_LINE_COUNT + wrapped_line_count;
    let required_total_height =
        required_input_height + VERTICAL_GAP + TIP_BLOCK_HEIGHT + BORDER_AND_PADDING;

    if required_total_height <= minimum_height {
        return layout_with_tip(minimum_height, wrapped_line_count);
    }

    if required_total_height <= maximum_height {
        return layout_with_tip(required_total_height, wrapped_line_count);
    }

    layout_without_tip(maximum_height, wrapped_line_count)
}

const fn layout_with_tip(height: u16, wrapped_line_count: u16) -> AddTaskPopupLayout {
    let input_area_height = height
        .saturating_sub(BORDER_AND_PADDING)
        .saturating_sub(VERTICAL_GAP)
        .saturating_sub(TIP_BLOCK_HEIGHT);

    AddTaskPopupLayout {
        height,
        scroll_offset: calculate_scroll_offset(wrapped_line_count, input_area_height),
        show_tip_block: true,
    }
}

const fn layout_without_tip(height: u16, wrapped_line_count: u16) -> AddTaskPopupLayout {
    let input_area_height = height.saturating_sub(BORDER_AND_PADDING);

    AddTaskPopupLayout {
        height,
        scroll_offset: calculate_scroll_offset(wrapped_line_count, input_area_height),
        show_tip_block: false,
    }
}

const fn calculate_scroll_offset(wrapped_line_count: u16, input_area_height: u16) -> u16 {
    let visible_input_lines = input_area_height.saturating_sub(PROMPT_LINE_COUNT);
    wrapped_line_count.saturating_sub(visible_input_lines)
}

fn render_input_area(
    frame: &mut Frame,
    area: Rect,
    state: &AddTaskDialogState<'_>,
    scroll_offset: u16,
) {
    let prompt_line = Line::from(Span::styled(
        state.prompt,
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    ));

    let prompt_paragraph = Paragraph::new(prompt_line);
    frame.render_widget(prompt_paragraph, area);

    let input_area = Rect {
        y: area.y + PROMPT_LINE_COUNT,
        height: area.height.saturating_sub(PROMPT_LINE_COUNT),
        ..area
    };

    let input_with_cursor = format!("{}▏", state.input);
    let input_paragraph = Paragraph::new(input_with_cursor)
        .style(Style::default().fg(Color::White))
        .wrap(Wrap { trim: false })
        .scroll((scroll_offset, 0));

    frame.render_widget(input_paragraph, input_area);
}

fn render_tip_block(frame: &mut Frame, area: Rect) {
    let block = Block::default()
        .title(" How It Works ")
        .title_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .padding(Padding::horizontal(1));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let content = vec![
        Line::from(""),
        Line::from(Span::styled(
            "Just describe your task briefly - an AI agent will",
            Style::default().fg(Color::Gray),
        )),
        Line::from(Span::styled(
            "automatically expand it into a full task with title,",
            Style::default().fg(Color::Gray),
        )),
        Line::from(Span::styled(
            "description, and relevant tags.",
            Style::default().fg(Color::Gray),
        )),
    ];

    frame.render_widget(Paragraph::new(content), inner);
}

const fn percentage_of(value: u16, percent: u16) -> u16 {
    value.saturating_mul(percent) / PERCENTAGE_FULL
}

fn centered_area(width: u16, height: u16, area: Rect) -> Rect {
    let vertical_margin = area.height.saturating_sub(height) / 2;
    let horizontal_margin = area.width.saturating_sub(width) / 2;

    let vertical_layout = Layout::default()
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
        .split(vertical_layout[1])[1]
}

fn calculate_wrapped_line_count(text: &str, maximum_width: u16) -> u16 {
    if text.is_empty() || maximum_width == 0 {
        return 1;
    }

    let maximum_width = maximum_width as usize;
    let mut line_count: u16 = 0;

    for line in text.split('\n') {
        if line.is_empty() {
            line_count += 1;
            continue;
        }

        let mut current_line_length = 0;
        for word in line.split_whitespace() {
            let word_length = word.chars().count();

            if current_line_length == 0 {
                current_line_length = word_length;
            } else if current_line_length + 1 + word_length <= maximum_width {
                current_line_length += 1 + word_length;
            } else {
                line_count += 1;
                current_line_length = word_length;
            }
        }

        if current_line_length > 0 {
            line_count += 1;
        }
    }

    line_count.max(1)
}
