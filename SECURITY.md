# Security Policy

This document outlines security posture, credential handling, and vulnerability reporting for Splunk TUI.

## Safe Defaults

`Config::default()` uses **placeholder** credentials:

- `username`: `replace-with-your-username`
- `password`: `replace-with-your-password`

These placeholders are intentionally non-working and are meant to force explicit configuration.

## Credential Management Best Practices

1. Use API tokens where possible.
2. Prefer system keyring-backed credentials over plaintext secrets.
3. Never commit `.env`, `.env.test`, or real credential files.
4. Keep TLS verification enabled (`skip_verify: false`) outside local test environments.

## Secure Credential Storage

Splunk TUI supports system keyring-backed credentials.

Example profile snippet:

```json
{
  "profiles": {
    "production": {
      "base_url": "https://splunk.example.com:8089",
      "username": "replace-with-your-username",
      "password": { "keyring_account": "splunk-production-user" }
    }
  }
}
```

Then store the secret in your OS keyring:

```bash
# macOS
security add-generic-password -s "splunk-tui" -a "splunk-production-user" -w "your-secure-password"

# Linux (secret-tool)
secret-tool store --label="Splunk Production" service splunk-tui username splunk-production-user
```

## Runtime Warnings

- CLI emits a warning when placeholder credentials are detected in active configuration.
- TUI enters bootstrap/connection-recovery flows when credentials are missing or invalid.

## Secret-Commit Guard

Use the built-in guardrail before commits:

```bash
make lint-secrets
```

This verifies forbidden local secret files are not tracked by git.

## Reporting Security Issues

Do not open public issues for vulnerabilities.

Preferred process:

1. Open a private vulnerability report from the repository **Security** tab (GitHub private reporting).
2. If private reporting is unavailable, contact maintainers directly with reproduction details.
3. Allow reasonable remediation time before public disclosure.

## Security Checklist

Before production use:

- [ ] Placeholder credentials replaced with real secrets
- [ ] API tokens or keyring-backed secrets in use
- [ ] TLS verification enabled (`skip_verify: false`)
- [ ] Secret files excluded from version control
- [ ] `make lint-secrets` passes

## Related Documentation

- [Configuration Guide](./docs/usage.md#configuration)
- [Secure Credential Storage](./docs/usage.md#secure-credential-storage)
- [Secret-Commit Guard](./docs/usage.md#secret-commit-guard)
