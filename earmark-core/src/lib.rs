use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Standing {
    pub epistemic: EpistemicStanding,
    pub review: ReviewStanding,
    pub process: ProcessStanding,
}

impl Default for Standing {
    fn default() -> Self {
        Self {
            epistemic: EpistemicStanding::Working,
            review: ReviewStanding::Unreviewed,
            process: ProcessStanding::Active,
        }
    }
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelationPayload {
    pub source: ObjectRef,
    pub target: ObjectRef,
    pub relation_type: String,
    pub qualifiers: BTreeMap<String, ScalarValue>,
    pub scope: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ClassStandingRules {
    pub allowed_epistemic: Vec<EpistemicStanding>,
    pub allowed_review: Vec<ReviewStanding>,
    pub allowed_process: Vec<ProcessStanding>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelationRule {
    pub relation_type: String,
    pub target_classes: Vec<String>,
    pub direction: Option<String>,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperationRequirement {
    pub operation: String,
    pub minimums: BTreeMap<String, String>,
    pub forbidden: BTreeMap<String, Vec<String>>,
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
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    pub usage: Option<ProviderUsage>,
    pub received_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    #[serde(default)]
    pub metadata: BTreeMap<String, ScalarValue>,
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
    pub dimension: String, // "epistemic", "review", or "process"
    pub from_value: String,
    pub to_value: String,
    pub rationale: Option<String>,
    pub status: StandingRequestStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StandingRequestStatus {
    Proposed,
    Accepted,
    Rejected,
}

pub fn validate_standing_request(request: &StandingTransitionRequest) -> Result<(), String> {
    let allowed_dimensions = ["epistemic", "review", "process"];
    if !allowed_dimensions.contains(&request.dimension.as_str()) {
        return Err(format!("invalid dimension: {}", request.dimension));
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
    pub allowed_epistemic: Vec<EpistemicStanding>,
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
}
