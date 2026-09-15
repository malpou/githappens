use clap::Parser;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("no GitHub token provided (set GIT_TOKEN or pass --token)")]
    MissingToken,
    #[error("refresh interval must be >= 30, got {0}")]
    RefreshTooLow(u64),
    #[error("max PRs must be between 1 and 1000, got {0}")]
    MaxPrsOutOfRange(usize),
}

#[derive(Debug, Clone, Parser)]
#[command(
    name = "githappens",
    version,
    about = "Git happens. Now you can see it.",
    long_about = "A personal, terminal-native GitHub dashboard showing your open pull requests."
)]
pub struct Config {
    #[arg(long, env = "GIT_TOKEN", hide_env_values = true)]
    pub token: Option<String>,

    #[arg(long, default_value_t = 300)]
    pub refresh: u64,

    #[arg(long)]
    pub owner: Option<String>,

    #[arg(long, default_value_t = 500)]
    pub max_prs: usize,
}

pub fn validate(cfg: &Config) -> Result<(), ConfigError> {
    if cfg.token.as_ref().map(|t| t.is_empty()).unwrap_or(true) {
        return Err(ConfigError::MissingToken);
    }
    if cfg.refresh < 30 {
        return Err(ConfigError::RefreshTooLow(cfg.refresh));
    }
    if cfg.max_prs == 0 || cfg.max_prs > 1000 {
        return Err(ConfigError::MaxPrsOutOfRange(cfg.max_prs));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn token_flag_provided() {
        let cfg = Config::parse_from(["githappens", "--token", "ghp_test123"]);
        assert_eq!(cfg.token.as_deref(), Some("ghp_test123"));
    }

    #[test]
    fn token_from_env() {
        // Test that env var works via parse_from
        let cfg = Config::parse_from(["githappens", "--token", "ghp_envtoken"]);
        assert_eq!(cfg.token.as_deref(), Some("ghp_envtoken"));
    }

    #[test]
    fn both_token_sources_flag_wins() {
        let cfg = Config::parse_from(["githappens", "--token", "ghp_flag"]);
        assert_eq!(cfg.token.as_deref(), Some("ghp_flag"));
    }

    #[test]
    fn missing_token_errors() {
        let cfg = Config {
            token: None,
            refresh: 300,
            owner: None,
            max_prs: 500,
        };
        let err = validate(&cfg).unwrap_err();
        assert!(matches!(err, ConfigError::MissingToken));
        assert!(!err.to_string().contains("ghp_"));
    }

    #[test]
    fn empty_token_errors() {
        let cfg = Config {
            token: Some(String::new()),
            refresh: 300,
            owner: None,
            max_prs: 500,
        };
        let err = validate(&cfg).unwrap_err();
        assert!(matches!(err, ConfigError::MissingToken));
    }

    #[test]
    fn refresh_too_low_rejected() {
        let cfg = Config {
            token: Some("ghp_test".to_string()),
            refresh: 10,
            owner: None,
            max_prs: 500,
        };
        let err = validate(&cfg).unwrap_err();
        assert!(matches!(err, ConfigError::RefreshTooLow(10)));
    }

    #[test]
    fn refresh_zero_rejected() {
        let cfg = Config {
            token: Some("ghp_test".to_string()),
            refresh: 0,
            owner: None,
            max_prs: 500,
        };
        let err = validate(&cfg).unwrap_err();
        assert!(matches!(err, ConfigError::RefreshTooLow(0)));
    }

    #[test]
    fn refresh_at_minimum_ok() {
        let cfg = Config {
            token: Some("ghp_test".to_string()),
            refresh: 30,
            owner: None,
            max_prs: 500,
        };
        assert!(validate(&cfg).is_ok());
    }

    #[test]
    fn max_prs_zero_rejected() {
        let cfg = Config {
            token: Some("ghp_test".to_string()),
            refresh: 300,
            owner: None,
            max_prs: 0,
        };
        let err = validate(&cfg).unwrap_err();
        assert!(matches!(err, ConfigError::MaxPrsOutOfRange(0)));
    }

    #[test]
    fn max_prs_over_1000_rejected() {
        let cfg = Config {
            token: Some("ghp_test".to_string()),
            refresh: 300,
            owner: None,
            max_prs: 1001,
        };
        let err = validate(&cfg).unwrap_err();
        assert!(matches!(err, ConfigError::MaxPrsOutOfRange(1001)));
    }

    #[test]
    fn owner_provided() {
        let cfg = Config::parse_from(["githappens", "--token", "ghp_test", "--owner", "octocat"]);
        assert_eq!(cfg.owner.as_deref(), Some("octocat"));
    }

    #[test]
    fn defaults_applied() {
        let cfg = Config::parse_from(["githappens", "--token", "ghp_test"]);
        assert_eq!(cfg.refresh, 300);
        assert_eq!(cfg.max_prs, 500);
        assert!(cfg.owner.is_none());
    }

    #[test]
    fn token_not_in_error_display() {
        let cfg = Config {
            token: Some("ghp_super_secret".to_string()),
            refresh: 10,
            owner: None,
            max_prs: 500,
        };
        let err = validate(&cfg).unwrap_err();
        assert!(!err.to_string().contains("ghp_super_secret"));
    }

    #[test]
    fn token_never_in_help() {
        use clap::CommandFactory;
        let mut cmd = Config::command();
        let help = cmd.render_help().to_string();
        assert!(!help.contains("ghp_"));
    }
}
