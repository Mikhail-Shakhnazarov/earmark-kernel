/*
 * Copyright (c) 2026 Mikhail Shakhnazarov. Dual-licensed under AGPL-3.0-or-later or commercial terms.
 * PROPRIETARY AND INTERNAL. ONLY LOCALLY COMMITTED.
 * v0.1_internal kernel.
 */

use crate::ids::WorkerProfileId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkerProfile {
    pub worker_profile_id: WorkerProfileId,
    pub adapter_kind: AdapterKind,
    pub title: String,
    pub description: String,
    pub config: serde_json::Value,
    pub capabilities: Vec<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdapterKind {
    OpenCode,
    Gemini,
    GeminiApi,
    MockWorker,
    Custom(String),
}
