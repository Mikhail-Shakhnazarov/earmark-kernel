/*
 * Copyright (c) 2026 Mikhail Shakhnazarov.
 * Dual-licensed under AGPL-3.0-or-later or commercial terms.
 */

use crate::errors::IndexError;
use crate::traits::{DerivedIndex, ObjectQuery};
use chrono::Utc;
use earmark_core::{
    DispatchId, DispatchRecord, HandoffManifestId, HandoffManifestRecord, ObjectId, ObjectRecord,
    PacketId, PacketRecord, RelationId, RelationRecord, ReviewId, ReviewRecord, RunId, RunRecord,
    VersionId,
};
use earmark_store::CanonicalStore;
use rusqlite::{params, Connection, OptionalExtension};
use serde_json;
use std::path::PathBuf;

pub struct SqliteIndex {
    conn: Connection,
}

impl SqliteIndex {
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, IndexError> {
        let conn = Connection::open(path.into())?;
        let index = Self { conn };
        index.init_schema()?;
        Ok(index)
    }

    fn init_schema(&self) -> Result<(), IndexError> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS objects (
                object_id TEXT PRIMARY KEY,
                class_id TEXT,
                latest_version_id TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                record_json TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS versions (
                version_id TEXT PRIMARY KEY,
                object_id TEXT NOT NULL,
                record_json TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS relations (
                relation_id TEXT PRIMARY KEY,
                source_id TEXT NOT NULL,
                target_id TEXT NOT NULL,
                relation_type TEXT NOT NULL,
                record_json TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS runs (
                run_id TEXT PRIMARY KEY,
                record_json TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS dispatches (
                dispatch_id TEXT PRIMARY KEY,
                run_id TEXT NOT NULL,
                record_json TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS active_dispatch_claims (
                dispatch_id TEXT PRIMARY KEY,
                claimed_by TEXT NOT NULL,
                claimed_at TEXT NOT NULL,
                lease_expires_at TEXT NOT NULL,
                record_json TEXT NOT NULL
            );

            -- Indices for traversal and filtering
            CREATE INDEX IF NOT EXISTS idx_relations_source ON relations(source_id);
            CREATE INDEX IF NOT EXISTS idx_versions_object ON versions(object_id);
            CREATE INDEX IF NOT EXISTS idx_claims_expires ON active_dispatch_claims(lease_expires_at);
            "#,
        )?;
        Ok(())
    }
}

impl DerivedIndex for SqliteIndex {
    fn rebuild_from_store(&mut self, store: &dyn CanonicalStore) -> Result<(), IndexError> {
        // Clear existing data
        self.conn.execute("DELETE FROM objects", [])?;
        self.conn.execute("DELETE FROM versions", [])?;
        self.conn.execute("DELETE FROM relations", [])?;
        self.conn.execute("DELETE FROM runs", [])?;
        self.conn.execute("DELETE FROM dispatches", [])?;

        // 1. Rebuild Objects and Versions
        let object_ids = store.list_objects()?;
        for id in object_ids {
            let obj = store.get_object(&id)?;
            let obj_json = serde_json::to_string(&obj)?;

            self.conn.execute(
                "INSERT INTO objects (object_id, class_id, latest_version_id, updated_at, record_json) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    obj.id.as_str(),
                    obj.class_id.as_ref().map(|c| c.as_str()),
                    obj.latest_version_id.as_str(),
                    obj.updated_at.to_rfc3339(),
                    obj_json
                ],
            )?;

            // Rebuild Versions
            let version_ids = store.list_versions(&id)?;
            for vid in version_ids {
                let ver = store.get_version(&id, &vid)?;
                let ver_json = serde_json::to_string(&ver)?;
                self.conn.execute(
                    "INSERT INTO versions (version_id, object_id, record_json) VALUES (?1, ?2, ?3)",
                    params![vid.as_str(), id.as_str(), ver_json],
                )?;
            }
        }

        // 2. Rebuild Relations
        let relation_ids = store.list_all_relations()?;
        for rid in relation_ids {
            let rel = store.get_relation(&rid)?;
            let rel_json = serde_json::to_string(&rel)?;
            self.conn.execute(
                "INSERT INTO relations (relation_id, source_id, target_id, relation_type, record_json) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    rel.id.as_str(),
                    rel.source_id.as_str(),
                    rel.target_id.as_str(),
                    rel.relation_type,
                    rel_json
                ],
            )?;
        }

        // 3. Rebuild Runs
        let run_ids = store.list_runs()?;
        for rid in run_ids {
            let run = store.get_run(&rid)?;
            let run_json = serde_json::to_string(&run)?;
            self.conn.execute(
                "INSERT INTO runs (run_id, record_json) VALUES (?1, ?2)",
                params![rid.as_str(), run_json],
            )?;
        }

        // 4. Rebuild Dispatches
        let dispatch_ids = store.list_dispatches()?;
        for did in dispatch_ids {
            let disp = store.get_dispatch(&did)?;
            let disp_json = serde_json::to_string(&disp)?;
            self.conn.execute(
                "INSERT INTO dispatches (dispatch_id, run_id, record_json) VALUES (?1, ?2, ?3)",
                params![did.as_str(), disp.run_id.as_str(), disp_json],
            )?;
        }

        // 5. Rebuild Active Dispatch Claims
        self.conn
            .execute("DELETE FROM active_dispatch_claims", [])?;
        let dispatch_ids = store.list_dispatches()?;
        for did in dispatch_ids {
            let disp = store.get_dispatch(&did)?;
            if let Some(ref claimed_by) = disp.claimed_by {
                if let (Some(claimed_at), Some(lease_expires_at)) =
                    (disp.claimed_at, disp.lease_expires_at)
                {
                    let claim_json = serde_json::to_string(&disp)?;
                    self.conn.execute(
                        "INSERT INTO active_dispatch_claims (dispatch_id, claimed_by, claimed_at, lease_expires_at, record_json) VALUES (?1, ?2, ?3, ?4, ?5)",
                        params![
                            did.as_str(),
                            claimed_by.as_str(),
                            claimed_at.to_rfc3339(),
                            lease_expires_at.to_rfc3339(),
                            claim_json
                        ],
                    )?;
                }
            }
        }

        Ok(())
    }

    fn get_object(&self, id: &ObjectId) -> Result<ObjectRecord, IndexError> {
        let mut stmt = self
            .conn
            .prepare("SELECT record_json FROM objects WHERE object_id = ?1")?;
        let json: String = stmt
            .query_row(params![id.as_str()], |row| row.get(0))
            .optional()?
            .ok_or_else(|| IndexError::NotFound(id.as_str().to_string()))?;
        serde_json::from_str(&json).map_err(IndexError::from)
    }

    fn get_head_version_id(&self, id: &ObjectId) -> Result<VersionId, IndexError> {
        let mut stmt = self
            .conn
            .prepare("SELECT latest_version_id FROM objects WHERE object_id = ?1")?;
        let ver_str: String = stmt
            .query_row(params![id.as_str()], |row| row.get(0))
            .optional()?
            .ok_or_else(|| IndexError::NotFound(id.as_str().to_string()))?;
        Ok(VersionId::parse(&ver_str)?)
    }

    fn get_relation(&self, id: &RelationId) -> Result<RelationRecord, IndexError> {
        let mut stmt = self
            .conn
            .prepare("SELECT record_json FROM relations WHERE relation_id = ?1")?;
        let json: String = stmt
            .query_row(params![id.as_str()], |row| row.get(0))
            .optional()?
            .ok_or_else(|| IndexError::NotFound(id.as_str().to_string()))?;
        serde_json::from_str(&json).map_err(IndexError::from)
    }

    fn find_objects(&self, query: ObjectQuery) -> Result<Vec<ObjectRecord>, IndexError> {
        let mut sql = "SELECT record_json FROM objects".to_string();
        let mut params_vec = Vec::new();
        if let Some(class_id) = query.class_id {
            sql.push_str(" WHERE class_id = ?1");
            params_vec.push(class_id.as_str().to_string());
        }

        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(params_vec), |row| {
            let json: String = row.get(0)?;
            Ok(json)
        })?;

        let mut results = Vec::new();
        for row in rows {
            let json = row?;
            results.push(serde_json::from_str(&json).map_err(IndexError::Serde)?);
        }
        Ok(results)
    }

