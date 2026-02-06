//! ResourceDisplay implementation for User.
//!
//! Note: User uses format-specific implementations rather than ResourceDisplay
//! because the format requirements differ significantly:
//! - CSV: lowercase headers, semicolon-separated roles, "0" for null last_login
//! - Table: UPPERCASE headers, comma-separated roles, "-" for null values
//! - XML: nested <role> elements for roles

// User doesn't implement ResourceDisplay because the three formatters
// have significantly different output requirements that can't be unified
// through a simple trait. The original implementations are kept in:
// - csv/users.rs
// - table/users.rs
// - xml/users.rs

#[cfg(test)]
mod tests {
    use splunk_client::User;

    #[test]
    fn test_user_structure() {
        // Basic sanity test that User struct exists
        let _user = User {
            name: "test".to_string(),
            realname: None,
            email: None,
            user_type: None,
            default_app: None,
            roles: vec![],
            last_successful_login: None,
        };
    }
}
