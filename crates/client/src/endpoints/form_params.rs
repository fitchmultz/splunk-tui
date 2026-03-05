//! Shared form parameter building utilities for endpoint modules.
//!
//! This module provides declarative macros [`form_params!`] and [`form_params_str!`]
//! to reduce repetitive form parameter construction patterns across endpoint implementations.
//!
//! # Example Usage
//!
//! ```ignore
//! // For Vec<(String, String)> containers:
//! let mut form_params: Vec<(String, String)> = vec![];
//! form_params! { form_params =>
//!     "name" => required_clone params.name,
//!     "maxSize" => params.max_data_size_mb,
//!     "homePath" => ref params.home_path,
//!     "roles" => join params.roles,
//!     "password" => secret &params.password,
//! }
//!
//! // For Vec<(&str, String)> containers:
//! let mut form_params: Vec<(&str, String)> = vec![];
//! form_params_str! { form_params =>
//!     "name" => str Some(request.name),
//!     "disabled" => required_bool request.disabled,
//!     "quota" => params.quota_bytes,
//! }
//! ```

/// Build form parameters using `String` keys.
///
/// Use this variant when the target vector is `Vec<(String, String)>`.
///
/// # Syntax Patterns
///
/// - `key => expr` - For `Option<T>` where `T: Display`, includes if Some
/// - `key => ref expr` - For `Option<String>`, includes if Some with clone
/// - `key => join expr` - For `Vec<String>`, includes if not empty with comma-join
/// - `key => join_opt expr` - For `Option<Vec<String>>`, includes if Some and not empty
/// - `key => secret &expr` - For required `SecretString`, always includes
/// - `key => secret_opt &expr` - For `Option<SecretString>`, includes if Some
/// - `key => required expr` - For required fields with Display
/// - `key => required_clone expr` - For required String fields
#[macro_export]
macro_rules! form_params {
    // Base case: no more parameters
    ($vec:ident =>) => {};

    // Required field with explicit clone (for String) - MUST be before generic expr
    ($vec:ident => $key:literal => required_clone $val:expr, $($rest:tt)*) => {
        $vec.push(($key.to_string(), $val.clone()));
        $crate::form_params!($vec => $($rest)*);
    };

    // Required field with to_string() (for &str, etc.) - MUST be before generic expr
    ($vec:ident => $key:literal => required $val:expr, $($rest:tt)*) => {
        $vec.push(($key.to_string(), $val.to_string()));
        $crate::form_params!($vec => $($rest)*);
    };

    // Required SecretString - always includes - MUST be before generic expr
    ($vec:ident => $key:literal => secret &$val:expr, $($rest:tt)*) => {
        $vec.push(($key.to_string(), $val.expose_secret().to_string()));
        $crate::form_params!($vec => $($rest)*);
    };

    // Optional SecretString - includes if Some - MUST be before generic expr
    ($vec:ident => $key:literal => secret_opt &$val:expr, $($rest:tt)*) => {
        if let Some(ref v) = $val {
            $vec.push(($key.to_string(), v.expose_secret().to_string()));
        }
        $crate::form_params!($vec => $($rest)*);
    };

    // Vec<String> direct - joins with comma if not empty - MUST be before generic expr
    ($vec:ident => $key:literal => join $val:expr, $($rest:tt)*) => {
        if !$val.is_empty() {
            $vec.push(($key.to_string(), $val.join(",")));
        }
        $crate::form_params!($vec => $($rest)*);
    };

    // Option<Vec<String>> - joins with comma if Some and not empty - MUST be before generic expr
    ($vec:ident => $key:literal => join_opt $val:expr, $($rest:tt)*) => {
        if let Some(ref v) = $val {
            if !v.is_empty() {
                $vec.push(($key.to_string(), v.join(",")));
            }
        }
        $crate::form_params!($vec => $($rest)*);
    };

    // Option<String> with ref (clones the string) - MUST be before generic expr
    ($vec:ident => $key:literal => ref $val:expr, $($rest:tt)*) => {
        if let Some(ref v) = $val {
            $vec.push(($key.to_string(), v.clone()));
        }
        $crate::form_params!($vec => $($rest)*);
    };

    // Option<T> for Copy/Display types (uses to_string()) - LAST because most generic
    ($vec:ident => $key:literal => $val:expr, $($rest:tt)*) => {
        if let Some(v) = $val {
            $vec.push(($key.to_string(), v.to_string()));
        }
        $crate::form_params!($vec => $($rest)*);
    };
}

