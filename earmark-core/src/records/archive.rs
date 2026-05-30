/*
 * Copyright (c) 2026 Mikhail Shakhnazarov. Dual-licensed under AGPL-3.0-or-later or commercial terms.
 * PROPRIETARY AND INTERNAL. ONLY LOCALLY COMMITTED.
 * v0.1_internal kernel.
 */

use crate::records::{
    ChangeSetRecord, DispatchRecord, HandoffManifestRecord, MigrationRecord, ObjectRecord,
    PacketRecord, RelationRecord, ReviewRecord, RunRecord, StandingTransitionRecord,
    SystemPackManifest, VersionRecord,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceArchive {
    pub objects: Vec<(ObjectRecord, Vec<VersionRecord>)>,
    pub relations: Vec<RelationRecord>,
    pub runs: Vec<RunRecord>,
    pub packets: Vec<PacketRecord>,
    pub dispatches: Vec<DispatchRecord>,
    pub change_sets: Vec<ChangeSetRecord>,
    pub handoffs: Vec<HandoffManifestRecord>,
    pub reviews: Vec<ReviewRecord>,
    pub standing: Vec<StandingTransitionRecord>,
    pub system_packs: Vec<SystemPackManifest>,
    pub classes: Vec<crate::records::ClassDeclaration>,
    pub systems: Vec<crate::records::SystemDeclaration>,
    pub workflows: Vec<crate::records::WorkflowDeclaration>,
    pub migrations: Vec<crate::records::MigrationRecord>,
    pub exported_at: DateTime<Utc>,
    pub protocol_version: String,
}
