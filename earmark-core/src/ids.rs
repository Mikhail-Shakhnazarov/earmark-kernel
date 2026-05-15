//! Typed identifiers and object/version references used across the core model.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::errors::CoreError;
use crate::kind::Kind;

/// A canonical durable identifier for a kernel object.
///
/// Pattern: `obj_[a-z0-9]{32}`
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ObjectId(String);

impl ObjectId {
    pub fn new() -> Self {
        Self(format!("obj_{}", Uuid::new_v4().simple()))
    }

    pub fn parse(s: impl Into<String>) -> Result<Self, CoreError> {
        let s = s.into();
        if s.len() > 128 {
            return Err(CoreError::InvalidIdentifier(
                "length exceeds 128 characters".to_string(),
            ));
        }

        if !s.starts_with("obj_") {
            return Err(CoreError::InvalidIdentifier(
                "must start with obj_".to_string(),
            ));
        }

        let hex_part = &s[4..];
        if hex_part.len() != 32
            || !hex_part
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        {
            return Err(CoreError::InvalidIdentifier(
                "invalid format: expected obj_ followed by 32 lowercase hex characters".to_string(),
            ));
        }

        Ok(Self(s))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for ObjectId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ObjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct VersionId(String);

impl VersionId {
    pub fn new() -> Self {
        Self(format!("ver_{}", Uuid::new_v4().simple()))
    }

    pub fn parse(s: impl Into<String>) -> Result<Self, CoreError> {
        let s = s.into();
        if s.len() > 128 {
            return Err(CoreError::InvalidIdentifier(
                "length exceeds 128 characters".to_string(),
            ));
        }

        // Pattern: ver_[a-z0-9]{32}
        if !s.starts_with("ver_") {
            return Err(CoreError::InvalidIdentifier(
                "must start with ver_".to_string(),
            ));
        }

        let hex_part = &s[4..];
        if hex_part.len() != 32
            || !hex_part
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        {
            return Err(CoreError::InvalidIdentifier(
                "invalid format: expected ver_ followed by 32 lowercase hex characters".to_string(),
            ));
        }

        Ok(Self(s))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns `true` if this `VersionId` is the "latest" sentinel value
    /// (`ver_00000000000000000000000000000000`). This sentinel is used in
    /// version references to indicate "resolve to the current head" without
    /// requiring a specific version lookup.
    pub fn is_latest_sentinel(&self) -> bool {
        self.0 == "ver_00000000000000000000000000000000"
    }
}

impl Default for VersionId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for VersionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SymbolicName(String);

impl SymbolicName {
    pub fn parse(s: impl Into<String>) -> Result<Self, CoreError> {
        let s = s.into();
        if s.is_empty() || s.len() > 64 {
            return Err(CoreError::InvalidIdentifier(
                "invalid symbolic name length: expected 1..=64 characters".to_string(),
            ));
        }
        let mut chars = s.chars();
        let first = chars.next().expect("checked non-empty");
        if !first.is_ascii_lowercase() {
            return Err(CoreError::InvalidIdentifier(
                "symbolic name must start with a lowercase letter".to_string(),
            ));
        }
        if !chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-') {
            return Err(CoreError::InvalidIdentifier(
                "symbolic name can only contain lowercase letters, digits, underscores, and hyphens"
                    .to_string(),
            ));
        }
        Ok(Self(s))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SymbolicName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct DimensionId(String);

impl DimensionId {
    /// Unchecked constructor for trusted constants only.
    /// Prefer `parse()` for any user- or declaration-derived input.
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Trusted static constructor for built-in kernel dimensions and test constants.
    /// Panics if the input is not a valid dimension ID.
    pub fn from_static(s: &'static str) -> Self {
        Self::parse(s).expect("invalid static dimension ID")
    }

    pub fn parse(s: impl Into<String>) -> Result<Self, CoreError> {
        let s = s.into();
        if s.is_empty() || s.len() > 128 {
            return Err(CoreError::InvalidIdentifier(
                "dimension ID must be 1..=128 characters".to_string(),
            ));
        }
        if !s.chars().all(|c| {
            c.is_ascii_lowercase() || c.is_ascii_digit() || c == ':' || c == '_' || c == '-'
        }) {
            return Err(CoreError::InvalidIdentifier(
                "dimension ID can only contain lowercase letters, digits, colons, underscores, and hyphens".to_string(),
            ));
        }
        Ok(Self(s))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for DimensionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TokenId(String);

impl TokenId {
    /// Unchecked constructor for trusted constants only.
    /// Prefer `parse()` for any user- or declaration-derived input.
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Trusted static constructor for built-in kernel tokens and test constants.
    /// Panics if the input is not a valid token ID.
    pub fn from_static(s: &'static str) -> Self {
        Self::parse(s).expect("invalid static token ID")
    }

    pub fn parse(s: impl Into<String>) -> Result<Self, CoreError> {
        let s = s.into();
        if s.is_empty() || s.len() > 64 {
            return Err(CoreError::InvalidIdentifier(
                "token ID must be 1..=64 characters".to_string(),
            ));
        }
        if !s
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
        {
            return Err(CoreError::InvalidIdentifier(
                "token ID can only contain lowercase letters, digits, underscores, and hyphens"
                    .to_string(),
            ));
        }
        Ok(Self(s))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for TokenId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct KernelProtocolId(String);

impl KernelProtocolId {
    /// Unchecked constructor for trusted constants only.
    /// Prefer `parse()` for any user- or declaration-derived input.
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Trusted static constructor for built-in kernel protocols and test constants.
    /// Panics if the input is not a valid protocol ID.
    pub fn from_static(s: &'static str) -> Self {
        Self::parse(s).expect("invalid static protocol ID")
    }

    pub fn parse(s: impl Into<String>) -> Result<Self, CoreError> {
        let s = s.into();
        if s.is_empty() || s.len() > 128 {
            return Err(CoreError::InvalidIdentifier(
                "protocol ID must be 1..=128 characters".to_string(),
            ));
        }
        if !s.chars().all(|c| {
            c.is_ascii_lowercase() || c.is_ascii_digit() || c == ':' || c == '_' || c == '-'
        }) {
            return Err(CoreError::InvalidIdentifier(
                "protocol ID can only contain lowercase letters, digits, colons, underscores, and hyphens".to_string(),
            ));
        }
        Ok(Self(s))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for KernelProtocolId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PayloadRef(pub String);

impl PayloadRef {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        let digest = hasher.finalize();
        Self(format!("sha256:{}", hex::encode(digest)))
    }
}

impl std::fmt::Display for PayloadRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionRef {
    pub id: ObjectId,
    pub version_id: VersionId,
}

impl VersionRef {
    pub fn new(id: ObjectId, version_id: VersionId) -> Self {
        Self { id, version_id }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectRef {
    pub id: ObjectId,
    pub version_id: VersionId,
    pub kind: Kind,
    pub class: Option<String>,
}

impl ObjectRef {
    pub fn new(id: ObjectId, version_id: VersionId, kind: Kind, class: Option<String>) -> Self {
        Self {
            id,
            version_id,
            kind,
            class,
        }
    }

    pub fn version_ref(&self) -> VersionRef {
        VersionRef::new(self.id.clone(), self.version_id.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_object_id_parse_strict_durable_ids() {
        assert!(ObjectId::parse("obj_00000000000000000000000000000001").is_ok());
        assert!(ObjectId::parse("obj_abc").is_err());
        assert!(ObjectId::parse("finding").is_err());
        assert!(ObjectId::parse("source_note_v2").is_err());
        assert!(ObjectId::parse("Invalid Name").is_err());
    }

    #[test]
    fn test_symbolic_name_parse() {
        assert!(SymbolicName::parse("pkm-core").is_ok());
        assert!(SymbolicName::parse("source_note_v2").is_ok());
        assert!(SymbolicName::parse("a").is_ok());
        assert!(SymbolicName::parse("1not-valid").is_err());
        assert!(SymbolicName::parse("Uppercase").is_err());
        assert!(SymbolicName::parse("contains space").is_err());
        assert!(SymbolicName::parse("").is_err());
    }

    #[test]
    fn test_invalid_dimension_id_fails_parse() {
        assert!(DimensionId::parse("").is_err());
        assert!(DimensionId::parse("a".repeat(129)).is_err());
        assert!(DimensionId::parse("UPPERCASE").is_err());
        assert!(DimensionId::parse("has space").is_err());
        assert!(DimensionId::parse("valid:name").is_ok());
    }

    #[test]
    fn test_invalid_token_id_fails_parse() {
        assert!(TokenId::parse("").is_err());
        assert!(TokenId::parse("a".repeat(65)).is_err());
        assert!(TokenId::parse("UPPERCASE").is_err());
        assert!(TokenId::parse("has space").is_err());
        assert!(TokenId::parse("valid_token").is_ok());
        assert!(TokenId::parse("draft").is_ok());
    }

    #[test]
    fn test_kernel_protocol_id_parse_rejects_invalid() {
        assert!(KernelProtocolId::parse("").is_err());
        assert!(KernelProtocolId::parse("a".repeat(129)).is_err());
        assert!(KernelProtocolId::parse("UPPERCASE").is_err());
        assert!(KernelProtocolId::parse("has space").is_err());
        assert!(KernelProtocolId::parse("kernel:review").is_ok());
        assert!(KernelProtocolId::parse("kernel:visibility").is_ok());
    }

    #[test]
    fn test_from_static_panics_on_invalid_input() {
        use std::panic;
        assert!(panic::catch_unwind(|| DimensionId::from_static("")).is_err());
        assert!(panic::catch_unwind(|| TokenId::from_static("UPPER")).is_err());
        assert!(panic::catch_unwind(|| KernelProtocolId::from_static("")).is_err());
        assert!(panic::catch_unwind(|| DimensionId::from_static("valid:name")).is_ok());
        assert!(panic::catch_unwind(|| TokenId::from_static("valid_name")).is_ok());
        assert!(panic::catch_unwind(|| KernelProtocolId::from_static("kernel:review")).is_ok());
    }
}
