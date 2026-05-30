/*
 * Copyright (c) 2026 Mikhail Shakhnazarov.
 * Dual-licensed under AGPL-3.0-or-later or commercial terms.
 */

use thiserror::Error;

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("Invalid identifier: {0}")]
    InvalidIdentifier(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Invalid standing: {0}")]
    InvalidStanding(String),

    #[error("Other error: {0}")]
    Other(String),
}
