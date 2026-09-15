pub fn open(url: &str) -> Result<(), BrowserError> {
    webbrowser::open(url).map_err(|e| BrowserError::OpenFailed(e.to_string()))
}

#[derive(Debug, thiserror::Error)]
pub enum BrowserError {
    #[error("failed to open browser: {0}")]
    OpenFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "environment-dependent: webbrowser may succeed even for invalid URLs on some platforms"]
    fn open_invalid_url_errors() {
        let result = open("not a url at all");
        assert!(result.is_err());
    }
}
