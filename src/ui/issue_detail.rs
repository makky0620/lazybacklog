use crate::api::models::Issue;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub fn render(frame: &mut Frame, area: Rect, issue: &Issue, scroll_offset: u16) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // title bar
            Constraint::Min(0),    // content
            Constraint::Length(1), // help bar
        ])
        .split(area);

    // Title bar
    let title = format!(" {}  {}", issue.issue_key, issue.summary);
    let title_paragraph = Paragraph::new(title).style(
        Style::default()
            .bg(Color::Cyan)
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD),
    );
    frame.render_widget(title_paragraph, chunks[0]);

    // Content
    let assignee = issue
        .assignee
        .as_ref()
        .map(|u| u.name.as_str())
        .unwrap_or("-");
    let priority = issue
        .priority
        .as_ref()
        .map(|p| p.name.as_str())
        .unwrap_or("-");
    let issue_type = issue
        .issue_type
        .as_ref()
        .map(|t| t.name.as_str())
        .unwrap_or("-");
    let due_date = issue.due_date.as_deref().unwrap_or("-");
    let description = issue.description.as_deref().unwrap_or("(no description)");

    let mut lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("Assignee: ", Style::default().fg(Color::Cyan)),
            Span::raw(assignee),
            Span::raw("    "),
            Span::styled("Priority: ", Style::default().fg(Color::Cyan)),
            Span::raw(priority),
        ]),
        Line::from(vec![
            Span::styled("Status: ", Style::default().fg(Color::Cyan)),
            Span::raw(issue.status.name.as_str()),
            Span::raw("    "),
            Span::styled("Type: ", Style::default().fg(Color::Cyan)),
            Span::raw(issue_type),
        ]),
        Line::from(vec![
            Span::styled("Due: ", Style::default().fg(Color::Cyan)),
            Span::raw(due_date),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Description:",
            Style::default().fg(Color::Cyan),
        )),
        Line::from(""),
    ];

    for desc_line in description.lines() {
        lines.push(Line::from(desc_line.to_string()));
    }

    let content_paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .wrap(Wrap { trim: false })
        .scroll((scroll_offset, 0));
    frame.render_widget(content_paragraph, chunks[1]);

    // Help bar
    let help =
        Paragraph::new(" [j/k] Scroll  [Esc] Back").style(Style::default().fg(Color::DarkGray));
    frame.render_widget(help, chunks[2]);
}
