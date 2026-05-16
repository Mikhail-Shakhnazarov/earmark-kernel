pub struct Redactor;

impl Redactor {
    pub fn redact_sensitive(text: &str) -> String {
        let text = Self::redact_url_credentials(text);
        let text = Self::redact_auth_header_values(&text);
        let text = Self::redact_env_var_value_outside_brackets(&text);
        let text = Self::redact_known_sensitive_env_names(&text);
        text
    }

    fn redact_url_credentials(text: &str) -> String {
        let mut result = String::with_capacity(text.len());
        let mut rest = text;
        while let Some(start) = rest.find("://") {
            let before = &rest[..start];
            let after_scheme = &rest[start + 3..];

            let userinfo_end = match after_scheme.find('@') {
                Some(pos) => pos,
                None => {
                    result.push_str(before);
                    result.push_str("://");
                    rest = after_scheme;
                    continue;
                }
            };

            let userinfo = &after_scheme[..userinfo_end];
            if userinfo.contains(':') || userinfo.len() >= 8 {
                result.push_str(before);
                result.push_str("://[REDACTED_CREDENTIALS]@");
                rest = &after_scheme[userinfo_end + 1..];
            } else {
                result.push_str(before);
                result.push_str("://");
                result.push_str(userinfo);
                rest = &after_scheme;
            }
        }
        result.push_str(rest);
        result
    }

    fn redact_auth_header_values(text: &str) -> String {
        let patterns: &[(&str, &str)] = &[
            ("Authorization: Bearer ", "[REDACTED_TOKEN]"),
            ("x-api-key: ", "[REDACTED_KEY]"),
            ("api-key: ", "[REDACTED_KEY]"),
            ("api_key=", "[REDACTED_KEY]"),
            ("apikey=", "[REDACTED_KEY]"),
            ("secret=", "[REDACTED_SECRET]"),
            ("token=", "[REDACTED_TOKEN]"),
        ];
        let mut result = text.to_string();
        for (prefix, replacement) in patterns {
            let lower_prefix = prefix.to_lowercase();
            let mut search_start = 0;
            let mut output = String::new();
            loop {
                let lower = result[search_start..].to_lowercase();
                match lower.find(&lower_prefix) {
                    Some(rel_pos) => {
                        let pos = search_start + rel_pos;
                        let val_start = pos + prefix.len();
                        let fragment = &result[val_start..];
                        if fragment.starts_with(replacement) {
                            let end = val_start + replacement.len();
                            output.push_str(&result[search_start..end]);
                            search_start = end;
                            continue;
                        }
                        let val_end = val_start
                            + fragment
                                .find(|c: char| c.is_whitespace() || c == '"' || c == '\'' || c == '&' || c == ',')
                                .unwrap_or(fragment.len().min(128));
                        output.push_str(&result[search_start..pos]);
                        output.push_str(prefix);
                        output.push_str(replacement);
                        search_start = val_end;
                    }
                    None => {
                        output.push_str(&result[search_start..]);
                        break;
                    }
                }
            }
            result = output;
        }
        result
    }

    fn redact_env_var_value_outside_brackets(text: &str) -> String {
        let mut result = text.to_string();
        let patterns: &[(&str, &str)] = &[("<unset:", ">"), ("<empty:", ">")];
        for (prefix, suffix) in patterns {
            let lower_prefix = prefix.to_lowercase();
            loop {
                let lower = result.to_lowercase();
                match lower.find(&lower_prefix) {
                    Some(pos) => {
                        let after = &result[pos + prefix.len()..];
                        let suffix_pos = after.find(suffix);
                        let inner = suffix_pos.map(|e| &after[..e]).unwrap_or("");
                        if inner == "[REDACTED_ENV]" {
                            break;
                        }
                        let end = suffix_pos.map(|e| pos + prefix.len() + e + suffix.len())
                            .unwrap_or(result.len());
                        let mut new = String::with_capacity(result.len());
                        new.push_str(&result[..pos]);
                        new.push_str(prefix);
                        new.push_str("[REDACTED_ENV]");
                        new.push_str(suffix);
                        new.push_str(&result[end..]);
                        result = new;
                    }
                    None => break,
                }
            }
        }
        result
    }

