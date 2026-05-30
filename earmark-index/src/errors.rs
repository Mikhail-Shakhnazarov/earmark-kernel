/*
 * Copyright (c) 2026 Mikhail Shakhnazarov. Dual-licensed under AGPL-3.0-or-later or commercial terms.
 * PROPRIETARY AND INTERNAL. ONLY LOCALLY COMMITTED.
 * v0.1_internal kernel.
 */

use earmark_core::CoreError;
use earmark_store::StoreError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum IndexError {
    #[error("SQL error: {0}")]
    Sql(#[from] rusqlite::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Store error: {0}")]
    Store(#[from] StoreError),

    #[error("Core error: {0}")]
    Core(#[from] CoreError),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Query invalid: {0}")]
    QueryInvalid(String),

    #[error("Rebuild failed: {0}")]
    RebuildFailed(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("Other error: {0}")]
    Other(String),
}
