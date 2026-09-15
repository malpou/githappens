use anyhow::Result;
use clap::Parser;

mod config;
mod log;

fn main() -> Result<()> {
    let cfg = config::Config::parse();
    config::validate(&cfg)?;

    if let Some(ref token) = cfg.token {
        log::set_redaction_token(token);
    }

    let _guard = log::init("info").map_err(|e| anyhow::anyhow!("{e}"))?;

    tracing::info!("githappens starting");

    Ok(())
}
