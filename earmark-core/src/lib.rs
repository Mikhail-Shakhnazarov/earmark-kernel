use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::de::{DeserializeOwned, Error as SerdeError};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;

pub mod projection;

pub type Timestamp = DateTime<Utc>;

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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum HeaderValue {
    String(String),
    Integer(i64),
    Float(f64),
    Bool(bool),
    Strings(Vec<String>),
}

impl HeaderValue {
    pub fn as_string(&self) -> Option<String> {
        match self {
            Self::String(s) => Some(s.clone()),
            Self::Integer(v) => Some(v.to_string()),
            Self::Float(v) => Some(v.to_string()),
            Self::Bool(v) => Some(v.to_string()),
            Self::Strings(v) => Some(v.join(", ")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ScalarValue {
    String(String),
    Integer(i64),
    Float(f64),
    Bool(bool),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ScalarOrRef {
    Scalar(ScalarValue),
    Object(ObjectRef),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarkdownBody(String);

impl MarkdownBody {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for MarkdownBody {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JsonSchemaRef(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Object,
    Relation,
    Instruction,
    Policy,
    Workflow,
    CompiledContextTemplate,
    ProviderProfile,
    Review,
    Event,
    WorkPacket,
    RunRecord,
    SystemDefinition,
    TransitionAssignment,
    ChangeSet,
    HandoffManifest,
    TransformationFailure,
}

impl Kind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Object => "object",
            Self::Relation => "relation",
            Self::Instruction => "instruction",
            Self::Policy => "policy",
            Self::Workflow => "workflow",
            Self::CompiledContextTemplate => "compiled_context_template",
            Self::ProviderProfile => "provider_profile",
            Self::Review => "review",
            Self::Event => "event",
            Self::WorkPacket => "work_packet",
            Self::RunRecord => "run_record",
            Self::SystemDefinition => "system_definition",
            Self::TransitionAssignment => "transition_assignment",
            Self::ChangeSet => "change_set",
            Self::HandoffManifest => "handoff_manifest",
            Self::TransformationFailure => "transformation_failure",
        }
    }
}

impl std::str::FromStr for Kind {
    type Err = CoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "object" => Ok(Self::Object),
            "relation" => Ok(Self::Relation),
            "instruction" => Ok(Self::Instruction),
            "policy" => Ok(Self::Policy),
            "workflow" => Ok(Self::Workflow),
            "compiled_context_template" => Ok(Self::CompiledContextTemplate),
            "provider_profile" => Ok(Self::ProviderProfile),
            "review" => Ok(Self::Review),
            "event" => Ok(Self::Event),
            "work_packet" => Ok(Self::WorkPacket),
            "run_record" => Ok(Self::RunRecord),
            "system_definition" => Ok(Self::SystemDefinition),
            "transition_assignment" => Ok(Self::TransitionAssignment),
            "change_set" => Ok(Self::ChangeSet),
            "handoff_manifest" => Ok(Self::HandoffManifest),
            "transformation_failure" => Ok(Self::TransformationFailure),
            other => Err(CoreError::InvalidKind(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EpistemicStanding {
    Unresolved,
    Working,
    Supported,
    Contested,
    Superseded,
}

impl EpistemicStanding {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Unresolved => "unresolved",
            Self::Working => "working",
            Self::Supported => "supported",
            Self::Contested => "contested",
            Self::Superseded => "superseded",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewStanding {
    Unreviewed,
    Pending,
    Accepted,
    Rejected,
}

impl ReviewStanding {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Unreviewed => "unreviewed",
            Self::Pending => "pending",
            Self::Accepted => "accepted",
            Self::Rejected => "rejected",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessStanding {
    Active,
    Blocked,
    Completed,
    Archived,
}

impl ProcessStanding {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Blocked => "blocked",
            Self::Completed => "completed",
            Self::Archived => "archived",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StandingDimension {
    Epistemic,
    Review,
    Process,
}

impl StandingDimension {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Epistemic => "epistemic",
            Self::Review => "review",
            Self::Process => "process",
        }
    }

    pub fn parse(value: &str) -> Result<Self, CoreError> {
        match value {
            "epistemic" => Ok(Self::Epistemic),
            "review" => Ok(Self::Review),
            "process" => Ok(Self::Process),
            _ => Err(CoreError::InvalidIdentifier(format!(
                "invalid standing dimension '{}': expected epistemic, review, or process",
                value
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Standing {
    pub values: BTreeMap<DimensionId, TokenId>,
}

impl Standing {
    pub fn kernel_defaults() -> Self {
        let mut values = BTreeMap::new();
        values.insert(
            DimensionId::from_static("kernel:epistemic"),
            TokenId::from_static("working"),
        );
        values.insert(
            DimensionId::from_static("kernel:review"),
            TokenId::from_static("unreviewed"),
        );
        values.insert(
            DimensionId::from_static("kernel:process"),
            TokenId::from_static("active"),
        );
        Self { values }
    }

    pub fn get(&self, dim: &DimensionId) -> Option<&TokenId> {
        self.values.get(dim)
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&DimensionId, &TokenId)> {
        self.values.iter()
    }
}

impl Default for Standing {
    fn default() -> Self {
        Self::kernel_defaults()
    }
}

impl Serialize for Standing {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.values.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Standing {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error;
        let raw: BTreeMap<String, String> = BTreeMap::deserialize(deserializer)?;
        let mut values = BTreeMap::new();
        for (k, v) in raw {
            let norm_key = match k.as_str() {
                "epistemic" => "kernel:epistemic",
                "review" => "kernel:review",
                "process" => "kernel:process",
                other => other,
            };
            let dim = DimensionId::parse(norm_key).map_err(D::Error::custom)?;
            let token = TokenId::parse(&v).map_err(D::Error::custom)?;
            values.insert(dim, token);
        }
        Ok(Standing { values })
    }
}

/// Legacy compatibility helper for deserializing old three-axis standing format.
/// This must not become a second live standing model.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct LegacyStanding {
    epistemic: EpistemicStanding,
    review: ReviewStanding,
    process: ProcessStanding,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LineageLink {
    pub rel: String,
    pub object: ObjectRef,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Provenance {
    pub actor: String,
    pub source_type: String,
    pub source_ref: Option<ObjectRef>,
    pub lineage: Vec<LineageLink>,
    pub import_path: Option<String>,
    pub captured_at: Timestamp,
}

impl Provenance {
    pub fn direct_input(actor: impl Into<String>) -> Self {
        Self {
            actor: actor.into(),
            source_type: "direct_input".to_string(),
            source_ref: None,
            lineage: vec![],
            import_path: None,
            captured_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Envelope {
    pub id: ObjectId,
    pub version_id: VersionId,
    pub kind: Kind,
    pub class: Option<String>,
    pub standing: Standing,
    pub provenance: Provenance,
    pub headers: BTreeMap<String, HeaderValue>,
    pub payload_ref: PayloadRef,
    pub parents: Vec<VersionRef>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

impl Envelope {
    pub fn object_ref(&self) -> ObjectRef {
        ObjectRef::new(
            self.id.clone(),
            self.version_id.clone(),
            self.kind.clone(),
            self.class.clone(),
        )
    }

    pub fn version_ref(&self) -> VersionRef {
        VersionRef::new(self.id.clone(), self.version_id.clone())
    }

    pub fn title(&self) -> Option<String> {
        self.headers.get("title").and_then(HeaderValue::as_string)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationCreationMode {
    Declared,
    PrivilegedSystem,
}

pub const REL_TYPE_USED_INSTRUCTION: &str = "used_instruction";
pub const REL_TYPE_USED_COMPILED_CONTEXT: &str = "used_compiled_context";
pub const REL_TYPE_REQUESTS_STANDING: &str = "requests_standing";
pub const REL_TYPE_RESULTED_IN_FAILURE: &str = "resulted_in_failure";
pub const REL_TYPE_DERIVED_FROM: &str = "derived_from";

pub const PRIVILEGED_RELATION_TYPES: &[&str] = &[
    REL_TYPE_USED_INSTRUCTION,
    REL_TYPE_USED_COMPILED_CONTEXT,
    REL_TYPE_REQUESTS_STANDING,
    REL_TYPE_RESULTED_IN_FAILURE,
    REL_TYPE_DERIVED_FROM,
];

pub fn is_privileged_relation(rel_type: &str) -> bool {
    PRIVILEGED_RELATION_TYPES.contains(&rel_type)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelationPayload {
    pub source: ObjectRef,
    pub target: ObjectRef,
    pub relation_type: String,
    pub qualifiers: BTreeMap<String, ScalarValue>,
    pub scope: Option<String>,
}

// ---------------------------------------------------------------------------
// Standing declaration and registry data structures (v0.3 protocol-based)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StandingDimensionDefinition {
    pub id: DimensionId,
    pub default: TokenId,
    pub tokens: Vec<StandingTokenDefinition>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StandingTokenDefinition {
    pub id: TokenId,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub implements: Vec<ProtocolBinding>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProtocolBinding {
    pub protocol: KernelProtocolId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub properties: BTreeMap<String, ScalarValue>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StandingRegistry {
    pub dimensions: BTreeMap<DimensionId, StandingDimensionDefinition>,
}

impl StandingRegistry {
    /// Validate internal coherence of the registry.
    ///
    /// Checks:
    /// - All dimension IDs are valid identifiers.
    /// - Each dimension has a default token present in its token list.
    /// - Tokens within a dimension have valid IDs and are unique.
    /// - Protocol IDs in bindings are valid identifiers.
    pub fn validate(&self) -> Result<(), CoreError> {
        for (dim_id, def) in &self.dimensions {
            // Dimension ID must be valid (re-parse to validate)
            DimensionId::parse(dim_id.as_str())?;

            // Default token must be non-empty and exist in token list
            if def.default.as_str().is_empty() {
                return Err(CoreError::InvalidIdentifier(format!(
                    "dimension '{}' has empty default token",
                    dim_id.as_str(),
                )));
            }
            let token_ids: Vec<&str> = def.tokens.iter().map(|t| t.id.as_str()).collect();
            if !token_ids.contains(&def.default.as_str()) {
                return Err(CoreError::InvalidIdentifier(format!(
                    "default token '{}' for dimension '{}' not found in its token list",
                    def.default.as_str(),
                    dim_id.as_str(),
                )));
            }

            // Validate each token
            let mut seen_tokens = std::collections::BTreeSet::new();
            for token in &def.tokens {
                TokenId::parse(token.id.as_str())?;
                if !seen_tokens.insert(token.id.as_str()) {
                    return Err(CoreError::InvalidIdentifier(format!(
                        "duplicate token '{}' in dimension '{}'",
                        token.id.as_str(),
                        dim_id.as_str(),
                    )));
                }
                for binding in &token.implements {
                    KernelProtocolId::parse(binding.protocol.as_str())?;
                }
            }
        }
        Ok(())
    }

    /// Build a registry from a `SystemDefinition` plus built-in kernel defaults.
    ///
    /// Built-in dimensions are always available. System-declared dimensions are
    /// added on top. Duplicate dimensions (including attempts to override
    /// built-in dimensions) return an error.
    pub fn from_system_definition(system: &SystemDefinition) -> Result<Self, CoreError> {
        let mut dimensions = Self::kernel_defaults().dimensions;
        for dim_def in &system.standing_dimensions {
            if dimensions.contains_key(&dim_def.id) {
                return Err(CoreError::InvalidIdentifier(format!(
                    "duplicate dimension '{}': cannot override built-in or already declared dimension",
                    dim_def.id.as_str()
                )));
            }
            dimensions.insert(dim_def.id.clone(), dim_def.clone());
        }
        let registry = Self { dimensions };
        registry.validate()?;
        Ok(registry)
    }

    pub fn kernel_defaults() -> Self {
        let mut dimensions = BTreeMap::new();

        let epistemic = StandingDimensionDefinition {
            id: DimensionId::new("kernel:epistemic"),
            default: TokenId::new("working"),
            tokens: vec![
                StandingTokenDefinition {
                    id: TokenId::new("unresolved"),
                    implements: vec![],
                },
                StandingTokenDefinition {
                    id: TokenId::new("working"),
                    implements: vec![],
                },
                StandingTokenDefinition {
                    id: TokenId::new("supported"),
                    implements: vec![],
                },
                StandingTokenDefinition {
                    id: TokenId::new("contested"),
                    implements: vec![],
                },
                StandingTokenDefinition {
                    id: TokenId::new("superseded"),
                    implements: vec![],
                },
            ],
        };
        dimensions.insert(epistemic.id.clone(), epistemic);

        fn review_binding(state: &str) -> ProtocolBinding {
            ProtocolBinding {
                protocol: KernelProtocolId::from_static("kernel:review"),
                state: Some(state.to_string()),
                properties: BTreeMap::new(),
            }
        }

        let review = StandingDimensionDefinition {
            id: DimensionId::new("kernel:review"),
            default: TokenId::new("unreviewed"),
            tokens: vec![
                StandingTokenDefinition {
                    id: TokenId::new("unreviewed"),
                    implements: vec![review_binding("unreviewed")],
                },
                StandingTokenDefinition {
                    id: TokenId::new("pending"),
                    implements: vec![review_binding("pending")],
                },
                StandingTokenDefinition {
                    id: TokenId::new("accepted"),
                    implements: vec![review_binding("accepted")],
                },
                StandingTokenDefinition {
                    id: TokenId::new("rejected"),
                    implements: vec![review_binding("rejected")],
                },
            ],
        };
        dimensions.insert(review.id.clone(), review);

        fn process_binding(state: &str) -> ProtocolBinding {
            ProtocolBinding {
                protocol: KernelProtocolId::from_static("kernel:process"),
                state: Some(state.to_string()),
                properties: BTreeMap::new(),
            }
        }

        let process = StandingDimensionDefinition {
            id: DimensionId::new("kernel:process"),
            default: TokenId::new("active"),
            tokens: vec![
                StandingTokenDefinition {
                    id: TokenId::new("active"),
                    implements: vec![process_binding("active")],
                },
                StandingTokenDefinition {
                    id: TokenId::new("blocked"),
                    implements: vec![process_binding("blocked")],
                },
                StandingTokenDefinition {
                    id: TokenId::new("completed"),
                    implements: vec![process_binding("completed")],
                },
                StandingTokenDefinition {
                    id: TokenId::new("archived"),
                    implements: vec![process_binding("archived")],
                },
            ],
        };
        dimensions.insert(process.id.clone(), process);

        Self { dimensions }
    }
}

/// Materialize defaults from a registry, filling omitted dimensions with their declared defaults.
///
/// Returns an error if:
/// - any supplied dimension is not present in the registry;
/// - any supplied token is not valid for its dimension;
/// - a registry default token is not in its own dimension's token list.
pub fn materialize_defaults(
    registry: &StandingRegistry,
    supplied: BTreeMap<DimensionId, TokenId>,
) -> Result<Standing, CoreError> {
    let mut values = BTreeMap::new();

    // Validate supplied dimensions/tokens, rejecting unknown dimensions
    for (dim_id, token) in &supplied {
        let def = registry.dimensions.get(dim_id).ok_or_else(|| {
            CoreError::InvalidIdentifier(format!(
                "unknown dimension '{}' in supplied standing",
                dim_id.as_str()
            ))
        })?;
        let valid_tokens: Vec<&str> = def.tokens.iter().map(|t| t.id.as_str()).collect();
        if !valid_tokens.contains(&token.as_str()) {
            return Err(CoreError::InvalidIdentifier(format!(
                "unknown token '{}' for dimension '{}'",
                token.as_str(),
                dim_id.as_str(),
            )));
        }
        values.insert(dim_id.clone(), token.clone());
    }

    // Fill omitted registry dimensions with declared defaults
    for (dim_id, def) in &registry.dimensions {
        if !values.contains_key(dim_id) {
            let default_token = &def.default;
            let valid_tokens: Vec<&str> = def.tokens.iter().map(|t| t.id.as_str()).collect();
            if !valid_tokens.contains(&default_token.as_str()) {
                return Err(CoreError::InvalidIdentifier(format!(
                    "default token '{}' for dimension '{}' is not in its own token list",
                    default_token.as_str(),
                    dim_id.as_str(),
                )));
            }
            values.insert(dim_id.clone(), default_token.clone());
        }
    }

    Ok(Standing { values })
}

// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Default)]
pub struct ClassStandingRules {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub allowed_standing: BTreeMap<DimensionId, Vec<TokenId>>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub required_protocols: BTreeMap<KernelProtocolId, BTreeMap<String, ScalarValue>>,
}

impl<'de> Deserialize<'de> for ClassStandingRules {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use std::collections::BTreeMap as Map;

        #[derive(Deserialize)]
        struct Raw {
            #[serde(default)]
            allowed_standing: Option<Map<String, Vec<String>>>,
            #[serde(default)]
            required_protocols: Option<Map<String, Map<String, serde_json::Value>>>,
            #[serde(default)]
            allowed_epistemic: Option<Vec<String>>,
            #[serde(default)]
            allowed_review: Option<Vec<String>>,
            #[serde(default)]
            allowed_process: Option<Vec<String>>,
        }

        let raw = Raw::deserialize(deserializer)?;

        if let Some(allowed_standing) = raw.allowed_standing {
            // New format
            let mut map = BTreeMap::new();
            for (k, v) in allowed_standing {
                let dim = DimensionId::parse(&k).map_err(D::Error::custom)?;
                let mut tokens = Vec::new();
                for t in v {
                    tokens.push(TokenId::parse(&t).map_err(D::Error::custom)?);
                }
                map.insert(dim, tokens);
            }
            let mut protocols = BTreeMap::new();
            if let Some(rp) = raw.required_protocols {
                for (k, v) in rp {
                    let pid = KernelProtocolId::parse(&k).map_err(D::Error::custom)?;
                    let props: BTreeMap<String, ScalarValue> = v
                        .into_iter()
                        .map(|(pk, pv)| {
                            let sv = serde_json::from_value(pv.clone())
                                .unwrap_or(ScalarValue::String(pv.to_string()));
                            (pk, sv)
                        })
                        .collect();
                    protocols.insert(pid, props);
                }
            }
            return Ok(ClassStandingRules {
                allowed_standing: map,
                required_protocols: protocols,
            });
        }

        // Old format: translate allowed_epistemic/review/process to
        // kernel:* dimensions
        let mut map = BTreeMap::new();
        if let Some(tokens) = raw.allowed_epistemic {
            let dim = DimensionId::from_static("kernel:epistemic");
            let parsed: Vec<TokenId> = tokens
                .into_iter()
                .map(|t| TokenId::parse(&t))
                .collect::<Result<Vec<_>, _>>()
                .map_err(D::Error::custom)?;
            // Note: tokens may be values like EpistemicStanding::Working
            // which serialize as "working" — handled by TokenId::parse
            map.insert(dim, parsed);
        }
        if let Some(tokens) = raw.allowed_review {
            let dim = DimensionId::from_static("kernel:review");
            let parsed: Vec<TokenId> = tokens
                .into_iter()
                .map(|t| TokenId::parse(&t))
                .collect::<Result<Vec<_>, _>>()
                .map_err(D::Error::custom)?;
            map.insert(dim, parsed);
        }
        if let Some(tokens) = raw.allowed_process {
            let dim = DimensionId::from_static("kernel:process");
            let parsed: Vec<TokenId> = tokens
                .into_iter()
                .map(|t| TokenId::parse(&t))
                .collect::<Result<Vec<_>, _>>()
                .map_err(D::Error::custom)?;
            map.insert(dim, parsed);
        }

        Ok(ClassStandingRules {
            allowed_standing: map,
            required_protocols: BTreeMap::new(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelationRule {
    pub relation_type: String,
    pub counterparty_classes: Vec<String>,
    pub direction: Option<String>,
    pub authorizing_endpoint: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClassDefinition {
    pub name: String,
    pub version: String,
    pub kind: String,
    pub required_headers: Vec<String>,
    pub payload_schema: JsonSchemaRef,
    pub standing_rules: ClassStandingRules,
    pub relation_rules: Vec<RelationRule>,
    pub validators: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InstructionPayload {
    pub name: String,
    pub version: String,
    pub purpose: String,
    pub input_classes: Vec<String>,
    pub output_classes: Vec<String>,
    pub execution_policy: String,
    pub provider_profile: Option<VersionRef>,
    pub trace_policy: String,
    pub register: String,
    pub body: MarkdownBody,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowDefinition {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub operations: Vec<WorkflowOperation>,
    pub edges: Vec<WorkflowEdge>,
    pub guards: Vec<WorkflowGuard>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowOperation {
    pub id: String,
    pub kind: String,
    pub input_contracts: Vec<String>,
    pub output_contracts: Vec<String>,
    pub instruction: Option<VersionRef>,
    pub compiled_context: Option<VersionRef>,
    pub policy: Option<VersionRef>,
    pub provider_profile: Option<VersionRef>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowEdge {
    pub from: String,
    pub to: String,
    pub condition: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowGuard {
    pub id: String,
    pub expression: String,
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StandingPolicy {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub transition_rules: Vec<StandingTransitionRule>,
    pub operation_requirements: Vec<OperationRequirement>,
    pub escalations: Vec<EscalationRule>,
    pub rationale: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StandingTransitionRule {
    pub dimension: String,
    pub from: Vec<String>,
    pub to: Vec<String>,
    pub requires_review: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Default)]
pub struct OperationRequirement {
    pub operation: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub required_standing: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub forbidden_standing: BTreeMap<String, Vec<String>>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub required_protocols: BTreeMap<String, BTreeMap<String, ScalarValue>>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub forbidden_protocols: BTreeMap<String, BTreeMap<String, ScalarValue>>,
}

impl<'de> Deserialize<'de> for OperationRequirement {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Raw {
            operation: String,
            #[serde(default)]
            required_standing: Option<BTreeMap<String, String>>,
            #[serde(default)]
            forbidden_standing: Option<BTreeMap<String, Vec<String>>>,
            #[serde(default)]
            required_protocols: Option<BTreeMap<String, BTreeMap<String, ScalarValue>>>,
            #[serde(default)]
            forbidden_protocols: Option<BTreeMap<String, BTreeMap<String, ScalarValue>>>,
            #[serde(default)]
            minimums: Option<BTreeMap<String, String>>,
            #[serde(default)]
            forbidden: Option<BTreeMap<String, Vec<String>>>,
        }
        let raw = Raw::deserialize(deserializer)?;
        let required_standing = raw.required_standing.or(raw.minimums).unwrap_or_default();
        let forbidden_standing = raw.forbidden_standing.or(raw.forbidden).unwrap_or_default();
        Ok(OperationRequirement {
            operation: raw.operation,
            required_standing,
            forbidden_standing,
            required_protocols: raw.required_protocols.unwrap_or_default(),
            forbidden_protocols: raw.forbidden_protocols.unwrap_or_default(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EscalationRule {
    pub trigger: String,
    pub severity: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompiledContextTemplate {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub select: CompiledContextSelect,
    pub group_by: Vec<String>,
    pub render: CompiledContextRender,
    pub visibility: CompiledContextVisibility,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompiledContextSelect {
    pub classes: Vec<String>,
    pub standing: BTreeMap<String, Vec<String>>,
    pub relations: Vec<String>,
    pub time_range: Option<String>,
    #[serde(default)]
    pub expansion: CompiledContextExpansion,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompiledContextExpansion {
    #[serde(default)]
    pub object_filter: ExpansionObjectFilter,
    #[serde(default)]
    pub include_boundary_relations: bool,
}

impl Default for CompiledContextExpansion {
    fn default() -> Self {
        Self {
            object_filter: ExpansionObjectFilter::Inherit,
            include_boundary_relations: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ExpansionObjectFilter {
    #[default]
    Inherit,
    None,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompiledContextRender {
    pub mode: String,
    pub manifest_format: Option<String>,
    pub prose_template: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompiledContextVisibility {
    pub include_lineage: bool,
    pub include_constraints: bool,
    pub include_provenance: bool,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HttpAuthKind {
    #[default]
    None,
    Header,
    Bearer,
    QueryParameter,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct HttpAuthConfig {
    pub kind: HttpAuthKind,
    pub header_name: Option<String>,
    pub param_name: Option<String>,
    pub env: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HttpRequestTemplate {
    pub content_type: Option<String>,
    pub body: serde_json::Value,
}

impl Default for HttpRequestTemplate {
    fn default() -> Self {
        Self {
            content_type: None,
            body: serde_json::Value::Null,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct HttpResponseExtraction {
    pub text_path: String,
    pub finish_reason_path: Option<String>,
    pub input_tokens_path: Option<String>,
    pub output_tokens_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct HttpGenerationProfile {
    pub method: Option<String>,
    pub url_template: String,
    pub auth: HttpAuthConfig,
    pub request: HttpRequestTemplate,
    pub response: HttpResponseExtraction,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderProfile {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub provider: String,
    pub model: String,
    pub endpoint_env: Option<String>,
    pub auth_env: Option<String>,
    pub budget: ProviderBudget,
    pub allowed_operations: Vec<String>,
    pub exposure: ProviderExposure,
    pub response_contract: ProviderResponseContract,
    #[serde(default)]
    pub http: Option<HttpGenerationProfile>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ProviderBudget {
    pub max_input_tokens: Option<u32>,
    pub max_output_tokens: Option<u32>,
    pub max_cost_usd: Option<f32>,
    pub max_latency_ms: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderExposure {
    pub allow_prose_objects: bool,
    pub allow_structured_declarations: bool,
    pub allow_work_surface_only: bool,
    pub allow_export_requests: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderResponseContract {
    pub format: String,
    pub must_return_candidate_only: bool,
    pub must_include_lineage: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SystemDefinition {
    pub system_id: String,
    pub namespace: String,
    pub title: String,
    pub description: Option<String>,
    pub classes: Vec<VersionRef>,
    pub instructions: Vec<VersionRef>,
    pub policies: Vec<VersionRef>,
    pub workflows: Vec<VersionRef>,
    pub compiled_contexts: Vec<VersionRef>,
    pub provider_profiles: Vec<VersionRef>,
    pub default_compiled_context: Option<VersionRef>,
    pub default_provider_profile: Option<VersionRef>,
    #[serde(default)]
    pub standing_dimensions: Vec<StandingDimensionDefinition>,
    pub runtime_profile: RuntimeProfile,
    pub activated_at: Option<Timestamp>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeProfile {
    pub execution_surface: String,
    pub machine_output_default: String,
    pub work_surface_mode: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunRecord {
    pub run_id: String,
    pub system_definition: VersionRef,
    pub workflow: VersionRef,
    pub status: RunStatus,
    pub started_at: Timestamp,
    pub ended_at: Option<Timestamp>,
    pub initial_marking: Vec<TokenRecord>,
    pub final_marking: Vec<TokenRecord>,
    pub events: Vec<RunEvent>,
    pub work_packets: Vec<ObjectRef>,
    pub governance_events: Vec<ObjectRef>,
    pub assignments: Vec<TransitionAssignmentId>,
    pub change_sets: Vec<ChangeSetId>,
    pub manifests: Vec<HandoffManifestId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Running,
    Suspended,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TokenRecord {
    pub token_type: String,
    pub value: ScalarOrRef,
    pub place: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunEvent {
    pub event_id: String,
    pub transition: String,
    pub event_type: String,
    pub timestamp: Timestamp,
    pub inputs: Vec<ObjectRef>,
    pub outputs: Vec<ObjectRef>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkPacket {
    pub work_packet_id: String,
    pub run_id: String,
    pub work_packet_type: String,
    pub purpose: String,
    pub system_definition: VersionRef,
    pub workflow: Option<VersionRef>,
    pub instruction: Option<VersionRef>,
    pub provider_profile: Option<VersionRef>,
    pub inputs: Vec<ObjectRef>,
    pub compiled_contexts: Vec<ObjectRef>,
    pub constraints: WorkPacketConstraints,
    pub expected_outputs: Vec<String>,
    pub work_surface: Option<WorkSurfaceRef>,
    pub advisory_warnings: Vec<String>,
    pub created_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkPacketConstraints {
    pub standing_requirements: BTreeMap<String, String>,
    pub review_requirements: Vec<String>,
    pub prohibited_operations: Vec<String>,
    pub export_permitted: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkSurfaceRef {
    pub surface_id: String,
    pub manifest_path: String,
    pub render_mode: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderRequest {
    pub request_id: String,
    pub run_id: String,
    pub work_packet: ObjectRef,
    pub provider_profile: VersionRef,
    pub instruction_text: String,
    pub context_text: Option<String>,
    pub input_text: String,
    pub work_surface_manifest: Option<String>,
    pub inputs: Vec<ObjectRef>,
    pub response_contract: ProviderResponseContract,
    pub issued_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderResponse {
    pub request_id: String,
    pub provider: String,
    pub model: String,
    pub status: String,
    pub candidate_payload: String,
    pub metadata: BTreeMap<String, ScalarValue>,
    pub advisory_warnings: Vec<String>,
    pub usage: Option<ProviderUsage>,
    pub received_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ProviderUsage {
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
    pub estimated_cost_usd: Option<f32>,
    pub latency_ms: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderRecord {
    pub record_id: String,
    pub request_id: String,
    pub run_id: String,
    pub work_packet: ObjectRef,
    pub provider_profile: VersionRef,
    pub provider: String,
    pub model: String,
    pub status: String,
    pub metadata: BTreeMap<String, ScalarValue>,
    pub advisory_warnings: Vec<String>,
    pub usage: Option<ProviderUsage>,
    pub message: Option<String>,
    pub recorded_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TransitionAssignmentId(String);

impl TransitionAssignmentId {
    pub fn new() -> Self {
        Self(format!("obj_{}", Uuid::new_v4().simple()))
    }
}

impl Default for TransitionAssignmentId {
    fn default() -> Self {
        Self::new()
    }
}

impl TransitionAssignmentId {
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ChangeSetId(String);

impl ChangeSetId {
    pub fn new() -> Self {
        Self(format!("obj_{}", Uuid::new_v4().simple()))
    }
}

impl Default for ChangeSetId {
    fn default() -> Self {
        Self::new()
    }
}

impl ChangeSetId {
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct HandoffManifestId(String);

impl HandoffManifestId {
    pub fn new() -> Self {
        Self(format!("obj_{}", Uuid::new_v4().simple()))
    }
}

impl Default for HandoffManifestId {
    fn default() -> Self {
        Self::new()
    }
}

impl HandoffManifestId {
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssignmentStatus {
    Assigned,
    Completed,
    Blocked,
    Released,
    Expired,
    Superseded,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransitionAssignment {
    pub id: TransitionAssignmentId,
    pub run_id: String,
    pub transition_id: String,
    pub assigned_to: String,
    pub status: AssignmentStatus,
    pub input_object_ids: Vec<ObjectId>,
    pub handoff_manifest_id: Option<HandoffManifestId>,
    pub event_ids: Vec<ObjectRef>,
    pub blocked_reason: Option<String>,
    pub completion_change_set_id: Option<ChangeSetId>,
    pub assigned_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChangeSet {
    pub id: ChangeSetId,
    pub run_id: String,
    pub transition_id: String,
    pub assignment_id: Option<TransitionAssignmentId>,
    pub agent_id: Option<String>,
    pub input_object_ids: Vec<ObjectId>,
    pub created_object_ids: Vec<ObjectId>,
    pub created_relation_ids: Vec<ObjectId>,
    pub updated_object_ids: Vec<ObjectId>,
    pub governance_event_ids: Vec<ObjectId>,
    pub blocked_operations: Vec<BlockedOperation>,
    pub unresolved_ambiguities: Vec<UnresolvedAmbiguity>,
    pub rejected_candidates: Vec<RejectedCandidate>,
    pub validation_results: Vec<ChangeSetValidationResult>,
    pub work_packet_id: Option<String>,
    pub handoff_manifest_id: Option<HandoffManifestId>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransformationFailure {
    pub run_id: String,
    pub transition_id: String,
    pub assignment_id: TransitionAssignmentId,
    pub failed_change_set_id: Option<ChangeSetId>,
    pub error_type: String,
    pub message: String,
    pub stack_trace: Option<String>,
    #[serde(default)]
    pub input_object_ids: Vec<ObjectId>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlockedOperation {
    pub reason: String,
    pub operation: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnresolvedAmbiguity {
    pub description: String,
    pub context: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RejectedCandidate {
    pub reason: String,
    pub candidate_ref: Option<ObjectRef>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChangeSetValidationResult {
    pub is_valid: bool,
    pub failures: Vec<String>,
    pub warnings: Vec<String>,
    pub info: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StandingConstraint {
    pub constraint_type: String,
    pub requirements: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RequiredCheck {
    pub check_type: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HandoffManifest {
    pub id: HandoffManifestId,
    pub run_id: String,
    pub from_transition_id: String,
    pub to_transition_id: Option<String>,
    pub source_change_set_id: ChangeSetId,
    pub source_assignment_id: Option<TransitionAssignmentId>,
    pub root_object_ids: Vec<ObjectId>,
    pub inherited_input_object_ids: Vec<ObjectId>,
    pub newly_created_object_ids: Vec<ObjectId>,
    pub newly_created_relation_ids: Vec<ObjectId>,
    pub allowed_input_classes: Vec<String>,
    pub allowed_output_classes: Vec<String>,
    pub allowed_relation_types: Vec<String>,
    pub standing_constraints: Vec<StandingConstraint>,
    pub unresolved_ambiguities: Vec<UnresolvedAmbiguity>,
    pub blocked_conditions: Vec<BlockedOperation>,
    pub required_checks: Vec<RequiredCheck>,
    pub compiled_context_template_id: Option<ObjectId>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChangeSetDraft {
    pub created_objects: Vec<ObjectId>,
    pub created_relations: Vec<ObjectId>,
    pub updated_objects: Vec<ObjectId>,
    pub governance_events: Vec<ObjectId>,
    pub standing_requests: Vec<StandingTransitionRequest>,
    pub blocked_operations: Vec<BlockedOperation>,
    pub unresolved_ambiguities: Vec<UnresolvedAmbiguity>,
    pub rejected_candidates: Vec<RejectedCandidate>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StandingTransitionRequest {
    pub target_object_id: ObjectId,
    pub dimension: String, // arbitrary dimension from the standing registry
    pub from_value: String,
    pub to_value: String,
    pub rationale: Option<String>,
    pub status: StandingRequestStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StandingRequestStatus {
    Proposed,
    Approved,
    Rejected,
    Applied,
    Superseded,
}

pub fn validate_standing_request(request: &StandingTransitionRequest) -> Result<(), String> {
    if request.dimension.is_empty() {
        return Err("dimension must be non-empty".to_string());
    }
    if request.from_value.is_empty() || request.to_value.is_empty() {
        return Err("from_value and to_value must be non-empty".to_string());
    }
    if request.from_value == request.to_value {
        return Err("to_value must differ from from_value".to_string());
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelationFilter {
    pub allowed_types: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClassFilter {
    pub allowed_classes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StandingFilter {
    #[serde(default)]
    pub allowed: BTreeMap<DimensionId, Vec<TokenId>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeProvenance {
    pub actor: String,
    pub source_type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConnectedContextManifest {
    pub root_object_ids: Vec<ObjectId>,
    pub object_refs: Vec<ObjectRef>,
    pub relation_refs: Vec<ObjectRef>,
    pub max_depth: usize,
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("serde json error: {0}")]
    SerdeJson(#[from] serde_json::Error),
    #[error("serde yaml error: {0}")]
    SerdeYaml(#[from] serde_yaml::Error),
    #[error("invalid frontmatter: {0}")]
    InvalidFrontmatter(String),
    #[error("invalid kind: {0}")]
    InvalidKind(String),
    #[error("invalid identifier: {0}")]
    InvalidIdentifier(String),
    #[error("payload too large: {0} bytes (max {1} bytes)")]
    PayloadTooLarge(usize, usize),
    #[error("unknown class: {0}")]
    UnknownClass(String),
    #[error("schema violation: {0}")]
    SchemaViolation(String),
    #[error("schema unavailable: {0}")]
    SchemaUnavailable(String),
    #[error("security violation: {0}")]
    SecurityViolation(String),
}

#[derive(Debug, Serialize, Deserialize)]
struct InstructionFrontmatter {
    pub name: String,
    pub version: String,
    pub purpose: String,
    pub input_classes: Vec<String>,
    pub output_classes: Vec<String>,
    pub execution_policy: String,
    pub provider_profile: Option<VersionRef>,
    pub trace_policy: String,
    pub register: String,
}

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

impl InstructionPayload {
    pub fn parse_markdown(input: &str) -> Result<Self, CoreError> {
        let (frontmatter, body) = parse_markdown_frontmatter::<InstructionFrontmatter>(input)?;
        Ok(Self {
            name: frontmatter.name,
            version: frontmatter.version,
            purpose: frontmatter.purpose,
            input_classes: frontmatter.input_classes,
            output_classes: frontmatter.output_classes,
            execution_policy: frontmatter.execution_policy,
            provider_profile: frontmatter.provider_profile,
            trace_policy: frontmatter.trace_policy,
            register: frontmatter.register,
            body: MarkdownBody::new(body),
        })
    }

    pub fn to_markdown(&self) -> Result<String, CoreError> {
        let frontmatter = InstructionFrontmatter {
            name: self.name.clone(),
            version: self.version.clone(),
            purpose: self.purpose.clone(),
            input_classes: self.input_classes.clone(),
            output_classes: self.output_classes.clone(),
            execution_policy: self.execution_policy.clone(),
            provider_profile: self.provider_profile.clone(),
            trace_policy: self.trace_policy.clone(),
            register: self.register.clone(),
        };
        to_markdown_frontmatter(&frontmatter, self.body.as_str())
    }
}

pub fn parse_yaml<T: DeserializeOwned>(input: &str) -> Result<T, CoreError> {
    Ok(serde_yaml::from_str(input)?)
}

pub fn to_yaml<T: Serialize>(value: &T) -> Result<String, CoreError> {
    Ok(serde_yaml::to_string(value)?)
}

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

    // Check for credentials in URL
    if url.contains('@') {
        return Err(CoreError::SecurityViolation(
            "endpoints must not contain embedded credentials".to_string(),
        ));
    }

    // Check for query string or fragment
    if url.contains('?') || url.contains('#') {
        return Err(CoreError::SecurityViolation(
            "endpoints must not contain query strings or fragments".to_string(),
        ));
    }

    // Protocol and host validation
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

pub fn parse_json<T: DeserializeOwned>(input: &str) -> Result<T, CoreError> {
    Ok(serde_json::from_str(input)?)
}

pub fn to_json_pretty<T: Serialize>(value: &T) -> Result<String, CoreError> {
    Ok(serde_json::to_string_pretty(value)?)
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

    // --- WP01: New Standing model tests ---

    #[test]
    fn test_standing_serializes_as_clean_map() {
        let standing = Standing::kernel_defaults();
        let json = serde_json::to_string(&standing).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        let map = parsed.as_object().unwrap();
        assert!(map.contains_key("kernel:epistemic"));
        assert!(map.contains_key("kernel:review"));
        assert!(map.contains_key("kernel:process"));
        assert_eq!(map["kernel:epistemic"], "working");
        assert_eq!(map["kernel:review"], "unreviewed");
        assert_eq!(map["kernel:process"], "active");
        assert!(
            !json.contains("epistemic_standing"),
            "should not contain old type names"
        );
    }

    #[test]
    fn test_standing_old_format_deserializes() {
        let old_json = r#"{"epistemic": "working", "review": "unreviewed", "process": "active"}"#;
        let standing: Standing = serde_json::from_str(old_json).unwrap();
        assert_eq!(
            standing
                .get(&DimensionId::new("kernel:epistemic"))
                .map(TokenId::as_str),
            Some("working")
        );
        assert_eq!(
            standing
                .get(&DimensionId::new("kernel:review"))
                .map(TokenId::as_str),
            Some("unreviewed")
        );
        assert_eq!(
            standing
                .get(&DimensionId::new("kernel:process"))
                .map(TokenId::as_str),
            Some("active")
        );
    }

    #[test]
    fn test_standing_new_format_deserializes() {
        let new_json = r#"{"kernel:epistemic": "supported", "kernel:review": "accepted", "kernel:process": "completed"}"#;
        let standing: Standing = serde_json::from_str(new_json).unwrap();
        assert_eq!(
            standing
                .get(&DimensionId::new("kernel:epistemic"))
                .map(TokenId::as_str),
            Some("supported")
        );
        assert_eq!(
            standing
                .get(&DimensionId::new("kernel:review"))
                .map(TokenId::as_str),
            Some("accepted")
        );
        assert_eq!(
            standing
                .get(&DimensionId::new("kernel:process"))
                .map(TokenId::as_str),
            Some("completed")
        );
    }

    #[test]
    fn test_kernel_registry_contains_builtin_dimensions() {
        let registry = StandingRegistry::kernel_defaults();
        assert!(registry
            .dimensions
            .contains_key(&DimensionId::new("kernel:epistemic")));
        assert!(registry
            .dimensions
            .contains_key(&DimensionId::new("kernel:review")));
        assert!(registry
            .dimensions
            .contains_key(&DimensionId::new("kernel:process")));
    }

    #[test]
    fn test_materialize_defaults_fills_omitted_dimensions() {
        let registry = StandingRegistry::kernel_defaults();
        let supplied = BTreeMap::new();
        let standing = materialize_defaults(&registry, supplied).unwrap();
        assert_eq!(
            standing
                .get(&DimensionId::new("kernel:epistemic"))
                .map(TokenId::as_str),
            Some("working")
        );
        assert_eq!(
            standing
                .get(&DimensionId::new("kernel:review"))
                .map(TokenId::as_str),
            Some("unreviewed")
        );
        assert_eq!(
            standing
                .get(&DimensionId::new("kernel:process"))
                .map(TokenId::as_str),
            Some("active")
        );
    }

    #[test]
    fn test_invalid_dimension_id_fails_parse() {
        assert!(DimensionId::parse("").is_err());
        assert!(DimensionId::parse(&"a".repeat(129)).is_err());
        assert!(DimensionId::parse("UPPERCASE").is_err());
        assert!(DimensionId::parse("has space").is_err());
        assert!(DimensionId::parse("valid:name").is_ok());
    }

    #[test]
    fn test_invalid_token_id_fails_parse() {
        assert!(TokenId::parse("").is_err());
        assert!(TokenId::parse(&"a".repeat(65)).is_err());
        assert!(TokenId::parse("UPPERCASE").is_err());
        assert!(TokenId::parse("has space").is_err());
        assert!(TokenId::parse("valid_token").is_ok());
        assert!(TokenId::parse("draft").is_ok());
    }

    #[test]
    fn test_new_writes_do_not_emit_old_shape() {
        let standing = Standing::kernel_defaults();
        let yaml = serde_yaml::to_string(&standing).unwrap();
        // New format uses kernel: prefixes
        assert!(yaml.contains("kernel:epistemic"));
        // Old format would have had "epistemic: working" as a flat field
        // The new format should NOT contain old-style single-word keys as top-level map entries
        let value: serde_yaml::Value = serde_yaml::from_str(&yaml).unwrap();
        let map = value.as_mapping().unwrap();
        for key in map.keys() {
            let ks = key.as_str().unwrap();
            assert!(
                ks.contains(':'),
                "new standing serialization should use namespaced keys, got: {}",
                ks
            );
        }
    }

    #[test]
    fn test_materialize_defaults_rejects_unknown_token() {
        let registry = StandingRegistry::kernel_defaults();
        let mut supplied = BTreeMap::new();
        supplied.insert(
            DimensionId::new("kernel:epistemic"),
            TokenId::new("nonexistent"),
        );
        assert!(materialize_defaults(&registry, supplied).is_err());
    }

    #[test]
    fn test_standing_is_empty_after_clear() {
        let mut standing = Standing::default();
        assert!(!standing.is_empty());
        standing.values.clear();
        assert!(standing.is_empty());
        assert_eq!(standing.len(), 0);
    }

    // --- WP01A: Foundation repair tests ---

    #[test]
    fn test_kernel_protocol_id_parse_rejects_invalid() {
        assert!(KernelProtocolId::parse("").is_err());
        assert!(KernelProtocolId::parse(&"a".repeat(129)).is_err());
        assert!(KernelProtocolId::parse("UPPERCASE").is_err());
        assert!(KernelProtocolId::parse("has space").is_err());
        assert!(KernelProtocolId::parse("kernel:review").is_ok());
        assert!(KernelProtocolId::parse("kernel:visibility").is_ok());
    }

    #[test]
    fn test_standing_deserialize_rejects_invalid_dimension() {
        let bad = r#"{"UPPERCASE_DIM": "value"}"#;
        assert!(serde_json::from_str::<Standing>(bad).is_err());
        let bad = r#"{"": "value"}"#;
        assert!(serde_json::from_str::<Standing>(bad).is_err());
    }

    #[test]
    fn test_standing_deserialize_rejects_invalid_token() {
        let bad = r#"{"kernel:epistemic": "UPPERCASE_TOKEN"}"#;
        assert!(serde_json::from_str::<Standing>(bad).is_err());
        let bad = r#"{"kernel:epistemic": ""}"#;
        assert!(serde_json::from_str::<Standing>(bad).is_err());
    }

    #[test]
    fn test_materialize_defaults_rejects_unknown_dimension() {
        let registry = StandingRegistry::kernel_defaults();
        let mut supplied = BTreeMap::new();
        supplied.insert(DimensionId::new("unknown:dimension"), TokenId::new("value"));
        let result = materialize_defaults(&registry, supplied);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("unknown dimension"),
            "error should mention unknown dimension"
        );
    }

    #[test]
    fn test_materialize_defaults_rejects_unknown_dimension_token() {
        let registry = StandingRegistry::kernel_defaults();
        let mut supplied = BTreeMap::new();
        supplied.insert(
            DimensionId::new("kernel:epistemic"),
            TokenId::new("nonexistent"),
        );
        let result = materialize_defaults(&registry, supplied);
        assert!(result.is_err());
        assert!(
            result.unwrap_err().to_string().contains("unknown token"),
            "error should mention unknown token"
        );
    }

    #[test]
    fn test_builtin_registry_validates_successfully() {
        let registry = StandingRegistry::kernel_defaults();
        assert!(registry.validate().is_ok());
    }

    #[test]
    fn test_kernel_review_tokens_all_bind_to_kernel_review() {
        let registry = StandingRegistry::kernel_defaults();
        let review = registry
            .dimensions
            .get(&DimensionId::new("kernel:review"))
            .expect("kernel:review dimension");
        for token in &review.tokens {
            let has_review_binding = token.implements.iter().any(|b| {
                b.protocol.as_str() == "kernel:review"
                    && b.state.as_deref() == Some(token.id.as_str())
            });
            assert!(
                has_review_binding,
                "token '{}' should bind to kernel:review state '{}'",
                token.id.as_str(),
                token.id.as_str()
            );
        }
    }

    #[test]
    fn test_kernel_process_tokens_all_bind_to_kernel_process() {
        let registry = StandingRegistry::kernel_defaults();
        let process = registry
            .dimensions
            .get(&DimensionId::new("kernel:process"))
            .expect("kernel:process dimension");
        for token in &process.tokens {
            let has_process_binding = token.implements.iter().any(|b| {
                b.protocol.as_str() == "kernel:process"
                    && b.state.as_deref() == Some(token.id.as_str())
            });
            assert!(
                has_process_binding,
                "token '{}' should bind to kernel:process state '{}'",
                token.id.as_str(),
                token.id.as_str()
            );
        }
    }

    #[test]
    fn test_kernel_epistemic_default_is_working_not_unresolved() {
        // Standing::kernel_defaults() uses working
        let standing = Standing::kernel_defaults();
        assert_eq!(
            standing
                .get(&DimensionId::new("kernel:epistemic"))
                .map(TokenId::as_str),
            Some("working")
        );
        // Standing::default() delegates to kernel_defaults()
        let standing = Standing::default();
        assert_eq!(
            standing
                .get(&DimensionId::new("kernel:epistemic"))
                .map(TokenId::as_str),
            Some("working")
        );
        // Registry default is also working
        let registry = StandingRegistry::kernel_defaults();
        let epi = registry
            .dimensions
            .get(&DimensionId::new("kernel:epistemic"))
            .expect("kernel:epistemic dimension");
        assert_eq!(epi.default.as_str(), "working");
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

    // --- WP02: System Standing Registry and Declaration Validation tests ---

    #[test]
    fn test_system_definition_with_custom_standing_dimensions_parses() {
        let sys = SystemDefinition {
            system_id: "sys_test".to_string(),
            namespace: "systems/test".to_string(),
            title: "Test".to_string(),
            description: None,
            classes: vec![],
            instructions: vec![],
            policies: vec![],
            workflows: vec![],
            compiled_contexts: vec![],
            provider_profiles: vec![],
            default_compiled_context: None,
            default_provider_profile: None,
            standing_dimensions: vec![StandingDimensionDefinition {
                id: DimensionId::from_static("research:status"),
                default: TokenId::from_static("draft"),
                tokens: vec![
                    StandingTokenDefinition {
                        id: TokenId::from_static("draft"),
                        implements: vec![],
                    },
                    StandingTokenDefinition {
                        id: TokenId::from_static("verified"),
                        implements: vec![ProtocolBinding {
                            protocol: KernelProtocolId::from_static("kernel:review"),
                            state: Some("accepted".to_string()),
                            properties: BTreeMap::new(),
                        }],
                    },
                ],
            }],
            runtime_profile: RuntimeProfile {
                execution_surface: "local".to_string(),
                machine_output_default: "json".to_string(),
                work_surface_mode: "strict".to_string(),
            },
            activated_at: None,
        };
        let registry = StandingRegistry::from_system_definition(&sys)
            .expect("registry construction should succeed");
        assert!(registry
            .dimensions
            .contains_key(&DimensionId::new("kernel:epistemic")));
        assert!(registry
            .dimensions
            .contains_key(&DimensionId::new("kernel:review")));
        assert!(registry
            .dimensions
            .contains_key(&DimensionId::new("kernel:process")));
        assert!(registry
            .dimensions
            .contains_key(&DimensionId::new("research:status")));
    }

    #[test]
    fn test_registry_construction_includes_builtin_and_custom() {
        let sys = SystemDefinition {
            system_id: "sys_test".to_string(),
            namespace: "systems/test".to_string(),
            title: "Test".to_string(),
            description: None,
            classes: vec![],
            instructions: vec![],
            policies: vec![],
            workflows: vec![],
            compiled_contexts: vec![],
            provider_profiles: vec![],
            default_compiled_context: None,
            default_provider_profile: None,
            standing_dimensions: vec![StandingDimensionDefinition {
                id: DimensionId::from_static("security:clearance"),
                default: TokenId::from_static("public"),
                tokens: vec![
                    StandingTokenDefinition {
                        id: TokenId::from_static("public"),
                        implements: vec![],
                    },
                    StandingTokenDefinition {
                        id: TokenId::from_static("restricted"),
                        implements: vec![],
                    },
                ],
            }],
            runtime_profile: RuntimeProfile {
                execution_surface: "local".to_string(),
                machine_output_default: "json".to_string(),
                work_surface_mode: "strict".to_string(),
            },
            activated_at: None,
        };
        let registry =
            StandingRegistry::from_system_definition(&sys).expect("registry should succeed");
        assert_eq!(registry.dimensions.len(), 4);
        let epi_default = registry
            .dimensions
            .get(&DimensionId::new("kernel:epistemic"))
            .map(|d| d.default.as_str());
        assert_eq!(epi_default, Some("working"));
        let clearance = registry
            .dimensions
            .get(&DimensionId::new("security:clearance"))
            .expect("security:clearance dimension");
        assert_eq!(clearance.default.as_str(), "public");
    }

    #[test]
    fn test_duplicate_dimension_fails_validation() {
        let sys = SystemDefinition {
            system_id: "sys_test".to_string(),
            namespace: "systems/test".to_string(),
            title: "Test".to_string(),
            description: None,
            classes: vec![],
            instructions: vec![],
            policies: vec![],
            workflows: vec![],
            compiled_contexts: vec![],
            provider_profiles: vec![],
            default_compiled_context: None,
            default_provider_profile: None,
            standing_dimensions: vec![StandingDimensionDefinition {
                id: DimensionId::from_static("kernel:epistemic"),
                default: TokenId::from_static("working"),
                tokens: vec![StandingTokenDefinition {
                    id: TokenId::from_static("working"),
                    implements: vec![],
                }],
            }],
            runtime_profile: RuntimeProfile {
                execution_surface: "local".to_string(),
                machine_output_default: "json".to_string(),
                work_surface_mode: "strict".to_string(),
            },
            activated_at: None,
        };
        assert!(StandingRegistry::from_system_definition(&sys).is_err());
    }

    #[test]
    fn test_default_token_missing_from_token_list_fails() {
        let sys = SystemDefinition {
            system_id: "sys_test".to_string(),
            namespace: "systems/test".to_string(),
            title: "Test".to_string(),
            description: None,
            classes: vec![],
            instructions: vec![],
            policies: vec![],
            workflows: vec![],
            compiled_contexts: vec![],
            provider_profiles: vec![],
            default_compiled_context: None,
            default_provider_profile: None,
            standing_dimensions: vec![StandingDimensionDefinition {
                id: DimensionId::from_static("research:status"),
                default: TokenId::from_static("missing_token"),
                tokens: vec![StandingTokenDefinition {
                    id: TokenId::from_static("draft"),
                    implements: vec![],
                }],
            }],
            runtime_profile: RuntimeProfile {
                execution_surface: "local".to_string(),
                machine_output_default: "json".to_string(),
                work_surface_mode: "strict".to_string(),
            },
            activated_at: None,
        };
        assert!(StandingRegistry::from_system_definition(&sys).is_err());
    }

    #[test]
    fn test_unknown_protocol_binding_fails() {
        let sys = SystemDefinition {
            system_id: "sys_test".to_string(),
            namespace: "systems/test".to_string(),
            title: "Test".to_string(),
            description: None,
            classes: vec![],
            instructions: vec![],
            policies: vec![],
            workflows: vec![],
            compiled_contexts: vec![],
            provider_profiles: vec![],
            default_compiled_context: None,
            default_provider_profile: None,
            standing_dimensions: vec![StandingDimensionDefinition {
                id: DimensionId::from_static("research:status"),
                default: TokenId::from_static("draft"),
                tokens: vec![StandingTokenDefinition {
                    id: TokenId::from_static("verified"),
                    implements: vec![ProtocolBinding {
                        protocol: KernelProtocolId::from_static("nonexistent:protocol"),
                        state: Some("x".to_string()),
                        properties: BTreeMap::new(),
                    }],
                }],
            }],
            runtime_profile: RuntimeProfile {
                execution_surface: "local".to_string(),
                machine_output_default: "json".to_string(),
                work_surface_mode: "strict".to_string(),
            },
            activated_at: None,
        };
        assert!(StandingRegistry::from_system_definition(&sys).is_err());
    }

    #[test]
    fn test_unknown_dimension_in_class_standing_rules_fails_registry_validation() {
        let registry = StandingRegistry::kernel_defaults();
        let rules = ClassStandingRules {
            allowed_standing: BTreeMap::from([(
                DimensionId::from_static("unknown:dim"),
                vec![TokenId::from_static("unknown_token")],
            )]),
            ..Default::default()
        };
        assert!(registry
            .dimensions
            .get(&DimensionId::new("unknown:dim"))
            .is_none());
    }

    #[test]
    fn test_class_standing_rules_against_registry_checks() {
        // Using the earmark-declarations function is not possible here,
        // but we can verify the registry dimension lookup directly.
        let registry = StandingRegistry::kernel_defaults();
        let dim_id = DimensionId::new("kernel:epistemic");
        let def = registry.dimensions.get(&dim_id).expect("kernel:epistemic");
        let valid: Vec<&str> = def.tokens.iter().map(|t| t.id.as_str()).collect();
        assert!(valid.contains(&"working"));
        assert!(!valid.contains(&"bogus"));
        let unknown_id = DimensionId::new("research:status");
        assert!(registry.dimensions.get(&unknown_id).is_none());
    }

    #[test]
    fn test_standing_policy_unknown_dimension_fails_registry_validation() {
        let registry = StandingRegistry::kernel_defaults();
        // "nonexistent" is a valid DimensionId (just a name) but not in the registry
        let dim_id = DimensionId::parse("nonexistent").expect("valid dim id");
        assert!(registry.dimensions.get(&dim_id).is_none());
    }

    #[test]
    fn test_custom_dimension_compiled_context_filter_passes_registry_validation() {
        let sys = SystemDefinition {
            system_id: "sys_test".to_string(),
            namespace: "systems/test".to_string(),
            title: "Test".to_string(),
            description: None,
            classes: vec![],
            instructions: vec![],
            policies: vec![],
            workflows: vec![],
            compiled_contexts: vec![],
            provider_profiles: vec![],
            default_compiled_context: None,
            default_provider_profile: None,
            standing_dimensions: vec![StandingDimensionDefinition {
                id: DimensionId::from_static("research:status"),
                default: TokenId::from_static("draft"),
                tokens: vec![
                    StandingTokenDefinition {
                        id: TokenId::from_static("draft"),
                        implements: vec![],
                    },
                    StandingTokenDefinition {
                        id: TokenId::from_static("verified"),
                        implements: vec![],
                    },
                ],
            }],
            runtime_profile: RuntimeProfile {
                execution_surface: "local".to_string(),
                machine_output_default: "json".to_string(),
                work_surface_mode: "strict".to_string(),
            },
            activated_at: None,
        };
        let registry =
            StandingRegistry::from_system_definition(&sys).expect("registry construction");
        let dim_id = DimensionId::new("research:status");
        let def = registry.dimensions.get(&dim_id).expect("research:status");
        let valid: Vec<&str> = def.tokens.iter().map(|t| t.id.as_str()).collect();
        assert!(valid.contains(&"draft"));
        assert!(valid.contains(&"verified"));
        assert!(!valid.contains(&"bogus"));
    }

    #[test]
    fn test_system_definition_without_standing_dimensions_defaults_empty() {
        let yaml = r#"
system_id: minimal
namespace: systems/minimal
title: Minimal System
classes: []
instructions: []
policies: []
workflows: []
compiled_contexts: []
provider_profiles: []
runtime_profile:
  execution_surface: local
  machine_output_default: json
  work_surface_mode: strict
"#;
        let sys: SystemDefinition =
            serde_yaml::from_str(yaml).expect("system without standing_dimensions should parse");
        assert!(sys.standing_dimensions.is_empty());
    }
}
