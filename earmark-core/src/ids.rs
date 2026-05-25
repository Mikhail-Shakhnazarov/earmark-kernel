use std::marker::PhantomData;
use std::ops::Deref;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::errors::CoreError;
use crate::kind::Kind;

pub trait IdSpec: Sized {
    const PREFIX: &'static str;
    fn generate_body() -> String;
    fn validate_body(body: &str) -> Result<(), CoreError>;
    fn extra_parse(_full: &str) -> Result<Option<String>, CoreError> {
        Ok(None)
    }
    fn extra_new() -> Option<String> {
        None
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct TypedId<T: IdSpec>(String, #[serde(skip)] PhantomData<T>);

impl<T: IdSpec> TypedId<T> {
    pub fn generate() -> Self {
        if let Some(body) = T::extra_new() {
            return Self(format!("{}{}", T::PREFIX, body), PhantomData);
        }
        Self(format!("{}{}", T::PREFIX, T::generate_body()), PhantomData)
    }

    pub fn parse(s: impl Into<String>) -> Result<Self, CoreError> {
        let s = s.into();
        if let Some(resolved) = T::extra_parse(&s)? {
            return Ok(Self(resolved, PhantomData));
        }
        let prefix = T::PREFIX;
        if !s.starts_with(prefix) {
            return Err(CoreError::InvalidIdentifier(format!(
                "must start with {}",
                prefix
            )));
        }
        let body = &s[prefix.len()..];
        T::validate_body(body)?;
        Ok(Self(s, PhantomData))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

pub trait IntoObjectId {
    fn as_object_id(&self) -> ObjectId;
}

impl IntoObjectId for ObjectId {
    fn as_object_id(&self) -> ObjectId {
        self.clone()
    }
}

impl IntoObjectId for TransitionAssignmentId {
    fn as_object_id(&self) -> ObjectId {
        ObjectId::parse(self.0.clone())
            .expect("TransitionAssignmentId should follow ObjectId format")
    }
}

impl IntoObjectId for ChangeSetId {
    fn as_object_id(&self) -> ObjectId {
        ObjectId::parse(self.0.clone()).expect("ChangeSetId should follow ObjectId format")
    }
}

impl IntoObjectId for UndoRecordId {
    fn as_object_id(&self) -> ObjectId {
        ObjectId::parse(self.0.clone()).expect("UndoRecordId should follow ObjectId format")
    }
}

impl IntoObjectId for HandoffManifestId {
    fn as_object_id(&self) -> ObjectId {
        ObjectId::parse(self.0.clone()).expect("HandoffManifestId should follow ObjectId format")
    }
}

impl<T: IdSpec> Deref for TypedId<T> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: IdSpec> PartialEq<str> for TypedId<T> {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl<T: IdSpec> PartialEq<&str> for TypedId<T> {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

impl<T: IdSpec> PartialEq<String> for TypedId<T> {
    fn eq(&self, other: &String) -> bool {
        &self.0 == other
    }
}

impl Default for ObjectId {
    fn default() -> Self {
        Self::generate()
    }
}

impl Default for VersionId {
    fn default() -> Self {
        Self::generate()
    }
}

impl Default for RunId {
    fn default() -> Self {
        Self::generate()
    }
}

impl Default for TransitionAssignmentId {
    fn default() -> Self {
        Self::generate()
    }
}

impl Default for ChangeSetId {
    fn default() -> Self {
        Self::generate()
    }
}

impl Default for UndoRecordId {
    fn default() -> Self {
        Self::generate()
    }
}

impl Default for HandoffManifestId {
    fn default() -> Self {
        Self::generate()
    }
}

impl<T: IdSpec> std::fmt::Display for TypedId<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<T: IdSpec> FromStr for TypedId<T> {
    type Err = CoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl<T: IdSpec> AsRef<str> for TypedId<T> {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl<T: IdSpec> TryFrom<String> for TypedId<T> {
    type Error = CoreError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl<T: IdSpec> From<TypedId<T>> for String {
    fn from(id: TypedId<T>) -> Self {
        id.0
    }
}

impl<'de, T: IdSpec> Deserialize<'de> for TypedId<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        TypedId::parse(s).map_err(serde::de::Error::custom)
    }
}

fn hex32_body() -> String {
    Uuid::new_v4().simple().to_string()
}

fn validate_hex32(body: &str) -> Result<(), CoreError> {
    if body.len() != 32
        || !body
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
    {
        return Err(CoreError::InvalidIdentifier(
            "expected 32 lowercase hex characters".to_string(),
        ));
    }
    Ok(())
}

fn validate_lower_digits_ext(body: &str, extra: &str, max: usize) -> Result<(), CoreError> {
    validate_lower_digits_ext_impl(body, extra, max, true)
}

fn validate_relaxed_lower_digits(body: &str, extra: &str, max: usize) -> Result<(), CoreError> {
    validate_lower_digits_ext_impl(body, extra, max, false)
}

fn validate_lower_digits_ext_impl(
    body: &str,
    extra: &str,
    max: usize,
    require_letter_start: bool,
) -> Result<(), CoreError> {
    if body.is_empty() || body.len() > max {
        return Err(CoreError::InvalidIdentifier(format!(
            "must be 1..={} characters",
            max
        )));
    }
    let mut chars = body.chars();
    let first = chars.next().expect("checked non-empty");
    if require_letter_start {
        if !first.is_ascii_lowercase() {
            return Err(CoreError::InvalidIdentifier(
                "must start with a lowercase letter".to_string(),
            ));
        }
    } else {
        if !first.is_ascii_lowercase() && !first.is_ascii_digit() {
            return Err(CoreError::InvalidIdentifier(
                "must start with a lowercase letter or digit".to_string(),
            ));
        }
    }
    if !chars.all(|c| {
        c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-' || extra.contains(c)
    }) {
        return Err(CoreError::InvalidIdentifier(format!(
            "can only contain lowercase letters, digits{}",
            if extra.is_empty() {
                String::new()
            } else {
                format!(", and {}", extra)
            }
        )));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectIdSpec;
impl IdSpec for ObjectIdSpec {
    const PREFIX: &'static str = "obj_";
    fn generate_body() -> String {
        hex32_body()
    }
    fn validate_body(body: &str) -> Result<(), CoreError> {
        if body.len() > 124 {
            return Err(CoreError::InvalidIdentifier(
                "length exceeds 128 characters".to_string(),
            ));
        }
        validate_hex32(body)
    }
}
pub type ObjectId = TypedId<ObjectIdSpec>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VersionIdSpec;
impl IdSpec for VersionIdSpec {
    const PREFIX: &'static str = "ver_";
    fn generate_body() -> String {
        hex32_body()
    }
    fn validate_body(body: &str) -> Result<(), CoreError> {
        if body.len() > 124 {
            return Err(CoreError::InvalidIdentifier(
                "length exceeds 128 characters".to_string(),
            ));
        }
        validate_hex32(body)
    }
    fn extra_parse(full: &str) -> Result<Option<String>, CoreError> {
        if full == "latest" {
            return Ok(Some("ver_00000000000000000000000000000000".to_string()));
        }
        Ok(None)
    }
    fn extra_new() -> Option<String> {
        Some(hex32_body())
    }
}
pub type VersionId = TypedId<VersionIdSpec>;

impl VersionId {
    pub fn is_latest_sentinel(&self) -> bool {
        self.as_str() == "ver_00000000000000000000000000000000"
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SymbolicNameSpec;
impl IdSpec for SymbolicNameSpec {
    const PREFIX: &'static str = "";
    fn generate_body() -> String {
        panic!("SymbolicName cannot be auto-generated")
    }
    fn validate_body(body: &str) -> Result<(), CoreError> {
        validate_lower_digits_ext(body, "", 64)
    }
}
pub type SymbolicName = TypedId<SymbolicNameSpec>;

/// # ID Normalization Policy
///
/// Earmark uses a **Canonical Prefix Persistence** policy for domain identifiers:
///
/// 1. **Internalization**: All identifiers are internalized and persisted with their
///    mandatory prefix (e.g., "run_", "tr_").
/// 2. **Parsing**: User-provided inputs (via CLI or API) can be either raw symbolic
///    names ("my_run") or already-prefixed ("run_my_run"). The `parse()` method
///    automatically normalizes raw names to their prefixed form.
/// 3. **Persistence**: Storage layers (SQLite, Git) always store the prefixed form.
/// 4. **Comparison**: All internal logic and filtering must use the prefixed form.
/// 5. **Ergonomics**: Helpers like `RunId::name()` or `TransitionId::name()` can be
///    used to strip the prefix for display purposes.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RunIdSpec;
impl IdSpec for RunIdSpec {
    const PREFIX: &'static str = "run_";
    fn generate_body() -> String {
        hex32_body()
    }
    fn validate_body(body: &str) -> Result<(), CoreError> {
        validate_relaxed_lower_digits(body, "", 64)
    }
    fn extra_parse(full: &str) -> Result<Option<String>, CoreError> {
        // Allow parsing from plain symbolic names by adding prefix if missing
        if !full.starts_with(Self::PREFIX) && !full.is_empty() {
            if validate_relaxed_lower_digits(full, "", 64).is_ok() {
                return Ok(Some(format!("{}{}", Self::PREFIX, full)));
            }
        }
        Ok(None)
    }
}
pub type RunId = TypedId<RunIdSpec>;

impl RunId {
    /// Returns the raw body without the prefix.
    pub fn name(&self) -> &str {
        &self.as_str()[RunIdSpec::PREFIX.len()..]
    }

    /// Normalizes a string to a RunId if possible.
    pub fn normalize(s: &str) -> Result<Self, CoreError> {
        Self::parse(s)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TransitionIdSpec;
impl IdSpec for TransitionIdSpec {
    const PREFIX: &'static str = "tr_";
    fn generate_body() -> String {
        panic!("TransitionId cannot be auto-generated outside of a workflow context")
    }
    fn validate_body(body: &str) -> Result<(), CoreError> {
        validate_lower_digits_ext(body, "", 64)
    }
    fn extra_parse(full: &str) -> Result<Option<String>, CoreError> {
        // Allow parsing from plain symbolic names by adding prefix if missing
        if !full.starts_with(Self::PREFIX) && !full.is_empty() {
            // Check if it's a valid name first
            if validate_lower_digits_ext(full, "", 64).is_ok() {
                return Ok(Some(format!("{}{}", Self::PREFIX, full)));
            }
        }
        Ok(None)
    }
}
pub type TransitionId = TypedId<TransitionIdSpec>;

impl TransitionId {
    /// Returns the raw name without the prefix.
    pub fn name(&self) -> &str {
        &self.as_str()[TransitionIdSpec::PREFIX.len()..]
    }

    /// Normalizes a string to a TransitionId if possible.
    pub fn normalize(s: &str) -> Result<Self, CoreError> {
        Self::parse(s)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TransitionAssignmentIdSpec;
impl IdSpec for TransitionAssignmentIdSpec {
    const PREFIX: &'static str = "obj_";
    fn generate_body() -> String {
        hex32_body()
    }
    fn validate_body(body: &str) -> Result<(), CoreError> {
        validate_hex32(body)
    }
}
pub type TransitionAssignmentId = TypedId<TransitionAssignmentIdSpec>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ChangeSetIdSpec;
impl IdSpec for ChangeSetIdSpec {
    const PREFIX: &'static str = "obj_";
    fn generate_body() -> String {
        hex32_body()
    }
    fn validate_body(body: &str) -> Result<(), CoreError> {
        validate_hex32(body)
    }
}
pub type ChangeSetId = TypedId<ChangeSetIdSpec>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UndoRecordIdSpec;
impl IdSpec for UndoRecordIdSpec {
    const PREFIX: &'static str = "obj_";
    fn generate_body() -> String {
        hex32_body()
    }
    fn validate_body(body: &str) -> Result<(), CoreError> {
        validate_hex32(body)
    }
}
pub type UndoRecordId = TypedId<UndoRecordIdSpec>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HandoffManifestIdSpec;
impl IdSpec for HandoffManifestIdSpec {
    const PREFIX: &'static str = "obj_";
    fn generate_body() -> String {
        hex32_body()
    }
    fn validate_body(body: &str) -> Result<(), CoreError> {
        validate_hex32(body)
    }
}
pub type HandoffManifestId = TypedId<HandoffManifestIdSpec>;

fn validate_dimension_like(body: &str, max: usize, name: &str) -> Result<(), CoreError> {
    if body.is_empty() || body.len() > max {
        return Err(CoreError::InvalidIdentifier(format!(
            "{} must be 1..={} characters",
            name, max
        )));
    }
    if !body
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == ':' || c == '_' || c == '-')
    {
        return Err(CoreError::InvalidIdentifier(format!(
            "{} can only contain lowercase letters, digits, colons, underscores, and hyphens",
            name
        )));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DimensionIdSpec;
impl IdSpec for DimensionIdSpec {
    const PREFIX: &'static str = "";
    fn generate_body() -> String {
        panic!("DimensionId cannot be auto-generated")
    }
    fn validate_body(body: &str) -> Result<(), CoreError> {
        validate_dimension_like(body, 128, "dimension ID")
    }
}
pub type DimensionId = TypedId<DimensionIdSpec>;

impl DimensionId {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into(), PhantomData)
    }
    pub fn from_static(s: &'static str) -> Self {
        Self::parse(s).expect("invalid static dimension ID")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TokenIdSpec;
impl IdSpec for TokenIdSpec {
    const PREFIX: &'static str = "";
    fn generate_body() -> String {
        panic!("TokenId cannot be auto-generated")
    }
    fn validate_body(body: &str) -> Result<(), CoreError> {
        if body.is_empty() || body.len() > 64 {
            return Err(CoreError::InvalidIdentifier(
                "token ID must be 1..=64 characters".to_string(),
            ));
        }
        if !body
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
        {
            return Err(CoreError::InvalidIdentifier(
                "token ID can only contain lowercase letters, digits, underscores, and hyphens"
                    .to_string(),
            ));
        }
        Ok(())
    }
}
pub type TokenId = TypedId<TokenIdSpec>;

impl TokenId {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into(), PhantomData)
    }
    pub fn from_static(s: &'static str) -> Self {
        Self::parse(s).expect("invalid static token ID")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KernelProtocolIdSpec;
impl IdSpec for KernelProtocolIdSpec {
    const PREFIX: &'static str = "";
    fn generate_body() -> String {
        panic!("KernelProtocolId cannot be auto-generated")
    }
    fn validate_body(body: &str) -> Result<(), CoreError> {
        validate_dimension_like(body, 128, "protocol ID")
    }
}
pub type KernelProtocolId = TypedId<KernelProtocolIdSpec>;

impl KernelProtocolId {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into(), PhantomData)
    }
    pub fn from_static(s: &'static str) -> Self {
        Self::parse(s).expect("invalid static protocol ID")
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

    #[test]
    fn test_run_id_normalization() {
        // Raw input
        let id1 = RunId::parse("test_run").unwrap();
        assert_eq!(id1.as_str(), "run_test_run");
        assert_eq!(id1.name(), "test_run");

        // Prefixed input
        let id2 = RunId::parse("run_other_run").unwrap();
        assert_eq!(id2.as_str(), "run_other_run");
        assert_eq!(id2.name(), "other_run");

        // Digit-prefixed (relaxed)
        let id3 = RunId::parse("123run").unwrap();
        assert_eq!(id3.as_str(), "run_123run");
        assert_eq!(id3.name(), "123run");

        // Invalid
        assert!(RunId::parse("").is_err());
        assert!(RunId::parse("has space").is_err());
    }

    #[test]
    fn test_transition_id_normalization() {
        // Raw input
        let id1 = TransitionId::parse("op_review").unwrap();
        assert_eq!(id1.as_str(), "tr_op_review");
        assert_eq!(id1.name(), "op_review");

        // Prefixed input
        let id2 = TransitionId::parse("tr_op_extract").unwrap();
        assert_eq!(id2.as_str(), "tr_op_extract");
        assert_eq!(id2.name(), "op_extract");

        // Strict alpha-start for TransitionId (unlike RunId)
        assert!(TransitionId::parse("123op").is_err());

        // Invalid
        assert!(TransitionId::parse("").is_err());
    }

    #[test]
    fn test_object_id_new_generates_valid() {
        let id = ObjectId::generate();
        assert!(id.as_str().starts_with("obj_"));
        assert_eq!(id.as_str().len(), 36);
        assert!(ObjectId::parse(id.as_str()).is_ok());
    }

    #[test]
    fn test_version_id_latest_sentinel() {
        let v = VersionId::parse("latest").unwrap();
        assert!(v.is_latest_sentinel());
        assert_eq!(v.as_str(), "ver_00000000000000000000000000000000");
    }

    #[test]
    fn test_typed_id_serialize_roundtrip() {
        let id = ObjectId::parse("obj_00000000000000000000000000000001").unwrap();
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"obj_00000000000000000000000000000001\"");
        let back: ObjectId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, back);
    }
}
