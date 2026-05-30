/*
 * Copyright (c) 2026 Mikhail Shakhnazarov.
 * Dual-licensed under AGPL-3.0-or-later or commercial terms.
 */

use earmark_core::CoreError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StoreError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Core error: {0}")]
    Core(#[from] CoreError),

    #[error("Workspace not initialized")]
    NotInitialized,

    #[error("Object not found: {0}")]
    ObjectNotFound(String),

    #[error("Version not found: {0}")]
    VersionNotFound(String),

    #[error("Invariant violation: {0}")]
    Invariant(String),

    #[error("Sovereignty violation: {0}")]
    Sovereignty(String),

    #[error("Other error: {0}")]
    Generic(String),

    #[error("Regression violation: {0}")]
    Regression(String),

    #[error("Dispatch already claimed: {0}")]
    AlreadyClaimed(String),

    #[error("Dispatch not claimed: {0}")]
    NotClaimed(String),

    #[error("Lease expired: {0}")]
    LeaseExpired(String),
}
