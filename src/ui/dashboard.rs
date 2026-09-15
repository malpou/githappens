use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, Row, Table};

use crate::analysis::approval::{ApprovalState, collapse_reviews};
use crate::analysis::mergeability::{MergeReadiness, assess};
use crate::analysis::workflows::{WorkflowCounts, count_checks, render_counts};
use crate::app::App;
use crate::ui::theme;

pub fn render(frame: &mut ratatui::Frame, app: &App) {
    let area = frame.area();

    if app.help_visible {
        crate::ui::help_overlay::render(frame, area);
        return;
    }

    match &app.state {
        crate::app::AppState::Error(msg) => {
            crate::ui::error_screen::render(frame, area, msg);
        }
        crate::app::AppState::RateLimited { retry_after_secs } => {
            let countdown = app
                .rate_limit_countdown()
                .unwrap_or_else(|| format!("{}s", retry_after_secs));
            let msg = format!("Rate limited by GitHub. Retry in {countdown}");
            crate::ui::error_screen::render(frame, area, &msg);
        }
        _ => {
            render_dashboard(frame, area, app);
        }
    }
}

fn render_dashboard(frame: &mut ratatui::Frame, area: Rect, app: &App) {
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .split(area);

    render_header(frame, chunks[0], app);
    render_table(frame, chunks[1], app);
    render_footer(frame, chunks[2], app);
}

fn render_header(frame: &mut ratatui::Frame, area: Rect, app: &App) {
    let title = format!(
        " {} — {}'s open PRs · refreshed {}s ago ",
        theme::HEADER_LABEL.trim(),
        app.viewer_login,
        app.last_refresh_secs()
    );

    let header = Block::default()
        .borders(Borders::BOTTOM)
        .title(Line::from(title.as_str()));

    frame.render_widget(header, area);
}

fn render_table(frame: &mut ratatui::Frame, area: Rect, app: &App) {
    if app.prs.is_empty() {
        let msg = Paragraph::new(theme::EMPTY_STATE_MSG)
            .style(Style::default().add_modifier(Modifier::ITALIC))
            .centered();
        frame.render_widget(msg, area);
        return;
    }

    let header_cells = ["", "#", "Title", "Checks", "Approval", "Up-to-date"];
    let header = Row::new(header_cells).style(Style::default().fg(theme::COLOR_HEADER));

    let rows: Vec<Row> = app
        .prs
        .iter()
        .enumerate()
        .map(|(i, pr)| {
            let readiness = assess(pr);
            let (glyph, color) = theme::merge_glyph_and_color(&readiness);
            let counts = count_checks(&pr.checks);
            let checks_str = render_counts(&counts);
            let approval = collapse_reviews(&pr.reviews);
            let (rev_glyph, rev_color) = theme::approval_glyph_and_color(&approval);
            let (utd_glyph, utd_color) = theme::up_to_date_glyph_and_color(&pr.up_to_date);

            let title = if pr.is_draft {
                format!("[Draft] {}", pr.title)
            } else {
                pr.title.clone()
            };

            Row::new([
                Line::from(glyph).style(Style::default().fg(color)),
                Line::from(pr.number.to_string()),
                Line::from(title),
                Line::from(checks_str),
                Line::from(rev_glyph).style(Style::default().fg(rev_color)),
                Line::from(utd_glyph).style(Style::default().fg(utd_color)),
            ])
            .style(if i == app.selected {
                Style::default()
                    .bg(theme::COLOR_SELECTED_BG)
                    .fg(theme::COLOR_SELECTED)
            } else {
                Style::default()
            })
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(theme::COLUMN_INDICATOR_WIDTH as u16),
            Constraint::Length(theme::COLUMN_NUMBER_WIDTH as u16),
            Constraint::Min(1),
            Constraint::Length(theme::COLUMN_CHECKS_WIDTH as u16),
            Constraint::Length(theme::COLUMN_REVIEW_WIDTH as u16),
            Constraint::Length(theme::COLUMN_UPTODATE_WIDTH as u16),
        ],
    )
    .header(header);

    frame.render_widget(table, area);
}

fn render_footer(frame: &mut ratatui::Frame, area: Rect, app: &App) {
    let total = app.prs.len();
    let ready = app
        .prs
        .iter()
        .filter(|p| assess(p) == MergeReadiness::Ready)
        .count();
    let failed = app
        .prs
        .iter()
        .filter(|p| assess(p) == MergeReadiness::Failed)
        .count();

    let rate_limited = match &app.state {
        crate::app::AppState::RateLimited { .. } => 1,
        _ => 0,
    };

    let footer = format!(
        " {} open PRs · {} ready · {} failed · {} rate-limited ",
        total, ready, failed, rate_limited
    );

    let paragraph = Paragraph::new(footer).style(
        Style::default()
            .add_modifier(Modifier::BOLD)
            .fg(theme::COLOR_HEADER),
    );

    frame.render_widget(paragraph, area);
}

pub fn render_counts_string(counts: &WorkflowCounts) -> String {
    render_counts(counts)
}

pub fn assess_readiness(pr: &crate::github::pr::PullRequestSnapshot) -> MergeReadiness {
    assess(pr)
}

pub fn get_approval(pr: &crate::github::pr::PullRequestSnapshot) -> ApprovalState {
    collapse_reviews(&pr.reviews)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::analysis::workflows::WorkflowCounts;

    #[test]
    fn render_counts_zero() {
        let counts = WorkflowCounts {
            completed: 0,
            total: 0,
        };
        assert_eq!(render_counts_string(&counts), "–/–");
    }

    #[test]
    fn render_counts_normal() {
        let counts = WorkflowCounts {
            completed: 5,
            total: 7,
        };
        assert_eq!(render_counts_string(&counts), "5/7");
    }

    #[test]
    fn render_counts_capped() {
        let counts = WorkflowCounts {
            completed: 100,
            total: 100,
        };
        assert_eq!(render_counts_string(&counts), "99+/99+");
    }
}
