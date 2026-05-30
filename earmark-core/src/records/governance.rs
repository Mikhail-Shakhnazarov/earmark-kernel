/*
 * Copyright (c) 2026 Mikhail Shakhnazarov. Dual-licensed under AGPL-3.0-or-later or commercial terms.
 * PROPRIETARY AND INTERNAL. ONLY LOCALLY COMMITTED.
 * v0.1_internal kernel.
 */

use crate::ids::{ActorId, CheckResultId, ObjectRef, ReviewId, ReviewTargetRef, StandingTargetRef};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReviewRecord {
    pub review_id: ReviewId,
    pub target_ref: ReviewTargetRef,
    pub reviewer_ref: ActorId,
    pub judgment: ReviewJudgment,
    pub rationale: String,
    pub check_result_refs: Vec<CheckResultId>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewJudgment {
    Accept,
    Reject,
    NeedsRevision,
    CommentOnly,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StandingTransitionRequest {
    pub target_ref: StandingTargetRef,
    pub dimension: String,
    pub from_token: Option<String>,
    pub to_token: String,
    pub requested_by: ActorId,
    pub authority_ref: Option<ObjectRef>,
    pub rationale: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StandingTransitionRecord {
    pub transition_record_id: String,
    pub target_ref: StandingTargetRef,
    pub dimension: String,
    pub from_token: Option<String>,
    pub to_token: String,
    pub authorized_by: ActorId,
    pub authority_ref: Option<ObjectRef>,
    pub rationale: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GovernanceEvent {
    pub event_id: String,
    pub event_type: String,
    pub target_ref: String,
    pub actor_ref: ActorId,
    pub rationale: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PackActivationRecord {
    pub pack_id: crate::ids::SystemPackId,
    pub status: PackActivationStatus,
    pub actor_ref: ActorId,
    pub rationale: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackActivationStatus {
    Active,
    Inactive,
    Deprecated,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UndoRecord {
    pub undo_id: String,
    pub target_ref: crate::ids::ObjectRef,
    pub original_version: String,
    pub reverted_by: ActorId,
    pub rationale: String,
    pub created_at: DateTime<Utc>,
}
