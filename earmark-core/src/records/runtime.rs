/*
 * Copyright (c) 2026 Mikhail Shakhnazarov. Dual-licensed under AGPL-3.0-or-later or commercial terms.
 * PROPRIETARY AND INTERNAL. ONLY LOCALLY COMMITTED.
 * v0.1_internal kernel.
 */

use crate::ids::{
    ActorId, ChangeSetId, CheckResultId, ClassId, DispatchId, HandoffManifestId, ObjectId,
    ObjectRef, PacketId, PacketTemplateId, ProviderProfileId, RelationId, RunId, RuntimeProtocolId,
    SelectionPolicyId, SystemId, SystemPackId, TransitionId, WorkerProfileId, WorkflowId,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunRecord {
    pub run_id: RunId,
    pub workflow_id: Option<WorkflowId>,
    pub status: RunStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Scheduled,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PacketRecord {
    pub packet_id: PacketId,
    pub system_pack_ref: SystemPackId,
    pub system_ref: SystemId,
    pub run_id: RunId,
    pub workflow_ref: WorkflowId,
    pub transition_id: TransitionId,
    pub packet_template_ref: PacketTemplateId,
    pub root_object_ids: Vec<ObjectId>,
    pub included_object_refs: Vec<ObjectRef>,
    pub excluded_object_refs: Vec<ObjectRef>,
    pub exclusion_reasons: Vec<String>,
    pub relation_traversal_trace: Vec<TraversalTraceEntry>,
    pub standing_filter_trace: Vec<StandingFilterTraceEntry>,
    pub redaction_trace: Vec<RedactionTraceEntry>,
    pub provider_exposure_trace: Vec<ProviderExposureTraceEntry>,
    pub instruction_ref: Option<ObjectRef>,
    pub protocol_ref: RuntimeProtocolId,
    pub selection_ref: Option<SelectionPolicyId>,
    pub provider_profile_ref: Option<ProviderProfileId>,
    #[serde(default)]
    pub worker_profile_ref: Option<WorkerProfileId>,
    pub output_contract_ref: ClassId,
    pub rendered_manifest: Option<String>,
    #[serde(default)]
    pub selection_trace: Vec<SelectionTraceEntry>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TraversalTraceEntry {
    pub object_ref: ObjectRef,
    pub depth: u32,
    pub relation_id: Option<RelationId>,
    pub relation_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StandingFilterTraceEntry {
    pub object_id: ObjectId,
    pub dimension: String,
    pub token: String,
    pub filter_match: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RedactionTraceEntry {
    pub object_ref: ObjectRef,
    pub rule_id: String,
    pub fully_redacted: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderExposureTraceEntry {
    pub object_ref: ObjectRef,
    pub rule_id: String,
    pub allowed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelectionTraceEntry {
    pub selection_id: SelectionPolicyId,
    pub candidate_worker_id: Option<WorkerProfileId>,
    pub candidate_provider_id: Option<ProviderProfileId>,
    pub decision: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DispatchRecord {
    pub dispatch_id: DispatchId,
    pub run_id: RunId,
    pub transition_id: TransitionId,
    pub packet_id: PacketId,
    pub actor_ref: Option<ActorId>,
    pub provider_ref: Option<ProviderProfileId>,
    pub worker_profile_ref: Option<crate::ids::WorkerProfileId>,
    pub status: DispatchStatus,
    pub input_object_ids: Vec<ObjectId>,
    pub candidate_refs: Vec<ObjectRef>,
    pub check_result_ids: Vec<CheckResultId>,
    pub completion_change_set_id: Option<ChangeSetId>,
    pub handoff_manifest_id: Option<HandoffManifestId>,
    pub blocked_reason: Option<String>,
    pub claimed_by: Option<ActorId>,
    pub claimed_at: Option<DateTime<Utc>>,
    pub lease_expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DispatchStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChangeSetRecord {
    pub change_set_id: ChangeSetId,
    pub run_id: RunId,
    pub transition_id: TransitionId,
    pub dispatch_id: Option<DispatchId>,
    pub agent_or_actor_ref: Option<ActorId>,
    pub input_object_ids: Vec<ObjectId>,
    pub created_object_ids: Vec<ObjectId>,
    pub created_relation_ids: Vec<RelationId>,
    pub updated_object_ids: Vec<ObjectId>,
    pub blocked_transitions: Vec<BlockedTransition>,
    pub unresolved_ambiguities: Vec<UnresolvedAmbiguity>,
    pub rejected_candidates: Vec<RejectedCandidate>,
    pub packet_id: Option<PacketId>,
    pub handoff_manifest_id: Option<HandoffManifestId>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CheckResultRecord {
    pub check_result_id: CheckResultId,
    pub run_id: Option<RunId>,
    pub transition_id: Option<TransitionId>,
    pub dispatch_id: Option<DispatchId>,
    pub validator_id: Option<ObjectId>,
    pub check_type: String,
    pub status: CheckStatus,
    pub message: Option<String>,
    pub target_change_set_id: Option<ChangeSetId>,
    pub input_object_ids: Vec<ObjectId>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckStatus {
    Passed,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HandoffManifestRecord {
    pub handoff_manifest_id: HandoffManifestId,
    pub run_id: RunId,
    pub from_transition_id: TransitionId,
    pub to_transition_id: Option<TransitionId>,
    pub source_dispatch_id: DispatchId,
    pub source_change_set_id: ChangeSetId,
    pub root_object_ids: Vec<ObjectId>,
    pub newly_created_object_ids: Vec<ObjectId>,
    pub newly_created_relation_ids: Vec<RelationId>,
    pub consumed_at: Option<DateTime<Utc>>,
    pub consuming_dispatch_id: Option<DispatchId>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlockedTransition {
    pub transition_id: TransitionId,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnresolvedAmbiguity {
    pub description: String,
    pub context: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RejectedCandidate {
    pub candidate_ref: Option<ObjectRef>,
    pub reason: String,
}
