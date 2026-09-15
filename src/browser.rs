pub fn open(url: &str) -> Result<(), BrowserError> {
    webbrowser::open(url).map_err(|e| BrowserError::OpenFailed(e.to_string()))
}

#[derive(Debug, thiserror::Error)]
pub enum BrowserError {
    #[error("failed to open browser: {0}")]
    OpenFailed(String),
}
