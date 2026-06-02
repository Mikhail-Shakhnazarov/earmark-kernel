/*
 * Copyright (c) 2026 Mikhail Shakhnazarov.
 * Dual-licensed under AGPL-3.0-or-later or commercial terms.
 */

use crate::ids::{ActorId, PacketId, ReviewId, RunId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalType {
    Produced,
    Accepted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalIntegrityState {
    Unchecked,
    Verified,
    Skipped,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProductionContext {
    pub run_id: Option<RunId>,
    pub packet_id: Option<PacketId>,
    pub actor_id: Option<ActorId>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SignalState {
    pub signal_type: SignalType,
    pub integrity_state: SignalIntegrityState,
    pub production_context: Option<ProductionContext>,
    pub acceptance_review: Option<ReviewId>,
}
