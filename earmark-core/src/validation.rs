//! Validation logic for names, sizes, and schemas.

use crate::errors::CoreError;

pub const MAX_OBJECT_SIZE: usize = 10 * 1024 * 1024; // 10 MiB

pub fn validate_class_name(name: &str) -> Result<(), CoreError> {
    if name.is_empty() {
        return Err(CoreError::InvalidIdentifier(
            "class name cannot be empty".to_string(),
        ));
    }
    if name.len() > 64 {
        return Err(CoreError::InvalidIdentifier(
            "class name too long (max 64 chars)".to_string(),
        ));
    }
    let mut chars = name.chars();
    if let Some(first) = chars.next() {
        if !first.is_ascii_lowercase() {
            return Err(CoreError::InvalidIdentifier(
                "class name must start with a lowercase letter".to_string(),
            ));
        }
    }
    for c in chars {
        if !c.is_ascii_lowercase() && !c.is_ascii_digit() && c != '_' {
            return Err(CoreError::InvalidIdentifier(format!(
                "invalid character in class name: {}",
                c
            )));
        }
    }
    Ok(())
}

pub fn validate_title(title: &str) -> Result<(), CoreError> {
    if title.len() > 512 {
        return Err(CoreError::InvalidIdentifier(format!(
            "title too long: {} bytes (max 512)",
            title.len()
        )));
    }
    Ok(())
}

pub fn validate_payload_size(len: usize) -> Result<(), CoreError> {
    if len > MAX_OBJECT_SIZE {
        return Err(CoreError::PayloadTooLarge(len, MAX_OBJECT_SIZE));
    }
    Ok(())
}

pub fn validate_env_var_name(name: &str) -> Result<(), CoreError> {
    if name.is_empty() {
        return Err(CoreError::InvalidIdentifier(
            "env var name cannot be empty".to_string(),
        ));
    }
    if name.len() > 128 {
        return Err(CoreError::InvalidIdentifier(
            "env var name exceeds 128 characters".to_string(),
        ));
    }
    let mut chars = name.chars();
    if let Some(first) = chars.next() {
        if !first.is_ascii_uppercase() {
            return Err(CoreError::InvalidIdentifier(
                "env var name must start with an uppercase letter".to_string(),
            ));
        }
    }
    for c in chars {
        if !c.is_ascii_uppercase() && !c.is_ascii_digit() && c != '_' {
            return Err(CoreError::InvalidIdentifier(format!("invalid character in env var name '{}': must be uppercase alphanumeric with underscores", name)));
        }
    }
    Ok(())
}

pub fn validate_endpoint_url(url: &str) -> Result<(), CoreError> {
    if url.is_empty() {
        return Err(CoreError::InvalidIdentifier(
            "endpoint URL cannot be empty".to_string(),
        ));
    }

    if url.contains('@') {
        return Err(CoreError::SecurityViolation(
            "endpoints must not contain embedded credentials".to_string(),
        ));
    }

    if url.contains('?') || url.contains('#') {
        return Err(CoreError::SecurityViolation(
            "endpoints must not contain query strings or fragments".to_string(),
        ));
    }

    if let Some(rest) = url.strip_prefix("http://") {
        if !rest.starts_with("localhost")
            && !rest.starts_with("127.0.0.1")
            && !rest.starts_with("[::1]")
        {
            return Err(CoreError::SecurityViolation(
                "plain http endpoints only allowed for loopback development".to_string(),
            ));
        }
    } else if !url.starts_with("https://") {
        return Err(CoreError::SecurityViolation(
            "endpoints must use https protocol".to_string(),
        ));
    }

    Ok(())
}

pub fn validate_schema(
    payload: &serde_json::Value,
    schema: &serde_json::Value,
) -> Result<(), CoreError> {
    match jsonschema::validator_for(schema) {
        Ok(validator) => {
            let mut errors = validator.iter_errors(payload).peekable();
            if errors.peek().is_some() {
                let msgs: Vec<String> = errors.map(|e| e.to_string()).collect();
                return Err(CoreError::SchemaViolation(msgs.join("; ")));
            }
            Ok(())
        }
        Err(e) => Err(CoreError::SchemaUnavailable(e.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_env_var_name() {
        assert!(validate_env_var_name("API_KEY").is_ok());
        assert!(validate_env_var_name("MY_ENV_123").is_ok());
        assert!(validate_env_var_name("api_key").is_err());
        assert!(validate_env_var_name("API-KEY").is_err());
        assert!(validate_env_var_name("_START_WITH_UNDERSCORE").is_err());
        assert!(validate_env_var_name("1_START_WITH_DIGIT").is_err());
        assert!(validate_env_var_name("").is_err());
        assert!(validate_env_var_name(&"A".repeat(128)).is_ok());
        assert!(validate_env_var_name(&"A".repeat(129)).is_err());
    }

    #[test]
    fn test_validate_endpoint_url() {
        assert!(validate_endpoint_url("https://api.example.com").is_ok());
        assert!(validate_endpoint_url("http://localhost:8080").is_ok());
        assert!(validate_endpoint_url("http://127.0.0.1:8000").is_ok());
        assert!(validate_endpoint_url("http://[::1]:8000").is_ok());
        assert!(validate_endpoint_url("http://evil.com").is_err());
        assert!(validate_endpoint_url("ftp://api.com").is_err());
        assert!(validate_endpoint_url("https://user:pass@api.com").is_err());
        assert!(validate_endpoint_url("https://api.com?secret=true").is_err());
        assert!(validate_endpoint_url("https://api.com#section").is_err());
    }
}
