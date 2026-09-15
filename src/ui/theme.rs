use ratatui::style::Color;

pub const GLYPH_READY: &str = "●";
pub const GLYPH_WAITING: &str = "◐";
pub const GLYPH_FAILED: &str = "●";
pub const GLYPH_APPROVED: &str = "●";
pub const GLYPH_PENDING: &str = "◔";
pub const GLYPH_NONE: &str = "○";
pub const GLYPH_CHANGES_REQUESTED: &str = "●";

pub const COLOR_READY: Color = Color::Green;
pub const COLOR_WAITING: Color = Color::Yellow;
pub const COLOR_FAILED: Color = Color::Red;
pub const COLOR_APPROVED: Color = Color::Green;
pub const COLOR_CHANGES_REQUESTED: Color = Color::Red;
pub const COLOR_PENDING: Color = Color::Yellow;
pub const COLOR_NONE: Color = Color::DarkGray;
pub const COLOR_HEADER: Color = Color::Cyan;
pub const COLOR_SELECTED: Color = Color::Black;
pub const COLOR_SELECTED_BG: Color = Color::White;

pub const HEADER_LABEL: &str = " githappens ";
pub const FOOTER_HINT: &str = " r refresh · q quit ";
pub const EMPTY_STATE_MSG: &str = "You have no open PRs. Go open one!";

pub const COLUMN_INDICATOR_WIDTH: usize = 2;
pub const COLUMN_NUMBER_WIDTH: usize = 6;
pub const COLUMN_CHECKS_WIDTH: usize = 8;
pub const COLUMN_REVIEW_WIDTH: usize = 10;
pub const COLUMN_UPTODATE_WIDTH: usize = 10;

pub fn merge_glyph_and_color(
    readiness: &crate::analysis::mergeability::MergeReadiness,
) -> (&'static str, Color) {
    match readiness {
        crate::analysis::mergeability::MergeReadiness::Ready => (GLYPH_READY, COLOR_READY),
        crate::analysis::mergeability::MergeReadiness::Waiting => (GLYPH_WAITING, COLOR_WAITING),
        crate::analysis::mergeability::MergeReadiness::Failed => (GLYPH_FAILED, COLOR_FAILED),
    }
}

pub fn approval_glyph_and_color(
    approval: &crate::analysis::approval::ApprovalState,
) -> (&'static str, Color) {
    match approval {
        crate::analysis::approval::ApprovalState::Approved => (GLYPH_APPROVED, COLOR_APPROVED),
        crate::analysis::approval::ApprovalState::ChangesRequested => {
            (GLYPH_CHANGES_REQUESTED, COLOR_CHANGES_REQUESTED)
        }
        crate::analysis::approval::ApprovalState::Pending => (GLYPH_PENDING, COLOR_PENDING),
        crate::analysis::approval::ApprovalState::None => (GLYPH_NONE, COLOR_NONE),
    }
}

pub fn up_to_date_glyph_and_color(
    state: &crate::github::pr::UpToDateState,
) -> (&'static str, Color) {
    use crate::github::pr::UpToDateState;
    match state {
        UpToDateState::UpToDate => ("●", Color::Green),
        UpToDateState::OutOfDate => ("●", Color::Red),
        UpToDateState::Unknown => ("○", Color::DarkGray),
    }
}
