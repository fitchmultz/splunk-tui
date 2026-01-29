# Security Policy

This document outlines security considerations for using Splunk TUI, including credential management and safe configuration practices.

## Default Credentials

Splunk TUI provides default credentials (`admin`/`changeme`) in its `Config::default()` implementation. These are Splunk's default credentials intended **only** for local development environments.

### Security Implications

- **Default credentials target localhost:8089**: This is the default Splunk management port on the local machine
- **Publicly known credentials**: The default `admin`/`changeme` combination is well-documented and should never be used for production Splunk instances
- **No automatic credential rotation**: The application does not enforce credential changes

### Best Practices

1. **Change default credentials immediately** when connecting to any non-local Splunk instance
2. **Use API tokens** instead of username/password when possible (see below)
3. **Use the system keyring** for secure credential storage (see [Secure Credential Storage](#secure-credential-storage))
4. **Review configuration files** before committing them to version control

## Secure Credential Storage

Splunk TUI supports storing credentials in your system's secure keyring instead of plain text configuration files.

### Using Keyring Storage

In your `config.json`, specify credentials using the `keyring_account` field:

```json
{
  "profiles": {
    "production": {
      "base_url": "https://splunk.example.com:8089",
      "username": "admin",
      "password": { "keyring_account": "splunk-production-admin" }
    }
  }
}
```

Then store the password in your system keyring:

```bash
# macOS
security add-generic-password -s "splunk-tui" -a "splunk-production-admin" -w "your-secure-password"

# Linux (secret-tool)
secret-tool store --label="Splunk Production" service splunk-tui username splunk-production-admin
```

### Benefits

- Credentials are encrypted at rest using OS-provided mechanisms
- Credentials are not visible in config files
- Reduces risk of accidental credential exposure in version control

## Runtime Warnings

Splunk TUI logs a warning when default credentials are detected:

```
WARN Using default Splunk credentials (admin/changeme). These are for local development only - change before production use.
```

This warning appears in:
- Log files (`splunk-tui.log` for the TUI)
- stderr output (for the CLI when using `RUST_LOG=warn`)

## Reporting Security Issues

If you discover a security vulnerability in Splunk TUI:

1. **Do not open a public issue**
2. Contact the maintainers directly with details of the vulnerability
3. Allow reasonable time for a fix before public disclosure

## Security Checklist

Before deploying to production:

- [ ] Changed default `admin`/`changeme` credentials
- [ ] Using API tokens or secure keyring storage for credentials
- [ ] TLS verification enabled (`skip_verify: false`)
- [ ] Configuration files excluded from version control
- [ ] `.env` files excluded from version control (use `.env.example` as template)
- [ ] Reviewed the [secret-commit guard](./docs/usage.md#secret-commit-guard) output

## Related Documentation

- [Configuration Guide](./docs/usage.md#configuration)
- [Secure Credential Storage](./docs/usage.md#secure-credential-storage)
- [Secret-Commit Guard](./docs/usage.md#secret-commit-guard)