    fn find_relations_by_source(
        &self,
        source_id: &ObjectId,
    ) -> Result<Vec<RelationRecord>, IndexError> {
        let mut stmt = self
            .conn
            .prepare("SELECT record_json FROM relations WHERE source_id = ?1")?;
        let rows = stmt.query_map(params![source_id.as_str()], |row| {
            let json: String = row.get(0)?;
            Ok(json)
        })?;

        let mut results = Vec::new();
        for row in rows {
            let json = row?;
            results.push(serde_json::from_str(&json).map_err(IndexError::Serde)?);
        }
        Ok(results)
    }

    fn get_run(&self, id: &RunId) -> Result<RunRecord, IndexError> {
        let mut stmt = self
            .conn
            .prepare("SELECT record_json FROM runs WHERE run_id = ?1")?;
        let json: String = stmt
            .query_row(params![id.as_str()], |row| row.get(0))
            .optional()?
            .ok_or_else(|| IndexError::NotFound(id.as_str().to_string()))?;
        serde_json::from_str(&json).map_err(IndexError::from)
    }

    fn get_dispatch(&self, id: &DispatchId) -> Result<DispatchRecord, IndexError> {
        let mut stmt = self
            .conn
            .prepare("SELECT record_json FROM dispatches WHERE dispatch_id = ?1")?;
        let json: String = stmt
            .query_row(params![id.as_str()], |row| row.get(0))
            .optional()?
            .ok_or_else(|| IndexError::NotFound(id.as_str().to_string()))?;
        serde_json::from_str(&json).map_err(IndexError::from)
    }

