use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Row, Table};

use chrono::{DateTime, Utc};

use crate::analysis::approval::{ApprovalState, collapse_reviews};
use crate::analysis::mergeability::{MergeReadiness, assess};
use crate::analysis::workflows::{WorkflowCounts, count_checks, render_counts};
use crate::app::App;
use crate::ui::theme;

pub fn render(frame: &mut ratatui::Frame, app: &mut App) {
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
        crate::app::AppState::Loading | crate::app::AppState::Refreshing if app.prs.is_empty() => {
            let spinner = app.spinner();
            let msg = format!(" {spinner}  Fetching your PRs... ");
            let paragraph = Paragraph::new(msg).centered();
            frame.render_widget(paragraph, area);
        }
        _ => {
            render_dashboard(frame, area, app);
        }
    }
}

fn render_dashboard(frame: &mut ratatui::Frame, area: Rect, app: &mut App) {
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .split(area);

    render_header(frame, chunks[0], &mut *app);
    render_table(frame, chunks[1], app);
    render_footer(frame, chunks[2], app);
}

fn render_header(frame: &mut ratatui::Frame, area: Rect, app: &mut App) {
    let title = if app.is_refreshing() {
        let spinner = app.spinner();
        format!(
            " {} {} — {}'s open PRs · refreshing... ",
            spinner,
            theme::HEADER_LABEL.trim(),
            app.viewer_login,
        )
    } else {
        format!(
            " {} — {}'s open PRs · refreshed {}s ago ",
            theme::HEADER_LABEL.trim(),
            app.viewer_login,
            app.last_refresh_secs()
        )
    };

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

    let header_cells = [
        "",
        "#",
        "Title",
        "Diff",
        "Checks",
        "Approval",
        "Up-to-date",
        "Age",
    ];
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
            let diff_spans = vec![
                Span::styled(
                    format!("+{}", pr.additions),
                    if pr.additions > 0 {
                        Style::default().fg(Color::Green)
                    } else {
                        Style::default()
                    },
                ),
                Span::raw("/"),
                Span::styled(
                    format!("-{}", pr.deletions),
                    if pr.deletions > 0 {
                        Style::default().fg(Color::Red)
                    } else {
                        Style::default()
                    },
                ),
            ];
            let age_str = format_age(&pr.created_at);

            let title = if pr.is_draft {
                format!("[Draft] {}", pr.title)
            } else {
                pr.title.clone()
            };

            Row::new([
                Line::from(glyph).style(Style::default().fg(color)),
                Line::from(pr.number.to_string()),
                Line::from(title),
                Line::from(diff_spans),
                Line::from(checks_str),
                Line::from(rev_glyph).style(Style::default().fg(rev_color)),
                Line::from(utd_glyph).style(Style::default().fg(utd_color)),
                Line::from(age_str),
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
            Constraint::Length(theme::COLUMN_DIFF_WIDTH as u16),
            Constraint::Length(theme::COLUMN_CHECKS_WIDTH as u16),
            Constraint::Length(theme::COLUMN_REVIEW_WIDTH as u16),
            Constraint::Length(theme::COLUMN_UPTODATE_WIDTH as u16),
            Constraint::Length(theme::COLUMN_AGE_WIDTH as u16),
        ],
    )
    .header(header);

    frame.render_widget(table, area);
}

fn format_age(created_at: &str) -> String {
    let parsed: DateTime<Utc> = match DateTime::parse_from_rfc3339(created_at) {
        Ok(dt) => dt.with_timezone(&Utc),
        Err(_) => return "?".to_string(),
    };
    let elapsed = Utc::now().signed_duration_since(parsed);
    let secs = elapsed.num_seconds();
    if secs < 60 {
        return format!("{secs}s");
    }
    let mins = secs / 60;
    if mins < 60 {
        return format!("{mins}m");
    }
    let hours = mins / 60;
    if hours < 24 {
        return format!("{hours}h");
    }
    let days = hours / 24;
    if days < 30 {
        return format!("{days}d");
    }
    let months = days / 30;
    if months < 12 {
        return format!("{months}mo");
    }
    let years = months / 12;
    format!("{years}y")
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

    let footer = format!(" {total} open PRs · {ready} ready · {failed} failed ");

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
    use chrono::{Duration, Utc};

    #[test]
    fn format_age_seconds() {
        let ts = (Utc::now() - Duration::seconds(30)).to_rfc3339();
        assert_eq!(format_age(&ts), "30s");
    }

    #[test]
    fn format_age_minutes() {
        let ts = (Utc::now() - Duration::minutes(5)).to_rfc3339();
        assert_eq!(format_age(&ts), "5m");
    }

    #[test]
    fn format_age_hours() {
        let ts = (Utc::now() - Duration::hours(3)).to_rfc3339();
        assert_eq!(format_age(&ts), "3h");
    }

    #[test]
    fn format_age_days() {
        let ts = (Utc::now() - Duration::days(7)).to_rfc3339();
        assert_eq!(format_age(&ts), "7d");
    }

    #[test]
    fn format_age_months() {
        let ts = (Utc::now() - Duration::days(60)).to_rfc3339();
        assert_eq!(format_age(&ts), "2mo");
    }

    #[test]
    fn format_age_just_under_a_year() {
        let ts = (Utc::now() - Duration::days(350)).to_rfc3339();
        assert_eq!(format_age(&ts), "11mo");
    }

    #[test]
    fn format_age_twelve_months_shows_year() {
        let ts = (Utc::now() - Duration::days(360)).to_rfc3339();
        assert_eq!(format_age(&ts), "1y");
    }

    #[test]
    fn format_age_one_year() {
        let ts = (Utc::now() - Duration::days(365)).to_rfc3339();
        assert_eq!(format_age(&ts), "1y");
    }

    #[test]
    fn format_age_multiple_years() {
        let ts = (Utc::now() - Duration::days(730)).to_rfc3339();
        assert_eq!(format_age(&ts), "2y");
    }

    #[test]
    fn format_age_invalid_returns_question() {
        assert_eq!(format_age("not a date"), "?");
    }

    #[test]
    fn format_age_empty_returns_question() {
        assert_eq!(format_age(""), "?");
    }

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
