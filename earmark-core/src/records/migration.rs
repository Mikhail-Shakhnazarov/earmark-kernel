/*
 * Copyright (c) 2026 Mikhail Shakhnazarov.
 * Dual-licensed under AGPL-3.0-or-later or commercial terms.
 */

use crate::ids::ActorId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MigrationRecord {
    pub migration_id: String,
    pub from_version: String,
    pub to_version: String,
    pub applied_by: ActorId,
    pub strategy: MigrationStrategy,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MigrationStrategy {
    Implicit,
    ExplicitScript,
    ManualOverride,
}
