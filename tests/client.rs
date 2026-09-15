#![allow(clippy::unwrap_used, clippy::expect_used)]

use githappens::github::client::{FetchError, GitHubFetcher, HttpGitHubFetcher};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn setup_mock_server() -> MockServer {
    MockServer::start().await
}

const ONE_PR_FIXTURE: &str = include_str!("fixtures/one_open_pr.json");
const PAGE1_FIXTURE: &str = include_str!("fixtures/many_prs_page1.json");
const PAGE2_FIXTURE: &str = include_str!("fixtures/many_prs_page2.json");

#[tokio::test]
async fn test_single_page_returns_all_prs() {
    let server = setup_mock_server().await;
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .respond_with(ResponseTemplate::new(200).set_body_string(ONE_PR_FIXTURE))
        .mount(&server)
        .await;

    let client = build_client(&server);
    let outcome = client.fetch_open_prs(None, 100).await.unwrap();
    assert_eq!(outcome.prs.len(), 1);
    assert_eq!(outcome.prs[0].number, 42);
    assert!(!outcome.truncated);
}

#[tokio::test]
async fn test_pagination_concat_and_dedup() {
    let server = setup_mock_server().await;

    Mock::given(method("POST"))
        .and(path("/graphql"))
        .and(header("authorization", "Bearer ghp_test_token"))
        .respond_with(ResponseTemplate::new(200).set_body_string(PAGE1_FIXTURE))
        .up_to_n_times(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/graphql"))
        .and(header("authorization", "Bearer ghp_test_token"))
        .respond_with(ResponseTemplate::new(200).set_body_string(PAGE2_FIXTURE))
        .mount(&server)
        .await;

    let client = build_client(&server);
    let outcome = client.fetch_open_prs(None, 500).await.unwrap();
    assert_eq!(outcome.prs.len(), 3);
    let numbers: Vec<u32> = outcome.prs.iter().map(|p| p.number).collect();
    assert!(numbers.contains(&100));
    assert!(numbers.contains(&99));
    assert!(numbers.contains(&98));
    let count_100 = outcome.prs.iter().filter(|p| p.number == 100).count();
    assert_eq!(count_100, 1);
}

#[tokio::test]
async fn test_max_prs_stops_gracefully() {
    let server = setup_mock_server().await;

    Mock::given(method("POST"))
        .and(path("/graphql"))
        .respond_with(ResponseTemplate::new(200).set_body_string(PAGE1_FIXTURE))
        .mount(&server)
        .await;

    let client = build_client(&server);
    let outcome = client.fetch_open_prs(None, 2).await.unwrap();
    assert_eq!(outcome.prs.len(), 2);
}

#[tokio::test]
async fn test_http_401_token_invalid() {
    let server = setup_mock_server().await;
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&server)
        .await;

    let client = build_client(&server);
    let err = client.fetch_open_prs(None, 100).await.unwrap_err();
    assert!(matches!(err, FetchError::TokenInvalid));
    assert!(!err.to_string().contains("ghp_test_token"));
}

#[tokio::test]
async fn test_rate_limited_surfaces_retry_after() {
    let server = setup_mock_server().await;
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .respond_with(
            ResponseTemplate::new(403)
                .insert_header("x-ratelimit-remaining", "0")
                .insert_header("retry-after", "120"),
        )
        .mount(&server)
        .await;

    let client = build_client(&server);
    let err = client.fetch_open_prs(None, 100).await.unwrap_err();
    match err {
        FetchError::RateLimited { retry_after_secs } => {
            assert_eq!(retry_after_secs, 120);
        }
        _ => panic!("expected RateLimited, got {err:?}"),
    }
}

#[tokio::test]
async fn test_503_retry_then_success() {
    let server = setup_mock_server().await;

    Mock::given(method("POST"))
        .and(path("/graphql"))
        .respond_with(ResponseTemplate::new(503))
        .up_to_n_times(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/graphql"))
        .respond_with(ResponseTemplate::new(200).set_body_string(ONE_PR_FIXTURE))
        .mount(&server)
        .await;

    let client = build_client(&server);
    let outcome = client.fetch_open_prs(None, 100).await.unwrap();
    assert_eq!(outcome.prs.len(), 1);
}

#[tokio::test]
async fn test_503_twice_github_unavailable() {
    let server = setup_mock_server().await;
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&server)
        .await;

    let client = build_client(&server);
    let err = client.fetch_open_prs(None, 100).await.unwrap_err();
    assert!(matches!(err, FetchError::GitHubUnavailable));
}

#[tokio::test]
async fn test_malformed_json_parse_error() {
    let server = setup_mock_server().await;
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .respond_with(ResponseTemplate::new(200).set_body_string("not json at all"))
        .mount(&server)
        .await;

    let client = build_client(&server);
    let err = client.fetch_open_prs(None, 100).await.unwrap_err();
    assert!(matches!(err, FetchError::Parse(_)));
}

#[tokio::test]
async fn test_token_in_header_not_in_url() {
    let server = setup_mock_server().await;
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .and(header("authorization", "Bearer ghp_test_token"))
        .respond_with(ResponseTemplate::new(200).set_body_string(ONE_PR_FIXTURE))
        .mount(&server)
        .await;

    let client = build_client(&server);
    let _ = client.fetch_open_prs(None, 100).await.unwrap();
}

#[tokio::test]
async fn test_token_not_in_error_display() {
    let server = setup_mock_server().await;
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&server)
        .await;

    let client = build_client(&server);
    let err = client.fetch_open_prs(None, 100).await.unwrap_err();
    let err_string = err.to_string();
    assert!(
        !err_string.contains("ghp_test_token"),
        "token found in error: {err_string}"
    );
}

fn build_client(server: &MockServer) -> HttpGitHubFetcher {
    let url = server.uri() + "/graphql";
    HttpGitHubFetcher::new_with_endpoint("ghp_test_token".to_string(), url)
}
