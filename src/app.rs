use std::time::{Duration, Instant};

use crate::analysis::mergeability::{MergeReadiness, assess};
use crate::config::Config;
use crate::github::client::{FetchError, FetchOutcome, GitHubFetcher};
use crate::github::pr::PullRequestSnapshot;

#[derive(Debug, Clone, PartialEq)]
pub enum AppState {
    Loading,
    Ready,
    Refreshing,
    Error(String),
    RateLimited { retry_after_secs: u64 },
    Help,
}

#[derive(Debug, Clone, PartialEq)]
pub enum KeyAction {
    Quit,
    Refresh,
    ForceRefresh,
    OpenUrl(String),
    None,
}

pub struct App {
    pub state: AppState,
    pub prs: Vec<PullRequestSnapshot>,
    pub selected: usize,
    pub viewer_login: String,
    pub help_visible: bool,
    pub last_refresh: Option<Instant>,
    pub truncated: bool,
    refresh_interval: Duration,
}

impl App {
    pub fn new(_config: &Config) -> Self {
        Self {
            state: AppState::Loading,
            prs: Vec::new(),
            selected: 0,
            viewer_login: String::new(),
            help_visible: false,
            last_refresh: None,
            truncated: false,
            refresh_interval: Duration::from_secs(300),
        }
    }

    pub fn with_refresh_interval(mut self, secs: u64) -> Self {
        self.refresh_interval = Duration::from_secs(secs);
        self
    }

    pub fn last_refresh_secs(&self) -> u64 {
        self.last_refresh
            .map(|t| t.elapsed().as_secs())
            .unwrap_or(0)
    }

    pub fn should_auto_refresh(&self) -> bool {
        if self.state != AppState::Ready {
            return false;
        }
        self.last_refresh
            .map(|t| t.elapsed() >= self.refresh_interval)
            .unwrap_or(true)
    }

    pub fn select_down(&mut self) {
        if !self.prs.is_empty() {
            self.selected = (self.selected + 1).min(self.prs.len() - 1);
        }
    }

    pub fn select_up(&mut self) {
        if !self.prs.is_empty() {
            self.selected = self.selected.saturating_sub(1);
        }
    }

    pub fn select_top(&mut self) {
        self.selected = 0;
    }

    pub fn select_bottom(&mut self) {
        if !self.prs.is_empty() {
            self.selected = self.prs.len() - 1;
        }
    }

