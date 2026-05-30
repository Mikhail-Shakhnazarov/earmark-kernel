/*
 * Copyright (c) 2026 Mikhail Shakhnazarov. Dual-licensed under AGPL-3.0-or-later or commercial terms.
 * PROPRIETARY AND INTERNAL. ONLY LOCALLY COMMITTED.
 * v0.1_internal kernel.
 */

use thiserror::Error;

#[derive(Error, Debug)]
pub enum DeclarationError {
    #[error("Declaration not found: {0}")]
    NotFound(String),

    #[error("Invalid manifest: {0}")]
    InvalidManifest(String),

    #[error("Duplicate ID: {0}")]
    DuplicateId(String),

    #[error("Missing reference: {0}")]
    MissingReference(String),

    #[error("Schema invalid: {0}")]
    SchemaInvalid(String),

    #[error("Activation failed: {0}")]
    ActivationFailed(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}