/// Build form parameters using `&'static str` keys.
///
/// Use this variant when the target vector is `Vec<(&str, String)>`.
///
/// # Syntax Patterns
///
/// - `key => expr` - For `Option<T>` where `T: Display`, includes if Some
/// - `key => str expr` - For `Option<&str>`, includes if Some
/// - `key => bool expr` - For `Option<bool>`, includes if Some
/// - `key => required_bool expr` - For required `bool`, always includes
#[macro_export]
macro_rules! form_params_str {
    // Base case: no more parameters
    ($vec:ident =>) => {};

    // Option<T> for Copy/Display types (uses to_string())
    ($vec:ident => $key:literal => $val:expr, $($rest:tt)*) => {
        if let Some(v) = $val {
            $vec.push(($key, v.to_string()));
        }
        $crate::form_params_str!($vec => $($rest)*);
    };

    // Option<&str> - converts to String
    ($vec:ident => $key:literal => str $val:expr, $($rest:tt)*) => {
        if let Some(v) = $val {
            $vec.push(($key, v.to_string()));
        }
        $crate::form_params_str!($vec => $($rest)*);
    };

    // Option<bool> - includes if Some
    ($vec:ident => $key:literal => bool $val:expr, $($rest:tt)*) => {
        if let Some(v) = $val {
            $vec.push(($key, v.to_string()));
        }
        $crate::form_params_str!($vec => $($rest)*);
    };

    // Required bool - always includes
    ($vec:ident => $key:literal => required_bool $val:expr, $($rest:tt)*) => {
        $vec.push(($key, $val.to_string()));
        $crate::form_params_str!($vec => $($rest)*);
    };
}

#[cfg(test)]
mod tests {
    use secrecy::{ExposeSecret, SecretString};

    #[test]
    fn test_form_params_option_copy() {
        let mut params: Vec<(String, String)> = vec![];
        let value: Option<usize> = Some(100);
        form_params! { params =>
            "maxSize" => value,
        }
        assert_eq!(params, vec![("maxSize".to_string(), "100".to_string())]);
    }

    #[test]
    fn test_form_params_option_copy_none() {
        let mut params: Vec<(String, String)> = vec![];
        let value: Option<usize> = None;
        form_params! { params =>
            "maxSize" => value,
        }
        assert!(params.is_empty());
    }

    #[test]
    fn test_form_params_option_string_ref() {
        let mut params: Vec<(String, String)> = vec![];
        let value: Option<String> = Some("test".to_string());
        form_params! { params =>
            "name" => ref value,
        }
        assert_eq!(params, vec![("name".to_string(), "test".to_string())]);
    }

    #[test]
    fn test_form_params_option_string_ref_none() {
        let mut params: Vec<(String, String)> = vec![];
        let value: Option<String> = None;
        form_params! { params =>
            "name" => ref value,
        }
        assert!(params.is_empty());
    }

    #[test]
    fn test_form_params_vec_join() {
        let mut params: Vec<(String, String)> = vec![];
        let value = ["admin".to_string(), "user".to_string()];
        form_params! { params =>
            "roles" => join value,
        }
        assert_eq!(
            params,
            vec![("roles".to_string(), "admin,user".to_string())]
        );
    }

    #[test]
    fn test_form_params_vec_join_empty() {
        let mut params: Vec<(String, String)> = vec![];
        let value: Vec<String> = vec![];
        form_params! { params =>
            "roles" => join value,
        }
        assert!(params.is_empty());
    }

    #[test]
    fn test_form_params_option_vec_join() {
        let mut params: Vec<(String, String)> = vec![];
        let value: Option<Vec<String>> = Some(vec!["admin".to_string(), "user".to_string()]);
        form_params! { params =>
            "roles" => join_opt value,
        }
        assert_eq!(
            params,
            vec![("roles".to_string(), "admin,user".to_string())]
        );
    }

    #[test]
    fn test_form_params_option_vec_join_none() {
        let mut params: Vec<(String, String)> = vec![];
        let value: Option<Vec<String>> = None;
        form_params! { params =>
            "roles" => join_opt value,
        }
        assert!(params.is_empty());
    }

    #[test]
    fn test_form_params_option_vec_join_empty() {
        let mut params: Vec<(String, String)> = vec![];
        let value: Option<Vec<String>> = Some(vec![]);
        form_params! { params =>
            "roles" => join_opt value,
        }
        assert!(params.is_empty());
    }

    #[test]
    fn test_form_params_secret_required() {
        let mut params: Vec<(String, String)> = vec![];
        let secret = SecretString::from("password123");
        form_params! { params =>
            "password" => secret &secret,
        }
        assert_eq!(
            params,
            vec![("password".to_string(), "password123".to_string())]
        );
    }

