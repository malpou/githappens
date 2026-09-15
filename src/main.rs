use std::io::{self, stdout};
use std::time::Duration;

use anyhow::Result;
use clap::Parser;
use crossterm::ExecutableCommand;
use crossterm::event::{KeyCode, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use githappens::app::{App, AppState, KeyAction};
use githappens::config;
use githappens::github::client::HttpGitHubFetcher;
use githappens::log;
use githappens::ui;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

#[tokio::main]
async fn main() -> Result<()> {
    let cfg = config::Config::parse();
    config::validate(&cfg)?;

    if let Some(ref token) = cfg.token {
        log::set_redaction_token(token);
    }

    let _guard = log::init("info").map_err(|e| anyhow::anyhow!("{e}"))?;

    tracing::info!("githappens starting");

    let token = cfg.token.clone().unwrap_or_default();
    let fetcher = HttpGitHubFetcher::new(token);
    let refresh_interval = cfg.refresh;
    let max_prs = cfg.max_prs;
    let owner = cfg.owner.clone();

    run_tui(fetcher, refresh_interval, max_prs, owner).await
}

async fn run_tui(
    fetcher: HttpGitHubFetcher,
    refresh_interval: u64,
    max_prs: usize,
    owner: Option<String>,
) -> Result<()> {
    setup_terminal()?;
    let result = run_app(fetcher, refresh_interval, max_prs, owner).await;
    restore_terminal();
    result
}

fn setup_terminal() -> Result<()> {
    enable_raw_mode()?;
    stdout().execute(crossterm::terminal::EnterAlternateScreen)?;
    Ok(())
}

fn restore_terminal() {
    let _ = disable_raw_mode();
    let _ = stdout().execute(crossterm::terminal::LeaveAlternateScreen);
}

async fn run_app(
    fetcher: HttpGitHubFetcher,
    refresh_interval: u64,
    max_prs: usize,
    owner: Option<String>,
) -> Result<()> {
    let cfg = config::Config {
        token: Some(String::new()),
        refresh: refresh_interval,
        owner: owner.clone(),
        max_prs,
    };
    let mut app = App::new(&cfg).with_refresh_interval(refresh_interval);

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    app.refresh(&fetcher, owner.as_deref(), max_prs).await;

    let tick_interval = Duration::from_millis(250);

    loop {
        terminal.draw(|frame| ui::render(frame, &app))?;

        if let Some(event) = githappens::event::read_event(tick_interval) {
            match event {
                githappens::event::Event::Key(key) => {
                    if key.code == KeyCode::Char('c') && key.modifiers == KeyModifiers::CONTROL {
                        restore_terminal();
                        std::process::exit(130);
                    }
                    let action = app.handle_key(key);
                    match action {
                        KeyAction::Quit => break,
                        KeyAction::Refresh => {
                            if app.can_refresh() {
                                app.refresh(&fetcher, owner.as_deref(), max_prs).await;
                            }
                        }
                        KeyAction::ForceRefresh => {
                            app.state = AppState::Refreshing;
                            app.refresh(&fetcher, owner.as_deref(), max_prs).await;
                        }
                        KeyAction::OpenUrl(url) => {
                            let _ = githappens::browser::open(&url);
                        }
                        KeyAction::None => {}
                    }
                }
                githappens::event::Event::Tick => {
                    if app.should_auto_refresh() && app.can_refresh() {
                        app.refresh(&fetcher, owner.as_deref(), max_prs).await;
                    }
                }
            }
        }
    }

    restore_terminal();
    Ok(())
}