    fn get_packet(&self, _id: &PacketId) -> Result<PacketRecord, IndexError> {
        Err(IndexError::NotFound(
            "packet lookup not implemented".to_string(),
        ))
    }

    fn get_handoff(&self, _id: &HandoffManifestId) -> Result<HandoffManifestRecord, IndexError> {
        Err(IndexError::NotFound(
            "handoff lookup not implemented".to_string(),
        ))
    }

    fn get_review(&self, _id: &ReviewId) -> Result<ReviewRecord, IndexError> {
        Err(IndexError::NotFound(
            "review lookup not implemented".to_string(),
        ))
    }

    fn list_active_claims(&self) -> Result<Vec<DispatchRecord>, IndexError> {
        let mut stmt = self.conn.prepare(
            "SELECT record_json FROM active_dispatch_claims WHERE lease_expires_at > ?1",
        )?;
        let now = Utc::now().to_rfc3339();
        let rows = stmt.query_map(params![now], |row| {
            let json: String = row.get(0)?;
            Ok(json)
        })?;
        let mut results = Vec::new();
        for row in rows {
            let json = row?;
            results.push(serde_json::from_str(&json).map_err(IndexError::Serde)?);
        }
        Ok(results)
    }

    fn list_expired_leases(&self) -> Result<Vec<DispatchRecord>, IndexError> {
        let mut stmt = self.conn.prepare(
            "SELECT record_json FROM active_dispatch_claims WHERE lease_expires_at <= ?1",
        )?;
        let now = Utc::now().to_rfc3339();
        let rows = stmt.query_map(params![now], |row| {
            let json: String = row.get(0)?;
            Ok(json)
        })?;
        let mut results = Vec::new();
        for row in rows {
            let json = row?;
            results.push(serde_json::from_str(&json).map_err(IndexError::Serde)?);
        }
        Ok(results)
    }
}
