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
    fn open_valid_url() {
        let result = open("https://github.com");
        assert!(result.is_ok());
    }

    #[test]
    fn open_invalid_url_errors() {
        let result = open("not a url at all");
        assert!(result.is_err());
    }
}
