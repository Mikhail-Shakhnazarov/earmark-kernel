/*
 * Copyright (c) 2026 Mikhail Shakhnazarov. Dual-licensed under AGPL-3.0-or-later or commercial terms.
 * PROPRIETARY AND INTERNAL. ONLY LOCALLY COMMITTED.
 * v0.1_internal kernel.
 */

use crate::errors::IndexError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexDirtyMarker {
    pub reason: String,
    pub timestamp: DateTime<Utc>,
}

pub fn mark_dirty(root: PathBuf, marker: IndexDirtyMarker) -> Result<(), IndexError> {
    let path = root
        .join(".earmark")
        .join("derived")
        .join("index_dirty.json");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(&marker)?;
    fs::write(path, json)?;
    Ok(())
}

pub fn clear_dirty(root: PathBuf) -> Result<(), IndexError> {
    let path = root
        .join(".earmark")
        .join("derived")
        .join("index_dirty.json");
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

pub fn get_dirty_status(root: PathBuf) -> Result<Option<IndexDirtyMarker>, IndexError> {
    let path = root
        .join(".earmark")
        .join("derived")
        .join("index_dirty.json");
    if !path.exists() {
        return Ok(None);
    }
    let json = fs::read_to_string(path)?;
    Ok(Some(serde_json::from_str(&json)?))
}
