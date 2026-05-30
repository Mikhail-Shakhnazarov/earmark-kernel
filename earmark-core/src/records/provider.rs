/*
 * Copyright (c) 2026 Mikhail Shakhnazarov. Dual-licensed under AGPL-3.0-or-later or commercial terms.
 * PROPRIETARY AND INTERNAL. ONLY LOCALLY COMMITTED.
 * v0.1_internal kernel.
 */

use crate::ids::{
    ActorId, DispatchId, ExternalConnectionId, ObjectId, PacketId, ProviderProfileId,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderProfile {
    pub provider_profile_id: ProviderProfileId,
    pub provider_type: ProviderType,
    pub model_or_endpoint: String,
    pub auth_reference: AuthReference,
    pub budget: ProviderBudget,
    pub content_constraints: Vec<String>,
    pub exposure_constraints: Vec<String>,
    pub capabilities: Vec<String>,
    pub logging_policy: ProviderLoggingPolicy,
    pub failure_policy: ProviderFailurePolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderType {
    LocalMock,
    HttpGeneration,
    OpenAiCompatible,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderRecord {
    pub provider_record_id: String,
    pub provider_profile_ref: ProviderProfileId,
    pub dispatch_id: DispatchId,
    pub packet_id: PacketId,
    pub request_metadata: serde_json::Value,
    pub response_metadata: serde_json::Value,
    pub status: ProviderCallStatus,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderCallStatus {
    NotCalled,
    Succeeded,
    Failed,
    TimedOut,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExternalConnectionRecord {
    pub connection_id: ExternalConnectionId,
    pub source_object_id: ObjectId,
    pub relation_type: String,
    pub external_type: ExternalType,
    pub external_ref: String,
    pub external_location: Option<String>,
    pub evidence_level: EvidenceLevel,
    pub cached_metadata: serde_json::Value,
    pub created_by: ActorId,
    pub created_at: DateTime<Utc>,
    pub last_verified_at: Option<DateTime<Utc>>,
    pub verification_status: VerificationStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalType {
    GitCommit,
    PullRequest,
    Issue,
    File,
    Document,
    Dataset,
    EmailThread,
    Publication,
    ReleaseArtifact,
    ProviderTranscript,
    Url,
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceLevel {
    Direct,
    Indirect,
    Contextual,
    Unverified,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationStatus {
    Verified,
    Unverified,
    Broken,
    Inaccessible,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthReference {
    pub kind: AuthKind,
    pub env_var: Option<String>,
    pub secret_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthKind {
    None,
    EnvVar,
    BearerToken,
    ApiKey,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderBudget {
    pub max_input_tokens: Option<u64>,
    pub max_output_tokens: Option<u64>,
    pub max_cost_units: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderLoggingPolicy {
    MetadataOnly,
    FullRequestResponse,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderFailurePolicy {
    FailDispatch,
    RecordCheckResult,
    Retry { max_attempts: u8 },
}
