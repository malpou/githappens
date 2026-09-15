use anyhow::Result;
use clap::Parser;

mod config;

fn main() -> Result<()> {
    let cfg = config::Config::parse();
    config::validate(&cfg)?;
    Ok(())
}
