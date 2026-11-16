use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem},
    Frame,
};

use crate::app::{App, Pane, ScopeItem};
use crate::checklist::checklist_progress;
use crate::format::date_short;

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.focused_pane == Pane::TodoLists;
    let border_color = if is_focused {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    let title_style = if is_focused {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    };

    let scope_name = match app.get_sidebar_item(app.sidebar_selected) {
        ScopeItem::All => "All".to_string(),
        ScopeItem::Pinned => "★ Pinned".to_string(),
        ScopeItem::Standalone => "Standalone".to_string(),
        ScopeItem::Project(p) => p.name,
    };

    let title_text = format!(" [2] Todo Lists ({} · {}) ", scope_name, app.todo_lists.len());

    let mut items = Vec::new();

    if app.todo_lists.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled(
            "  (no todo lists found)",
            Style::default().fg(Color::DarkGray),
        ))));
    } else {
        for (idx, tl) in app.todo_lists.iter().enumerate() {
            let is_selected = app.todo_lists_selected == idx;
            let cursor = if is_selected { "▶ " } else { "  " };

            let star = if tl.pinned { "★ " } else { "  " };
            let star_style = if tl.pinned {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            let title_style = if is_selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            // Checklist progress
            let (done, total) = checklist_progress(tl.body.as_deref().unwrap_or(""));
            let progress_span = if total > 0 {
                let color = if done == total {
                    Color::Green
                } else if done > 0 {
                    Color::LightYellow
                } else {
                    Color::DarkGray
                };
                Span::styled(format!(" [{}/{}]", done, total), Style::default().fg(color))
            } else {
                Span::raw("")
            };

            // Attachments indicator
            let att_span = if tl.attachments_count > 0 {
                Span::styled(
                    format!(" 📎{}", tl.attachments_count),
                    Style::default().fg(Color::Blue),
                )
            } else {
                Span::raw("")
            };

            // Project badge
            let proj_span = if let Some(pname) = &tl.project_name {
                Span::styled(
                    format!(" · {}", pname),
                    Style::default().fg(Color::Magenta),
                )
            } else {
                Span::raw("")
            };

            let date_str = tl
                .updated_at
                .as_deref()
                .map(date_short)
                .unwrap_or_default();
            let date_span = Span::styled(
                format!("  {}", date_str),
                Style::default().fg(Color::DarkGray),
            );

            let line = Line::from(vec![
                Span::styled(cursor, title_style),
                Span::styled(star, star_style),
                Span::styled(tl.title.clone(), title_style),
                progress_span,
                att_span,
                proj_span,
                date_span,
            ]);

            items.push(ListItem::new(line));
        }
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(title_text, title_style));

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}
