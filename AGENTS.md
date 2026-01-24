# Splunk TUI - Project Philosophy

## Core Principles

1. **Simplicity First**: The project uses straightforward Rust patterns with clear separation of concerns. Each crate has a single responsibility.

2. **Type Safety**: Leverage Rust's type system for compile-time guarantees. Use `thiserror` for library error types and `anyhow` for application error propagation.

3. **Secure by Default**: Credentials are handled with `secrecy` crate. Never log sensitive information. Session tokens are stored securely and auto-renewed.

4. **Testability**: All HTTP interactions are mockable. Unit tests cover business logic; integration tests verify API contracts.

5. **Error Clarity**: Errors provide actionable context. Include relevant details (endpoint, status code, suggested fix) in error messages.

## Architecture

```
┌─────────────────────────────────────────┐
│         User Interface                  │
│  ┌──────────────┐  ┌──────────────┐    │
│  │ CLI (clap)   │  │ TUI (ratatui)│   │
│  └──────┬───────┘  └──────┬───────┘    │
└─────────┼──────────────────┼───────────┘
          └────────┬─────────┘
                   │
┌─────────────────────────────────────────┐
│      Application Logic Layer            │
│  Command Handlers / State Machine       │
└───────────────────┬─────────────────────┘
                    │
┌─────────────────────────────────────────┐
│         Splunk Client Layer             │
│  - Auth (session/API token)             │
│  - Search jobs & results                │
│  - Cluster management                   │
│  - Index operations                     │
└───────────────────┬─────────────────────┘
                    │
┌─────────────────────────────────────────┐
│       Configuration Layer               │
│  Environment, files, CLI args           │
└─────────────────────────────────────────┘
```

## Splunk API Integration

### Authentication
- **Session Token**: Username/password login, session stored and auto-renewed
- **API Token**: Bearer token authentication (preferred for automation)

### Rate Limiting
- Implement exponential backoff for 429 responses
- Default: 3 retries with 1s, 2s, 4s delays

### Search Jobs
- Jobs are created asynchronously
- Poll for completion with exponential backoff
- Results fetched in pages (default: 1000 rows)

## Development Workflow

1. **Feature Development**: Start with types/models, then client, then UI
2. **Testing**: Write tests alongside code. Use mockito for HTTP mocking.
3. **CI Pipeline**: `make ci` must pass before any commit
   - **CRITICAL**: Never end your turn with outstanding CI failures. All tests must pass before completing the implementation phase.
   - If CI fails, you MUST fix it before considering the task done. Transient failures (e.g., test isolation issues) are still failures that require fixes.
4. **Documentation**:
    - Update CLI help (`--help`) when adding commands.
    - Any changes to CLI commands, configuration parameters, or TUI keyboard shortcuts MUST be reflected in `docs/usage.md` before the task is considered complete.
    - The CLI `--help` menu must be kept in sync with the actual implementation and `docs/usage.md`.

## Common Patterns

### Error Handling in Client Library
```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("Authentication failed: {0}")]
    AuthFailed(String),

    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("API error ({status}): {message}")]
    ApiError { status: u16, message: String },

    #[error("Session expired, please re-authenticate")]
    SessionExpired,
}
```

### Configuration Loading
```rust
// Priority: CLI args > .env file > defaults
let config = Config::from_args()?.with_env()?.with_defaults()?;
```

### Async Client Pattern
```rust
pub struct SplunkClient {
    http: reqwest::Client,
    base_url: String,
    auth: AuthStrategy,
}

impl SplunkClient {
    pub async fn search(&self, query: &str) -> Result<Vec<Record>, ClientError> {
        // Auto-retry on 401/403 (session renewal)
        let response = self.request_with_auth(|| async {
            self.http
                .post(&format!("{}/services/search/jobs", self.base_url))
                .query(&[("search", query)])
                .send()
                .await
        }).await?;

        // Parse response
    }
}
```

## Known Constraints

- Splunk Enterprise v9+ REST API
- Minimum Rust version: 1.84
- TLS 1.2+ required for HTTPS connections
- Session tokens expire after 1 hour of inactivity

## Future Enhancements

These are NOT in scope for initial release:
- Distributed search across multiple Splunk instances
- Real-time search updates
- Advanced alerting configuration
- Custom visualization dashboards
