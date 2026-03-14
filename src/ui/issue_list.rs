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
        let loading =
            Paragraph::new("Loading issues...").style(Style::default().fg(Color::Gray));
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

    let rows: Vec<Row> = issues
        .iter()
        .map(|issue| {
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

    let footer_msg = if issues.len() >= 100 {
        format!("(表示: {}件 / 上限100件)", issues.len())
    } else {
        format!("({}件)", issues.len())
    };

    // Reserve last line for count
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
    if !issues.is_empty() {
        table_state.select(Some(state.selected_issue_idx));
    }

    frame.render_stateful_widget(table, table_area, &mut table_state);

    let footer = Paragraph::new(footer_msg).style(Style::default().fg(Color::DarkGray));
    frame.render_widget(footer, footer_area);
}
