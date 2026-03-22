use crate::app::AppState;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{List, ListItem, ListState, Paragraph},
    Frame,
};

pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // title bar
            Constraint::Min(0),    // content
            Constraint::Length(1), // help bar
        ])
        .split(area);

    render_title(frame, chunks[0]);
    render_content(frame, chunks[1], state);
    render_help_bar(frame, chunks[2]);
}

fn render_title(frame: &mut Frame, area: Rect) {
    let paragraph = Paragraph::new(" lazybacklog").style(
        Style::default()
            .bg(Color::Cyan)
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD),
    );
    frame.render_widget(paragraph, area);
}

fn render_content(frame: &mut Frame, area: Rect, state: &AppState) {
    let items: Vec<ListItem> = state
        .config
        .spaces
        .iter()
        .map(|s| ListItem::new(s.name.clone()))
        .collect();

    let list = List::new(items)
        .highlight_style(
            Style::default()
                .bg(Color::White)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    let mut list_state = ListState::default();
    list_state.select(Some(state.space_cursor_idx));

    frame.render_stateful_widget(list, area, &mut list_state);
}

fn render_help_bar(frame: &mut Frame, area: Rect) {
    let text = " [j/k] Move  [Enter] Select  [q] Quit";
    let paragraph = Paragraph::new(text).style(Style::default().fg(Color::DarkGray));
    frame.render_widget(paragraph, area);
}