    fn redact_known_sensitive_env_names(text: &str) -> String {
        let candidates = sensitive_env_var_candidates();
        let mut result = text.to_string();
        for name in candidates {
            let quoted = format!("'{}'", name);
            result = result.replace(&quoted, "'[REDACTED_ENV]'");
        }
        result
    }

    pub fn redact_failure_message(message: &str) -> String {
        Self::redact_sensitive(message)
    }
}

fn sensitive_env_var_candidates() -> Vec<String> {
    let mut names = Vec::new();
    for (key, _value) in std::env::vars() {
        let upper = key.to_uppercase();
        if upper.contains("API")
            || upper.contains("KEY")
            || upper.contains("TOKEN")
            || upper.contains("SECRET")
            || upper.contains("AUTH")
            || upper.contains("PASSWORD")
            || upper.contains("PASS")
            || upper.contains("CREDENTIAL")
            || upper.contains("BEARER")
        {
            names.push(key);
        }
    }
    names.sort();
    names.dedup();
    names
}

pub fn redact_sensitive(text: &str) -> String {
    Redactor::redact_sensitive(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redact_url_credentials_basic_auth() {
        let result = redact_sensitive("https://user:password@api.example.com/v1/chat");
        assert_eq!(result, "https://[REDACTED_CREDENTIALS]@api.example.com/v1/chat");
    }

    #[test]
    fn test_redact_url_token_only() {
        let result = redact_sensitive("https://ghp_token123@api.example.com/data");
        assert_eq!(result, "https://[REDACTED_CREDENTIALS]@api.example.com/data");
    }

    #[test]
    fn test_redact_url_no_credentials() {
        let input = "https://api.example.com/v1/chat";
        assert_eq!(redact_sensitive(input), input);
    }

    #[test]
    fn test_redact_auth_bearer() {
        let result = redact_sensitive("Authorization: Bearer sk-abcdef123456");
        assert!(result.contains("[REDACTED_TOKEN]"));
        assert!(!result.contains("sk-abcdef123456"));
    }

    #[test]
    fn test_redact_x_api_key() {
        let result = redact_sensitive("x-api-key: my-secret-key-12345");
        assert!(result.contains("[REDACTED_KEY]"));
        assert!(!result.contains("my-secret-key-12345"));
    }

    #[test]
    fn test_redact_env_var_unset() {
        let result = redact_sensitive("<unset:MY_API_KEY>");
        assert_eq!(result, "<unset:[REDACTED_ENV]>");
    }

    #[test]
    fn test_redact_env_var_empty() {
        let result = redact_sensitive("<empty:MY_SECRET_TOKEN>");
        assert_eq!(result, "<empty:[REDACTED_ENV]>");
    }

    #[test]
    fn test_redact_env_var_name_in_message() {
        std::env::set_var("TEST_MY_API_KEY_FOR_REDACT", "dummy");
        std::env::set_var("TEST_MY_AUTH_TOKEN_FOR_REDACT", "dummy");
        let result = redact_sensitive("auth env variable 'TEST_MY_API_KEY_FOR_REDACT' not set");
        assert_eq!(result, "auth env variable '[REDACTED_ENV]' not set");
        let result2 = redact_sensitive("auth env variable 'TEST_MY_AUTH_TOKEN_FOR_REDACT' not set");
        assert_eq!(result2, "auth env variable '[REDACTED_ENV]' not set");
    }

    #[test]
    fn test_redact_mixed_content() {
        let input = "Error: URL https://user:pass@api.example.com with Authorization: Bearer tok_12345 and env <unset:MY_SECRET>";
        let result = redact_sensitive(input);
        assert!(result.contains("[REDACTED_CREDENTIALS]"));
        assert!(result.contains("[REDACTED_TOKEN]"));
        assert!(result.contains("[REDACTED_ENV]"));
        assert!(!result.contains("user:pass"));
        assert!(!result.contains("tok_12345"));
        assert!(!result.contains("MY_SECRET"));
    }

    #[test]
    fn test_no_false_positives_safe_url() {
        let input = "https://api.example.com/v1/chat/completions";
        assert_eq!(redact_sensitive(input), input);
    }

    #[test]
    fn test_redact_failure_message() {
        let msg = "Provider profile 'test' auth failed: <unset:OPENAI_API_KEY>";
        let result = Redactor::redact_failure_message(msg);
        assert_eq!(result, "Provider profile 'test' auth failed: <unset:[REDACTED_ENV]>");
    }
}
