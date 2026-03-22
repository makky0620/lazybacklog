use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::{AppState, Screen};

pub mod filter;
pub mod issue_detail;
pub mod issue_list;
pub mod project_select;
pub mod space_select;
pub mod status_filter;

pub fn render(frame: &mut Frame, state: &AppState) {
    let area = frame.area();

    if state.screen == Screen::SpaceSelect {
        space_select::render(frame, area, state);
        render_status_message(frame, area, state);
        return;
    }

    if state.screen == Screen::ProjectSelect {
        // Full-screen takeover: render project select layout first,
        // then overlay status message on the bottom line.
        // Order matters: project_select::render paints the help bar,
        // render_status_message overlays it when an error is present.
        project_select::render(frame, area, state);
        render_status_message(frame, area, state);
        return;
    }

    if state.screen == Screen::IssueDetail {
        if let Some(issue) = &state.detail_issue {
            issue_detail::render(frame, area, issue, state.detail_scroll_offset);
        }
        render_status_message(frame, area, state);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // title bar
            Constraint::Min(0),    // block panel
            Constraint::Length(1), // help bar
        ])
        .split(area);

    render_title(frame, chunks[0], state);

    let panel_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    let panel_inner = panel_block.inner(chunks[1]);
    frame.render_widget(panel_block, chunks[1]);

    let inner_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // filter bar
            Constraint::Min(0),    // issue list
        ])
        .split(panel_inner);

    render_filter_bar(frame, inner_chunks[0], state);
    issue_list::render(frame, inner_chunks[1], state);
    render_help_bar(frame, chunks[2], state);

    match state.screen {
        Screen::Filter => {
            filter::render(frame, area, state);
        }
        Screen::IssueDetail => {} // dead code — early return above handles this; satisfies exhaustiveness
        Screen::IssueList => {}
        Screen::ProjectSelect => {} // dead code — early return above handles this; satisfies exhaustiveness
        Screen::SpaceSelect => {} // dead code — early return above handles this; satisfies exhaustiveness
        Screen::StatusFilter => {
            status_filter::render(frame, area, state);
        }
    }

    render_status_message(frame, area, state);
}

fn render_title(frame: &mut Frame, area: Rect, state: &AppState) {
    let title = format!(" lazybacklog ──── [{}] ", state.current_space_name());
    let paragraph = Paragraph::new(title).style(
        Style::default()
            .bg(Color::Cyan)
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD),
    );
    frame.render_widget(paragraph, area);
}

fn render_filter_bar(frame: &mut Frame, area: Rect, state: &AppState) {
    let space_state = state.current_space_state();
    let assignee_name = if let Some(aid) = state.filter_assignee_id {
        space_state
            .users
            .as_ref()
            .and_then(|users| users.iter().find(|u| u.id == aid))
            .map(|u| u.name.clone())
            .unwrap_or_else(|| format!("ID:{}", aid))
    } else {
        "ALL".to_string()
    };

    let status_text =
        status_filter::status_filter_text(&space_state.filter_status_ids, &space_state.statuses);

    let text = format!(" Assignee: {}  |  Status: {}", assignee_name, status_text);
    let paragraph = Paragraph::new(text).style(Style::default().fg(Color::Gray));
    frame.render_widget(paragraph, area);
}

fn render_help_bar(frame: &mut Frame, area: Rect, state: &AppState) {
    let text = if state.search_active {
        " [Enter] Confirm  [Esc] Cancel"
    } else if !state.search_query.is_empty() {
        " [j/k] Move  [Enter] Detail  [f] Assignee  [s] Status  [r] Refresh  [n/N] Next/Prev Match  [Esc] Back  [q] Quit"
    } else {
        " [j/k] Move  [Enter] Detail  [f] Assignee  [s] Status  [r] Refresh  [/] Search  [Esc] Back  [q] Quit"
    };
    let paragraph = Paragraph::new(text).style(Style::default().fg(Color::DarkGray));
    frame.render_widget(paragraph, area);
}

fn render_status_message(frame: &mut Frame, area: Rect, state: &AppState) {
    if let Some(msg) = &state.status_message {
        let status_area = Rect {
            y: area.height.saturating_sub(1),
            height: 1,
            ..area
        };
        let paragraph = Paragraph::new(msg.as_str()).style(Style::default().fg(Color::Yellow));
        frame.render_widget(paragraph, status_area);
    }
}
