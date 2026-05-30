/*
 * Copyright (c) 2026 Mikhail Shakhnazarov. Dual-licensed under AGPL-3.0-or-later or commercial terms.
 * PROPRIETARY AND INTERNAL. ONLY LOCALLY COMMITTED.
 * v0.1_internal kernel.
 */

use chrono::{DateTime, Utc};
use earmark_core::ActorId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LedgerRecord {
    pub transaction_id: String,
    pub operation: String,
    pub actor: ActorId,
    pub input_refs: Vec<String>,
    pub created_refs: Vec<String>,
    pub updated_refs: Vec<String>,
    pub check_result_ids: Vec<String>,
    pub timestamp: DateTime<Utc>,
    pub failure_state: Option<String>,
}
