//! Helpers for Markdown, YAML, and JSON serialization/deserialization.

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::errors::CoreError;

pub fn parse_markdown_frontmatter<T: DeserializeOwned>(
    input: &str,
) -> Result<(T, String), CoreError> {
    let normalized = input.replace("\r\n", "\n");
    let trimmed = normalized.trim_start();
    if !trimmed.starts_with("---\n") {
        return Err(CoreError::InvalidFrontmatter(
            "missing opening frontmatter delimiter".to_string(),
        ));
    }

    let rest = &trimmed["---\n".len()..];
    let (yaml, body) = rest.split_once("\n---\n").ok_or_else(|| {
        CoreError::InvalidFrontmatter("missing closing frontmatter delimiter".to_string())
    })?;

    let meta = serde_yaml::from_str::<T>(yaml)?;
    Ok((meta, body.trim_start_matches('\n').to_string()))
}

pub fn to_markdown_frontmatter<T: Serialize>(meta: &T, body: &str) -> Result<String, CoreError> {
    let yaml = serde_yaml::to_string(meta)?;
    Ok(format!("---\n{}---\n\n{}", yaml, body))
}

pub fn parse_yaml<T: DeserializeOwned>(input: &str) -> Result<T, CoreError> {
    Ok(serde_yaml::from_str(input)?)
}

pub fn to_yaml<T: Serialize>(value: &T) -> Result<String, CoreError> {
    Ok(serde_yaml::to_string(value)?)
}

pub fn parse_json<T: DeserializeOwned>(input: &str) -> Result<T, CoreError> {
    Ok(serde_json::from_str(input)?)
}

pub fn to_json_pretty<T: Serialize>(value: &T) -> Result<String, CoreError> {
    Ok(serde_json::to_string_pretty(value)?)
}