    pub fn toggle_help(&mut self) {
        self.help_visible = !self.help_visible;
    }

    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> KeyAction {
        use crossterm::event::KeyCode;

        if self.help_visible {
            if key.code == KeyCode::Char('?') {
                self.toggle_help();
            }
            return KeyAction::None;
        }

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => KeyAction::Quit,
            KeyCode::Char('j') | KeyCode::Down => {
                self.select_down();
                KeyAction::None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.select_up();
                KeyAction::None
            }
            KeyCode::Char('g') => {
                self.select_top();
                KeyAction::None
            }
            KeyCode::Char('G') => {
                self.select_bottom();
                KeyAction::None
            }
            KeyCode::Enter => {
                if let Some(pr) = self.selected_pr() {
                    KeyAction::OpenUrl(pr.url.clone())
                } else {
                    KeyAction::None
                }
            }
            KeyCode::Char('r') => KeyAction::Refresh,
            KeyCode::Char('R') => KeyAction::ForceRefresh,
            KeyCode::Char('?') => {
                self.toggle_help();
                KeyAction::None
            }
            _ => KeyAction::None,
        }
    }

    pub fn selected_pr(&self) -> Option<&PullRequestSnapshot> {
        self.prs.get(self.selected)
    }

    pub async fn refresh<F: GitHubFetcher>(
        &mut self,
        fetcher: &F,
        owner: Option<&str>,
        max: usize,
    ) {
        self.state = AppState::Refreshing;
        match fetcher.fetch_open_prs(owner, max).await {
            Ok(outcome) => {
                self.apply_outcome(outcome);
            }
            Err(e) => {
                self.state = match e {
                    FetchError::RateLimited { retry_after_secs } => {
                        AppState::RateLimited { retry_after_secs }
                    }
                    FetchError::TokenInvalid => {
                        AppState::Error("Token invalid or expired".to_string())
                    }
                    FetchError::GitHubUnavailable => {
                        AppState::Error("GitHub unavailable, try again".to_string())
                    }
                    FetchError::Timeout => AppState::Error("Request timed out".to_string()),
                    _ => AppState::Error(e.to_string()),
                };
            }
        }
    }

    fn apply_outcome(&mut self, outcome: FetchOutcome) {
        self.prs = outcome.prs;
        self.viewer_login = outcome.login;
        self.truncated = outcome.truncated;
        self.last_refresh = Some(Instant::now());
        self.state = AppState::Ready;
        if self.selected >= self.prs.len() && !self.prs.is_empty() {
            self.selected = self.prs.len() - 1;
        }
    }

    pub fn ready_count(&self) -> usize {
        self.prs
            .iter()
            .filter(|p| assess(p) == MergeReadiness::Ready)
            .count()
    }

    pub fn failed_count(&self) -> usize {
        self.prs
            .iter()
            .filter(|p| assess(p) == MergeReadiness::Failed)
            .count()
    }

    pub fn is_rate_limited(&self) -> bool {
        matches!(self.state, AppState::RateLimited { .. })
    }

    pub fn rate_limit_retry_secs(&self) -> Option<u64> {
        match &self.state {
            AppState::RateLimited { retry_after_secs } => Some(*retry_after_secs),
            _ => None,
        }
    }

    pub fn can_refresh(&self) -> bool {
        !matches!(self.state, AppState::Refreshing)
    }

    pub fn rate_limit_countdown(&self) -> Option<String> {
        match &self.state {
            AppState::RateLimited { retry_after_secs } => {
                let elapsed = self
                    .last_refresh
                    .map(|t| t.elapsed().as_secs())
                    .unwrap_or(0);
                let remaining = retry_after_secs.saturating_sub(elapsed);
                if remaining >= 60 {
                    Some(format!("{}m", remaining / 60))
                } else {
                    Some(format!("{}s", remaining))
                }
            }
            _ => None,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::github::client::MockGitHubFetcher;

    fn make_config() -> Config {
        Config {
            token: Some("ghp_test".to_string()),
            refresh: 300,
            owner: None,
            max_prs: 500,
        }
    }

    #[tokio::test]
    async fn test_loading_to_ready() {
        let cfg = make_config();
        let mut app = App::new(&cfg);
        let fetcher = MockGitHubFetcher {
            outcome: Ok(FetchOutcome {
                login: "ska".to_string(),
                prs: vec![],
                truncated: false,
            }),
        };
        app.refresh(&fetcher, None, 500).await;
        assert_eq!(app.state, AppState::Ready);
        assert_eq!(app.viewer_login, "ska");
    }

    #[tokio::test]
    async fn test_error_state_on_token_invalid() {
        let cfg = make_config();
        let mut app = App::new(&cfg);
        let fetcher = MockGitHubFetcher {
            outcome: Err(FetchError::TokenInvalid),
        };
        app.refresh(&fetcher, None, 500).await;
        match &app.state {
            AppState::Error(msg) => assert!(msg.contains("Token invalid")),
            _ => panic!("expected Error state"),
        }
    }

    #[tokio::test]
    async fn test_rate_limited_state() {
        let cfg = make_config();
        let mut app = App::new(&cfg);
        let fetcher = MockGitHubFetcher {
            outcome: Err(FetchError::RateLimited {
                retry_after_secs: 120,
            }),
        };
        app.refresh(&fetcher, None, 500).await;
        match &app.state {
            AppState::RateLimited { retry_after_secs } => {
                assert_eq!(*retry_after_secs, 120);
            }
            _ => panic!("expected RateLimited state"),
        }
    }

    #[test]
    fn test_navigation() {
        let cfg = make_config();
        let mut app = App::new(&cfg);
        app.prs = vec![
            PullRequestSnapshot {
                number: 1,
                title: "PR 1".to_string(),
                url: "https://github.com/o/r/pull/1".to_string(),
                is_draft: false,
                mergeable: crate::github::models::MergeableState::Mergeable,
                repo: "o/r".to_string(),
                rollup_state: None,
                checks: vec![],
                reviews: vec![],
            },
            PullRequestSnapshot {
                number: 2,
                title: "PR 2".to_string(),
                url: "https://github.com/o/r/pull/2".to_string(),
                is_draft: false,
                mergeable: crate::github::models::MergeableState::Mergeable,
                repo: "o/r".to_string(),
                rollup_state: None,
                checks: vec![],
                reviews: vec![],
            },
        ];
        app.select_down();
        assert_eq!(app.selected, 1);
        app.select_down();
        assert_eq!(app.selected, 1);
        app.select_up();
        assert_eq!(app.selected, 0);
        app.select_up();
        assert_eq!(app.selected, 0);
        app.select_bottom();
        assert_eq!(app.selected, 1);
        app.select_top();
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn test_empty_navigation_no_panic() {
        let cfg = make_config();
        let mut app = App::new(&cfg);
        app.select_down();
        assert_eq!(app.selected, 0);
        app.select_up();
        assert_eq!(app.selected, 0);
        app.select_bottom();
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn test_help_toggle() {
        let cfg = make_config();
        let mut app = App::new(&cfg);
        assert!(!app.help_visible);
        app.toggle_help();
        assert!(app.help_visible);
        app.toggle_help();
        assert!(!app.help_visible);
    }
}
