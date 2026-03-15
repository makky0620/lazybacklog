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

    render_title(frame, chunks[0], state);
    render_content(frame, chunks[1], state);
    render_help_bar(frame, chunks[2]);
}

fn render_title(frame: &mut Frame, area: Rect, state: &AppState) {
    let title = format!(" lazybacklog ──── [{}] ", state.current_space_name());
    let paragraph = Paragraph::new(title).style(
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    );
    frame.render_widget(paragraph, area);
}

fn render_content(frame: &mut Frame, area: Rect, state: &AppState) {
    let space_state = state.current_space_state();

    if space_state.loading_projects {
        let loading = Paragraph::new("Loading projects...").style(Style::default().fg(Color::Gray));
        frame.render_widget(loading, area);
        return;
    }

    let projects = match &space_state.projects {
        Some(p) if !p.is_empty() => p,
        _ => {
            let msg = Paragraph::new("No projects found.").style(Style::default().fg(Color::Gray));
            frame.render_widget(msg, area);
            return;
        }
    };

    let items: Vec<ListItem> = projects
        .iter()
        .map(|p| ListItem::new(format!("{} - {}", p.project_key, p.name)))
        .collect();

    let list = List::new(items)
        .highlight_style(
            Style::default()
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    let mut list_state = ListState::default();
    list_state.select(Some(state.project_cursor_idx));

    frame.render_stateful_widget(list, area, &mut list_state);
}

fn render_help_bar(frame: &mut Frame, area: Rect) {
    let text = " [j/k] Move  [Enter] Select  [q] Quit";
    let paragraph = Paragraph::new(text).style(Style::default().fg(Color::DarkGray));
    frame.render_widget(paragraph, area);
}
