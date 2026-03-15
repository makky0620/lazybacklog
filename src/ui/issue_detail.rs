use crate::api::models::Issue;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

pub fn render(frame: &mut Frame, area: Rect, issue: &Issue) {
    let popup_area = centered_rect(80, 80, area);
    frame.render_widget(Clear, popup_area);

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
        Line::from(vec![Span::styled(
            issue.summary.as_str(),
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Assignee: ", Style::default().fg(Color::Yellow)),
            Span::raw(assignee),
            Span::raw("    "),
            Span::styled("Priority: ", Style::default().fg(Color::Yellow)),
            Span::raw(priority),
        ]),
        Line::from(vec![
            Span::styled("Status: ", Style::default().fg(Color::Yellow)),
            Span::raw(issue.status.name.as_str()),
            Span::raw("    "),
            Span::styled("Type: ", Style::default().fg(Color::Yellow)),
            Span::raw(issue_type),
        ]),
        Line::from(vec![
            Span::styled("Due: ", Style::default().fg(Color::Yellow)),
            Span::raw(due_date),
        ]),
        Line::from(""),
        Line::from(Span::styled("Description:", Style::default().fg(Color::Yellow))),
        Line::from(""),
    ];

    for desc_line in description.lines() {
        lines.push(Line::from(desc_line.to_string()));
    }

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .title(format!(" {} ", issue.issue_key))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, popup_area);

    // Help text
    if popup_area.height > 2 {
        let help_area = Rect {
            x: popup_area.x + 1,
            y: popup_area.y + popup_area.height - 2,
            width: popup_area.width.saturating_sub(2),
            height: 1,
        };
        let help = Paragraph::new("[Esc] Close").style(Style::default().fg(Color::DarkGray));
        frame.render_widget(help, help_area);
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let width = (area.width as u32 * percent_x as u32 / 100) as u16;
    let height = (area.height as u32 * percent_y as u32 / 100) as u16;
    Rect {
        x: area.x + (area.width - width) / 2,
        y: area.y + (area.height - height) / 2,
        width,
        height,
    }
}