    #[test]
    fn test_form_params_secret_optional() {
        let mut params: Vec<(String, String)> = vec![];
        let secret: Option<SecretString> = Some(SecretString::from("password123"));
        form_params! { params =>
            "password" => secret_opt &secret,
        }
        assert_eq!(
            params,
            vec![("password".to_string(), "password123".to_string())]
        );
    }

    #[test]
    fn test_form_params_secret_optional_none() {
        let mut params: Vec<(String, String)> = vec![];
        let secret: Option<SecretString> = None;
        form_params! { params =>
            "password" => secret_opt &secret,
        }
        assert!(params.is_empty());
    }

    #[test]
    fn test_form_params_required() {
        let mut params: Vec<(String, String)> = vec![];
        let name = "test_index";
        form_params! { params =>
            "name" => required name,
        }
        assert_eq!(params, vec![("name".to_string(), "test_index".to_string())]);
    }

    #[test]
    fn test_form_params_required_clone() {
        let mut params: Vec<(String, String)> = vec![];
        let name = "test_index".to_string();
        form_params! { params =>
            "name" => required_clone name,
        }
        assert_eq!(params, vec![("name".to_string(), "test_index".to_string())]);
    }

    #[test]
    fn test_form_params_str_option() {
        let mut params: Vec<(&str, String)> = vec![];
        let value: Option<usize> = Some(100);
        form_params_str! { params =>
            "maxSize" => value,
        }
        assert_eq!(params, vec![("maxSize", "100".to_string())]);
    }

    #[test]
    fn test_form_params_str_option_none() {
        let mut params: Vec<(&str, String)> = vec![];
        let value: Option<usize> = None;
        form_params_str! { params =>
            "maxSize" => value,
        }
        assert!(params.is_empty());
    }

    #[test]
    fn test_form_params_str_option_str() {
        let mut params: Vec<(&str, String)> = vec![];
        let value: Option<&str> = Some("test");
        form_params_str! { params =>
            "name" => str value,
        }
        assert_eq!(params, vec![("name", "test".to_string())]);
    }

    #[test]
    fn test_form_params_str_option_str_none() {
        let mut params: Vec<(&str, String)> = vec![];
        let value: Option<&str> = None;
        form_params_str! { params =>
            "name" => str value,
        }
        assert!(params.is_empty());
    }

    #[test]
    fn test_form_params_str_option_bool() {
        let mut params: Vec<(&str, String)> = vec![];
        let value: Option<bool> = Some(true);
        form_params_str! { params =>
            "disabled" => bool value,
        }
        assert_eq!(params, vec![("disabled", "true".to_string())]);
    }

    #[test]
    fn test_form_params_str_option_bool_none() {
        let mut params: Vec<(&str, String)> = vec![];
        let value: Option<bool> = None;
        form_params_str! { params =>
            "disabled" => bool value,
        }
        assert!(params.is_empty());
    }

    #[test]
    fn test_form_params_str_required_bool() {
        let mut params: Vec<(&str, String)> = vec![];
        let value = true;
        form_params_str! { params =>
            "disabled" => required_bool value,
        }
        assert_eq!(params, vec![("disabled", "true".to_string())]);
    }

    #[test]
    fn test_form_params_multiple() {
        let mut params: Vec<(String, String)> = vec![];
        let max_size: Option<usize> = Some(100);
        let home_path: Option<String> = Some("/home".to_string());
        let roles = ["admin".to_string()];

        form_params! { params =>
            "maxSize" => max_size,
            "homePath" => ref home_path,
            "roles" => join roles,
        }

        assert_eq!(params.len(), 3);
        assert_eq!(params[0], ("maxSize".to_string(), "100".to_string()));
        assert_eq!(params[1], ("homePath".to_string(), "/home".to_string()));
        assert_eq!(params[2], ("roles".to_string(), "admin".to_string()));
    }

    #[test]
    fn test_form_params_str_multiple() {
        let mut params: Vec<(&str, String)> = vec![];
        let quota: Option<usize> = Some(1000);
        let desc: Option<&str> = Some("test");
        let disabled = true;

        form_params_str! { params =>
            "quota" => quota,
            "description" => str desc,
            "disabled" => required_bool disabled,
        }

        assert_eq!(params.len(), 3);
        assert_eq!(params[0], ("quota", "1000".to_string()));
        assert_eq!(params[1], ("description", "test".to_string()));
        assert_eq!(params[2], ("disabled", "true".to_string()));
    }
}
