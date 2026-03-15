use ratatui::{
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    widgets::{Cell, Paragraph, Row, Table, TableState},
    Frame,
};

use crate::app::AppState;

pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let space_state = state.current_space_state();

    if space_state.loading_issues {
        let loading = Paragraph::new("Loading issues...").style(Style::default().fg(Color::Gray));
        frame.render_widget(loading, area);
        return;
    }

    let issues = match &space_state.issues {
        Some(issues) => issues,
        None => {
            let msg = Paragraph::new("No issues loaded. Press [r] to fetch.")
                .style(Style::default().fg(Color::Gray));
            frame.render_widget(msg, area);
            return;
        }
    };

    // Filter issues by search query (rendering only — full-list indices preserved)
    let display_indices: Vec<usize> = if state.search_query.is_empty() {
        (0..issues.len()).collect()
    } else {
        state.matching_issue_indices()
    };

    let rows: Vec<Row> = display_indices
        .iter()
        .map(|&i| {
            let issue = &issues[i];
            let assignee = issue
                .assignee
                .as_ref()
                .map(|u| u.name.as_str())
                .unwrap_or("-");
            Row::new(vec![
                Cell::from(issue.issue_key.clone()),
                Cell::from(issue.summary.clone()),
                Cell::from(assignee.to_string()),
                Cell::from(issue.status.name.clone()),
            ])
        })
        .collect();

    // Position of selected_issue_idx within the displayed (filtered) rows
    let display_selected = display_indices
        .iter()
        .position(|&i| i == state.selected_issue_idx);

    // Footer: search bar or issue count
    let footer_text = if state.search_active || !state.search_query.is_empty() {
        let cursor = if state.search_active { "█" } else { "" };
        format!(
            "/ {}{}  ({} matches)",
            state.search_query,
            cursor,
            display_indices.len()
        )
    } else if issues.len() >= 100 {
        format!("(showing: {} / limit 100)", issues.len())
    } else {
        format!("({})", issues.len())
    };

    // Reserve last line for footer
    let table_area = Rect {
        height: area.height.saturating_sub(1),
        ..area
    };
    let footer_area = Rect {
        y: area.y + area.height.saturating_sub(1),
        height: 1,
        ..area
    };

    let widths = [
        Constraint::Length(12),
        Constraint::Min(30),
        Constraint::Length(16),
        Constraint::Length(14),
    ];

    let table = Table::new(rows, widths)
        .header(
            Row::new(vec!["Key", "Summary", "Assignee", "Status"])
                .style(Style::default().fg(Color::Yellow)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    let mut table_state = TableState::default();
    if let Some(pos) = display_selected {
        table_state.select(Some(pos));
    }

    frame.render_stateful_widget(table, table_area, &mut table_state);

    let footer_style = if state.search_active {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let footer = Paragraph::new(footer_text).style(footer_style);
    frame.render_widget(footer, footer_area);
}
