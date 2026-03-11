//! Explicit request execution pipeline for authenticated Splunk API calls.
//!
//! Responsibilities:
//! - Acquire auth tokens for requests.
//! - Retry once on refreshable auth failures.
//! - Centralize request-execution policy metadata, tracing, and auth-retry instrumentation.
//!
//! Does NOT handle:
//! - Endpoint-specific HTTP construction.
//! - Cache serialization or response decoding.
//! - Binary-level tracing/exporter setup.
//!
//! Invariants:
//! - Session-auth requests may refresh and retry once on auth failures.
//! - API-token requests never attempt session refresh.

use crate::client::SplunkClient;
use crate::error::Result;
use crate::metrics::ErrorCategory;
use std::future::Future;
use std::time::Instant;

/// Explicit execution policy for a client request.
#[derive(Debug, Clone, Copy)]
pub struct RequestPolicy {
    pub operation: &'static str,
    pub auth_retry: bool,
}

impl RequestPolicy {
    pub const fn for_operation(operation: &'static str) -> Self {
        Self {
            operation,
            auth_retry: true,
        }
    }

    pub const fn without_auth_retry(mut self) -> Self {
        self.auth_retry = false;
        self
    }
}

impl SplunkClient {
    /// Execute an authenticated request with the shared request pipeline.
    pub(crate) async fn execute_request<T, F, Fut>(
        &self,
        policy: RequestPolicy,
        request: F,
    ) -> Result<T>
    where
        F: Fn(String) -> Fut,
        Fut: Future<Output = Result<T>>,
    {
        let span = ::tracing::info_span!("splunk_client_request", operation = policy.operation);
        let _entered = span.enter();
        let started_at = Instant::now();
        let token = self.get_auth_token().await?;
        let result = request(token).await;

        match result {
            Ok(data) => {
                ::tracing::debug!(
                    operation = policy.operation,
                    elapsed_ms = started_at.elapsed().as_millis() as u64,
                    "Request pipeline completed"
                );
                Ok(data)
            }
            Err(ref error)
                if policy.auth_retry && error.is_auth_error() && !self.is_api_token_auth() =>
            {
                ::tracing::warn!(
                    operation = policy.operation,
                    elapsed_ms = started_at.elapsed().as_millis() as u64,
                    "Auth error detected, clearing session and retrying request"
                );
                if let Some(metrics) = &self.metrics {
                    metrics.record_retry(policy.operation, "auth_refresh", 1);
                }
                self.session_manager.clear_session().await;
                let token = self.get_auth_token().await?;
                request(token).await.map_err(|error| {
                    let enriched = self.enrich_auth_error(error);
                    if let Some(metrics) = &self.metrics {
                        metrics.record_error(
                            policy.operation,
                            "executor",
                            ErrorCategory::from(&enriched),
                        );
                    }
                    enriched
                })
            }
            Err(error) => {
                let enriched = self.enrich_auth_error(error);
                if let Some(metrics) = &self.metrics {
                    metrics.record_error(
                        policy.operation,
                        "executor",
                        ErrorCategory::from(&enriched),
                    );
                }
                Err(enriched)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AuthStrategy, ClientError, SplunkClient};
    use secrecy::SecretString;
    use std::io;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use tracing_subscriber::fmt::MakeWriter;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn session_client(base_url: String) -> SplunkClient {
        SplunkClient::builder()
            .base_url(base_url)
            .auth_strategy(AuthStrategy::SessionToken {
                username: "admin".to_string(),
                password: SecretString::new("password".to_string().into()),
            })
            .skip_verify(true)
            .build()
            .expect("session client should build")
    }

    fn api_token_client(base_url: String) -> SplunkClient {
        SplunkClient::builder()
            .base_url(base_url)
            .auth_strategy(AuthStrategy::ApiToken {
                token: SecretString::new("api-token".to_string().into()),
            })
            .skip_verify(true)
            .build()
            .expect("api token client should build")
    }

    #[tokio::test]
    async fn session_auth_retries_once_and_refreshes_token() {
        let mock_server = MockServer::start().await;
        let login_calls = Arc::new(AtomicUsize::new(0));
        let login_calls_clone = login_calls.clone();

        Mock::given(method("POST"))
            .and(path("/services/auth/login"))
            .and(query_param("output_mode", "json"))
            .respond_with(move |_request: &wiremock::Request| {
                login_calls_clone.fetch_add(1, Ordering::SeqCst);
                ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "sessionKey": "fresh-session-token"
                }))
            })
            .mount(&mock_server)
            .await;

        let client = session_client(mock_server.uri());
        client
            .session_manager
            .set_session_token("stale-session-token".to_string(), Some(3600))
            .await;

        let seen_tokens = Arc::new(Mutex::new(Vec::new()));
        let attempts = Arc::new(AtomicUsize::new(0));

