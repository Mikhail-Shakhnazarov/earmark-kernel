//! Workflow execution record types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use crate::ids::{
    ChangeSetId, HandoffManifestId, ObjectId, ObjectRef, RunId, TransitionAssignmentId,
    TransitionId, UndoRecordId, VersionRef,
};
use crate::standing::{StandingConstraint, StandingTransitionRequest};
use crate::values::{ScalarOrRef, Timestamp};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunRecord {
    pub run_id: RunId,
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
    Partial,
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
    pub transition: TransitionId,
    pub event_type: String,
    pub timestamp: Timestamp,
    pub inputs: Vec<ObjectRef>,
    pub outputs: Vec<ObjectRef>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkPacket {
    pub work_packet_id: String,
    pub run_id: RunId,
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
    pub run_id: RunId,
    pub transition_id: TransitionId,
    pub assigned_to: String,
    pub status: AssignmentStatus,
    #[serde(default)]
    pub input_object_ids: Vec<ObjectId>,
    pub handoff_manifest_id: Option<HandoffManifestId>,
    #[serde(default)]
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
    pub run_id: RunId,
    pub transition_id: TransitionId,
    pub assignment_id: Option<TransitionAssignmentId>,
    pub agent_id: Option<String>,
    #[serde(default)]
    pub input_object_ids: Vec<ObjectId>,
    #[serde(default)]
    pub created_object_ids: Vec<ObjectId>,
    #[serde(default)]
    pub created_relation_ids: Vec<ObjectId>,
    #[serde(default)]
    pub updated_object_ids: Vec<ObjectId>,
    #[serde(default)]
    pub governance_event_ids: Vec<ObjectId>,
    #[serde(default)]
    pub blocked_operations: Vec<BlockedOperation>,
    #[serde(default)]
    pub unresolved_ambiguities: Vec<UnresolvedAmbiguity>,
    #[serde(default)]
    pub rejected_candidates: Vec<RejectedCandidate>,
    #[serde(default)]
    pub validation_results: Vec<ChangeSetValidationResult>,
    pub work_packet_id: Option<String>,
    pub handoff_manifest_id: Option<HandoffManifestId>,
    pub created_at: DateTime<Utc>,
}


#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UndoRecord {
    pub id: UndoRecordId,
    pub target_run_id: RunId,
    pub reverted_change_set_ids: Vec<ChangeSetId>,
    pub created_object_ids: Vec<ObjectId>,
    pub created_relation_ids: Vec<ObjectId>,
    pub restored_heads: Vec<VersionRef>,
    pub reason: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransformationFailure {
    pub run_id: RunId,
    pub transition_id: TransitionId,
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
pub struct RequiredCheck {
    pub check_type: String,
    pub description: String,
}


#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HandoffManifest {
    pub id: HandoffManifestId,
    pub run_id: RunId,
    pub from_transition_id: TransitionId,
    pub to_transition_id: Option<TransitionId>,
    pub source_change_set_id: ChangeSetId,
    pub source_assignment_id: Option<TransitionAssignmentId>,
    #[serde(default)]
    pub root_object_ids: Vec<ObjectId>,
    #[serde(default)]
    pub inherited_input_object_ids: Vec<ObjectId>,
    #[serde(default)]
    pub newly_created_object_ids: Vec<ObjectId>,
    #[serde(default)]
    pub newly_created_relation_ids: Vec<ObjectId>,
    #[serde(default)]
    pub allowed_input_classes: Vec<String>,
    #[serde(default)]
    pub allowed_output_classes: Vec<String>,
    #[serde(default)]
    pub allowed_relation_types: Vec<String>,
    #[serde(default)]
    pub standing_constraints: Vec<StandingConstraint>,
    #[serde(default)]
    pub unresolved_ambiguities: Vec<UnresolvedAmbiguity>,
    #[serde(default)]
    pub blocked_conditions: Vec<BlockedOperation>,
    #[serde(default)]
    pub required_checks: Vec<RequiredCheck>,
    pub compiled_context_template_id: Option<ObjectId>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ChangeSetDraft {
    #[serde(default)]
    pub created_objects: Vec<ObjectId>,
    #[serde(default)]
    pub created_relations: Vec<ObjectId>,
    #[serde(default)]
    pub updated_objects: Vec<ObjectId>,
    #[serde(default)]
    pub governance_events: Vec<ObjectId>,
    #[serde(default)]
    pub standing_requests: Vec<StandingTransitionRequest>,
    #[serde(default)]
    pub blocked_operations: Vec<BlockedOperation>,
    #[serde(default)]
    pub unresolved_ambiguities: Vec<UnresolvedAmbiguity>,
    #[serde(default)]
    pub rejected_candidates: Vec<RejectedCandidate>,
}
