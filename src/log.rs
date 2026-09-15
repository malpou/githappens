use std::io::Write;
use std::sync::{Mutex, OnceLock};

use directories::ProjectDirs;
use thiserror::Error;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::prelude::*;

#[derive(Debug, Error)]
pub enum LogError {
    #[error("failed to create log directory: {0}")]
    CreateDir(String),
}

const TOKEN_REDACTED: &str = "[REDACTED]";

static REDACTION_TOKEN: OnceLock<Mutex<String>> = OnceLock::new();

fn token_store() -> &'static Mutex<String> {
    REDACTION_TOKEN.get_or_init(|| Mutex::new(String::new()))
}

pub fn set_redaction_token(token: &str) {
    let mut guard = token_store().lock().unwrap_or_else(|e| e.into_inner());
    *guard = token.to_string();
}

pub fn redact(text: &str) -> String {
    let token = token_store().lock().unwrap_or_else(|e| e.into_inner());
    if token.is_empty() {
        text.to_string()
    } else {
        text.replace(token.as_str(), TOKEN_REDACTED)
    }
}

struct RedactingWriter<W: Write> {
    inner: W,
}

impl<W: Write> Write for RedactingWriter<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let text = String::from_utf8_lossy(buf);
        let redacted = redact(&text);
        self.inner.write_all(redacted.as_bytes())?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

pub fn init(level: &str) -> Result<(WorkerGuard, String), LogError> {
    let project_dirs = ProjectDirs::from("com", "githappens", "githappens").ok_or(
        LogError::CreateDir("cannot determine log directory".to_string()),
    )?;

    let log_dir = project_dirs.data_local_dir().join("logs");
    std::fs::create_dir_all(&log_dir).map_err(|e| LogError::CreateDir(e.to_string()))?;

    let file_appender = tracing_appender::rolling::daily(&log_dir, "githappens.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let env_filter = EnvFilter::try_new(level).unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(
            tracing_subscriber::fmt::layer().with_writer(Mutex::new(RedactingWriter {
                inner: non_blocking,
            })),
        )
        .init();

    Ok((guard, log_dir.to_string_lossy().to_string()))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use std::sync::Mutex as StdMutex;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;

    static TEST_LOCK: StdMutex<()> = StdMutex::new(());

    #[test]
    fn redact_all_cases() {
        let _lock = TEST_LOCK.lock().unwrap();
        set_redaction_token("ghp_secret123");
        let result = redact("my token is ghp_secret123 here");
        assert_eq!(result, "my token is [REDACTED] here");

        set_redaction_token("");
        let result = redact("nothing to replace");
        assert_eq!(result, "nothing to replace");

        set_redaction_token("ghp_multi");
        let result = redact("ghp_multi and ghp_multi again");
        assert_eq!(result, "[REDACTED] and [REDACTED] again");
    }

    #[test]
    fn redacting_writer_replaces_token() {
        let _lock = TEST_LOCK.lock().unwrap();
        set_redaction_token("ghp_writer_test");
        let cursor = Cursor::new(Vec::new());
        let mut writer = RedactingWriter { inner: cursor };
        writer.write_all(b"token: ghp_writer_test in log").unwrap();
        let result = String::from_utf8(writer.inner.into_inner()).unwrap();
        assert!(!result.contains("ghp_writer_test"));
        assert!(result.contains("[REDACTED]"));
    }

    #[test]
    fn token_not_in_log_output_regression() {
        let _lock = TEST_LOCK.lock().unwrap();
        let test_token = "ghp_regression_998877";
        set_redaction_token(test_token);

        let buf: std::sync::Arc<std::sync::Mutex<Vec<u8>>> =
            std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let buf_clone = buf.clone();

        let writer = Mutex::new(RedactingWriter {
            inner: BufWriter::new(buf_clone),
        });

        let _guard = tracing_subscriber::registry()
            .with(EnvFilter::new("debug"))
            .with(tracing_subscriber::fmt::layer().with_writer(writer))
            .set_default();

        tracing::info!("fetching with token {}", test_token);
        tracing::warn!("auth header: Bearer {}", test_token);

        let guard = buf.lock().unwrap();
        let output = String::from_utf8_lossy(&guard).to_string();
        assert!(
            !output.contains(test_token),
            "token leaked into log output: {output}"
        );
    }

    struct BufWriter {
        inner: std::sync::Arc<std::sync::Mutex<Vec<u8>>>,
    }

    impl BufWriter {
        fn new(inner: std::sync::Arc<std::sync::Mutex<Vec<u8>>>) -> Self {
            Self { inner }
        }
    }

    impl Write for BufWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.inner.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
}