        let result = client
            .execute_request(RequestPolicy::for_operation("unit_retry"), {
                let seen_tokens = seen_tokens.clone();
                let attempts = attempts.clone();
                move |token| {
                    let seen_tokens = seen_tokens.clone();
                    let attempts = attempts.clone();
                    async move {
                        seen_tokens.lock().expect("lock poisoned").push(token);
                        let attempt = attempts.fetch_add(1, Ordering::SeqCst);
                        if attempt == 0 {
                            Err(ClientError::SessionExpired {
                                username: "unknown".to_string(),
                            })
                        } else {
                            Ok("ok".to_string())
                        }
                    }
                }
            })
            .await;

        assert_eq!(result.expect("request should succeed after refresh"), "ok");
        assert_eq!(attempts.load(Ordering::SeqCst), 2);
        assert_eq!(login_calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            seen_tokens.lock().expect("lock poisoned").as_slice(),
            ["stale-session-token", "fresh-session-token"]
        );
    }

    #[tokio::test]
    async fn api_token_auth_does_not_retry_on_auth_failure() {
        let client = api_token_client("https://splunk.example.com:8089".to_string());
        let attempts = Arc::new(AtomicUsize::new(0));

        let error = client
            .execute_request(RequestPolicy::for_operation("api_token_failure"), {
                let attempts = attempts.clone();
                move |_token| {
                    let attempts = attempts.clone();
                    async move {
                        attempts.fetch_add(1, Ordering::SeqCst);
                        Err::<String, ClientError>(ClientError::Unauthorized(
                            "token rejected".to_string(),
                        ))
                    }
                }
            })
            .await
            .expect_err("api token auth should fail without retry");

        assert_eq!(attempts.load(Ordering::SeqCst), 1);
        assert!(matches!(error, ClientError::Unauthorized(_)));
    }

    #[tokio::test]
    async fn auth_retry_can_be_disabled_per_operation() {
        let mock_server = MockServer::start().await;
        let login_calls = Arc::new(AtomicUsize::new(0));
        let login_calls_clone = login_calls.clone();

        Mock::given(method("POST"))
            .and(path("/services/auth/login"))
            .and(query_param("output_mode", "json"))
            .respond_with(move |_request: &wiremock::Request| {
                login_calls_clone.fetch_add(1, Ordering::SeqCst);
                ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "sessionKey": "fresh-session-token"
                }))
            })
            .mount(&mock_server)
            .await;

        let client = session_client(mock_server.uri());
        client
            .session_manager
            .set_session_token("stale-session-token".to_string(), Some(3600))
            .await;

        let attempts = Arc::new(AtomicUsize::new(0));
        let error = client
            .execute_request(
                RequestPolicy::for_operation("no_retry").without_auth_retry(),
                {
                    let attempts = attempts.clone();
                    move |_token| {
                        let attempts = attempts.clone();
                        async move {
                            attempts.fetch_add(1, Ordering::SeqCst);
                            Err::<String, ClientError>(ClientError::SessionExpired {
                                username: "unknown".to_string(),
                            })
                        }
                    }
                },
            )
            .await
            .expect_err("request should fail without auth retry");

        assert_eq!(attempts.load(Ordering::SeqCst), 1);
        assert_eq!(login_calls.load(Ordering::SeqCst), 0);
        assert!(matches!(
            error,
            ClientError::SessionExpired { ref username } if username == "admin"
        ));
    }

    #[tokio::test]
    async fn tracing_logs_include_operation_name() {
        let buffer = SharedLogBuffer::default();
        let subscriber = tracing_subscriber::fmt()
            .with_writer(buffer.clone())
            .with_max_level(tracing::Level::DEBUG)
            .without_time()
            .finish();
        let dispatch = tracing::Dispatch::new(subscriber);

        let client = api_token_client("https://splunk.example.com:8089".to_string());
        let _guard = tracing::dispatcher::set_default(&dispatch);

        let result = client
            .execute_request(
                RequestPolicy::for_operation("trace_probe"),
                |_token| async move { Ok::<_, ClientError>("ok".to_string()) },
            )
            .await;

        assert_eq!(result.expect("request should succeed"), "ok");
        let logs = buffer.contents();
        assert!(
            logs.contains("trace_probe"),
            "logs missing operation name: {logs}"
        );
        assert!(
            logs.contains("Request pipeline completed"),
            "logs missing success message: {logs}"
        );
    }

    #[derive(Clone, Default)]
    struct SharedLogBuffer(Arc<Mutex<Vec<u8>>>);

    impl SharedLogBuffer {
        fn contents(&self) -> String {
            String::from_utf8(self.0.lock().expect("lock poisoned").clone())
                .expect("captured logs should be utf-8")
        }
    }

    impl<'a> MakeWriter<'a> for SharedLogBuffer {
        type Writer = BufferWriter;

        fn make_writer(&'a self) -> Self::Writer {
            BufferWriter(self.0.clone())
        }
    }

    struct BufferWriter(Arc<Mutex<Vec<u8>>>);

    impl io::Write for BufferWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.lock().expect("lock poisoned").extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
}
