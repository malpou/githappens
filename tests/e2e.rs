#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::print_stdout,
    clippy::print_stderr
)]

use githappens::github::client::{GitHubFetcher, HttpGitHubFetcher};

#[tokio::test]
async fn e2e_fetch_real_prs() {
    let Ok(token) = std::env::var("GIT_TOKEN") else {
        eprintln!("skipping e2e test: GIT_TOKEN not set");
        return;
    };
    let fetcher = HttpGitHubFetcher::new(token);
    let outcome = fetcher
        .fetch_open_prs(None, 100)
        .await
        .expect("fetch should succeed");

    assert!(!outcome.login.is_empty(), "viewer login should be set");
    println!("Viewer: {}", outcome.login);
    println!("PRs: {}", outcome.prs.len());
    for pr in &outcome.prs {
        println!(
            "  #{} {} [{}] checks={} reviews={}",
            pr.number,
            pr.title,
            pr.repo,
            pr.checks.len(),
            pr.reviews.len()
        );
    }
}
