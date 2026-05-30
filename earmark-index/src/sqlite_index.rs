/*
 * Copyright (c) 2026 Mikhail Shakhnazarov.
 * Dual-licensed under AGPL-3.0-or-later or commercial terms.
 */

use async_trait::async_trait;
use earmark_core::{
    DispatchId, DispatchRecord, HandoffManifestId, HandoffManifestRecord, ObjectId, ObjectRecord,
    PacketId, PacketRecord, RelationId, RelationRecord, ReviewId, ReviewRecord, RunId, RunRecord,
    VersionId,
};
use earmark_store::traits::{DerivedIndex, ObjectQuery};
use earmark_store::{CanonicalStore, StoreError};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json;
use std::path::PathBuf;
use std::time::Instant;
use tracing::info;

pub struct SqliteIndex {
    conn: tokio::sync::Mutex<Connection>,
}

impl SqliteIndex {
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, earmark_store::StoreError> {
        let conn = Connection::open(path.into())
            .map_err(|e| earmark_store::StoreError::Generic(e.to_string()))?;
        let index = Self {
            conn: tokio::sync::Mutex::new(conn),
        };
        // Note: init_schema is synchronous in the original code, but we can call it here
        // by briefly locking if we want, or just making it part of the constructor flow.
        // For simplicity, we'll keep it as is but wrap in block_on or just make it async.
        // Actually, let's make it async and call it.
        Ok(index)
    }

    pub async fn init_schema(&self) -> Result<(), earmark_store::StoreError> {
        let conn = self.conn.lock().await;
        conn.execute_batch(
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

            CREATE TABLE IF NOT EXISTS packets (
                packet_id TEXT PRIMARY KEY,
                record_json TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS handoffs (
                handoff_id TEXT PRIMARY KEY,
                record_json TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS reviews (
                review_id TEXT PRIMARY KEY,
                record_json TEXT NOT NULL
            );

            -- Indices for traversal and filtering
            CREATE INDEX IF NOT EXISTS idx_relations_source ON relations(source_id);
            CREATE INDEX IF NOT EXISTS idx_versions_object ON versions(object_id);
            CREATE INDEX IF NOT EXISTS idx_claims_expires ON active_dispatch_claims(lease_expires_at);
            "#,
        ).map_err(|e| earmark_store::StoreError::Generic(e.to_string()))?;
        Ok(())
    }
}

#[async_trait]
impl DerivedIndex for SqliteIndex {
    async fn rebuild_from_store(
        &mut self,
        store: &(dyn CanonicalStore + Sync),
    ) -> Result<(), StoreError> {
        let start = Instant::now();
        info!("Starting full SQLite index rebuild");

        let conn = self.conn.lock().await;
        // In rusqlite, we can't easily do a cross-boundary transaction with async locks
        // without more complexity (like a dedicated thread or pooling),
        // but for a rebuild we can just execute.

        conn.execute("DELETE FROM objects", [])
            .map_err(|e| StoreError::Generic(e.to_string()))?;
        conn.execute("DELETE FROM versions", [])
            .map_err(|e| StoreError::Generic(e.to_string()))?;
        conn.execute("DELETE FROM relations", [])
            .map_err(|e| StoreError::Generic(e.to_string()))?;
        conn.execute("DELETE FROM runs", [])
            .map_err(|e| StoreError::Generic(e.to_string()))?;
        conn.execute("DELETE FROM dispatches", [])
            .map_err(|e| StoreError::Generic(e.to_string()))?;
        conn.execute("DELETE FROM packets", [])
            .map_err(|e| StoreError::Generic(e.to_string()))?;
        conn.execute("DELETE FROM handoffs", [])
            .map_err(|e| StoreError::Generic(e.to_string()))?;
        conn.execute("DELETE FROM reviews", [])
            .map_err(|e| StoreError::Generic(e.to_string()))?;

        // 1. Rebuild Objects and Versions
        let object_ids = store
            .list_objects()
            .map_err(|e| StoreError::Generic(e.to_string()))?;
        for id in object_ids {
            let obj = store
                .get_object(&id)
                .map_err(|e| StoreError::Generic(e.to_string()))?;
            let obj_json =
                serde_json::to_string(&obj).map_err(|e| StoreError::Generic(e.to_string()))?;

            conn.execute(
                "INSERT INTO objects (object_id, class_id, latest_version_id, updated_at, record_json) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    obj.id.as_str(),
                    obj.class_id.as_ref().map(|c| c.as_str()),
                    obj.latest_version_id.as_str(),
                    obj.updated_at.to_rfc3339(),
                    obj_json
                ],
            ).map_err(|e| StoreError::Generic(e.to_string()))?;
        }

        // 2. Rebuild Relations
        let relation_ids = store
            .list_all_relations()
            .map_err(|e| StoreError::Generic(e.to_string()))?;
        for rid in relation_ids {
            let rel = store
                .get_relation(&rid)
                .map_err(|e| StoreError::Generic(e.to_string()))?;
            let rel_json =
                serde_json::to_string(&rel).map_err(|e| StoreError::Generic(e.to_string()))?;
            conn.execute(
                "INSERT INTO relations (relation_id, source_id, target_id, relation_type, record_json) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    rel.id.as_str(),
                    rel.source_id.as_str(),
                    rel.target_id.as_str(),
                    rel.relation_type,
                    rel_json
                ],
            ).map_err(|e| StoreError::Generic(e.to_string()))?;
        }

        // 3. Rebuild Runs
        let run_ids = store
            .list_runs()
            .map_err(|e| StoreError::Generic(e.to_string()))?;
        for rid in run_ids {
            let run = store
                .get_run(&rid)
                .map_err(|e| StoreError::Generic(e.to_string()))?;
            let run_json =
                serde_json::to_string(&run).map_err(|e| StoreError::Generic(e.to_string()))?;
            conn.execute(
                "INSERT INTO runs (run_id, record_json) VALUES (?1, ?2)",
                params![rid.as_str(), run_json],
            )
            .map_err(|e| StoreError::Generic(e.to_string()))?;
        }

        // 4. Rebuild Dispatches
        let dispatch_ids = store
            .list_dispatches()
            .map_err(|e| StoreError::Generic(e.to_string()))?;
        for did in dispatch_ids {
            let dispatch = store
                .get_dispatch(&did)
                .map_err(|e| StoreError::Generic(e.to_string()))?;
            let dispatch_json =
                serde_json::to_string(&dispatch).map_err(|e| StoreError::Generic(e.to_string()))?;
            conn.execute(
                "INSERT OR REPLACE INTO dispatches (dispatch_id, record_json) VALUES (?1, ?2)",
                params![did.as_str(), dispatch_json],
            )
            .map_err(|e| StoreError::Generic(e.to_string()))?;
        }

        // 5. Rebuild Packets
        let packet_ids = store
            .list_packets()
            .map_err(|e| StoreError::Generic(e.to_string()))?;
        for pid in packet_ids {
            let packet = store
                .get_packet(&pid)
                .map_err(|e| StoreError::Generic(e.to_string()))?;
            let packet_json =
                serde_json::to_string(&packet).map_err(|e| StoreError::Generic(e.to_string()))?;
            conn.execute(
                "INSERT INTO packets (packet_id, record_json) VALUES (?1, ?2)",
                params![pid.as_str(), packet_json],
            )
            .map_err(|e| StoreError::Generic(e.to_string()))?;
        }

        // 6. Rebuild Reviews
        let review_ids = store
            .list_reviews()
            .map_err(|e| StoreError::Generic(e.to_string()))?;
        for rid in review_ids {
            let review = store
                .get_review(&rid)
                .map_err(|e| StoreError::Generic(e.to_string()))?;
            let review_json =
                serde_json::to_string(&review).map_err(|e| StoreError::Generic(e.to_string()))?;
            conn.execute(
                "INSERT OR REPLACE INTO reviews (review_id, record_json) VALUES (?1, ?2)",
                params![review.review_id.as_str(), review_json],
            )
            .map_err(|e| StoreError::Generic(e.to_string()))?;
        }

        let duration = start.elapsed();
        metrics::histogram!("index.rebuild.duration").record(duration.as_secs_f64());
        Ok(())
    }

    async fn get_object(&self, id: &ObjectId) -> Result<ObjectRecord, StoreError> {
        let conn = self.conn.lock().await;
        let mut stmt = conn
            .prepare("SELECT record_json FROM objects WHERE object_id = ?1")
            .map_err(|e| StoreError::Generic(e.to_string()))?;
        let json: String = stmt
            .query_row(params![id.as_str()], |row| row.get(0))
            .optional()
            .map_err(|e| StoreError::Generic(e.to_string()))?
            .ok_or_else(|| StoreError::Generic(format!("Object {} not found", id)))?;
        serde_json::from_str(&json).map_err(|e| StoreError::Generic(e.to_string()))
    }

    async fn get_head_version_id(&self, id: &ObjectId) -> Result<VersionId, StoreError> {
        let conn = self.conn.lock().await;
        let mut stmt = conn
            .prepare("SELECT latest_version_id FROM objects WHERE object_id = ?1")
            .map_err(|e| StoreError::Generic(e.to_string()))?;
        let ver_str: String = stmt
            .query_row(params![id.as_str()], |row| row.get(0))
            .optional()
            .map_err(|e| StoreError::Generic(e.to_string()))?
            .ok_or_else(|| StoreError::Generic(format!("Object {} not found", id)))?;
        VersionId::parse(&ver_str).map_err(|e| StoreError::Generic(e.to_string()))
    }

    async fn get_relation(&self, id: &RelationId) -> Result<RelationRecord, StoreError> {
        let conn = self.conn.lock().await;
        let mut stmt = conn
            .prepare("SELECT record_json FROM relations WHERE relation_id = ?1")
            .map_err(|e| StoreError::Generic(e.to_string()))?;
        let json: String = stmt
            .query_row(params![id.as_str()], |row| row.get(0))
            .optional()
            .map_err(|e| StoreError::Generic(e.to_string()))?
            .ok_or_else(|| StoreError::Generic(format!("Relation {} not found", id)))?;
        serde_json::from_str(&json).map_err(|e| StoreError::Generic(e.to_string()))
    }

    async fn find_objects(&self, query: ObjectQuery) -> Result<Vec<ObjectRecord>, StoreError> {
        let conn = self.conn.lock().await;
        let mut sql = "SELECT record_json FROM objects".to_string();
        let mut params_vec = Vec::new();
        if let Some(class_id) = query.class_id {
            sql.push_str(" WHERE class_id = ?1");
            params_vec.push(class_id.as_str().to_string());
        }

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| StoreError::Generic(e.to_string()))?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(params_vec), |row| {
                let json: String = row.get(0)?;
                Ok(json)
            })
            .map_err(|e| StoreError::Generic(e.to_string()))?;

        let mut results = Vec::new();
        for row in rows {
            let json = row.map_err(|e| StoreError::Generic(e.to_string()))?;
            results
                .push(serde_json::from_str(&json).map_err(|e| StoreError::Generic(e.to_string()))?);
        }
        Ok(results)
    }

    async fn find_relations_by_source(
        &self,
        source_id: &ObjectId,
    ) -> Result<Vec<RelationRecord>, StoreError> {
        let conn = self.conn.lock().await;
        let mut stmt = conn
            .prepare("SELECT record_json FROM relations WHERE source_id = ?1")
            .map_err(|e| StoreError::Generic(e.to_string()))?;
        let rows = stmt
            .query_map(params![source_id.as_str()], |row| {
                let json: String = row.get(0)?;
                Ok(json)
            })
            .map_err(|e| StoreError::Generic(e.to_string()))?;

        let mut results = Vec::new();
        for row in rows {
            let json = row.map_err(|e| StoreError::Generic(e.to_string()))?;
            results
                .push(serde_json::from_str(&json).map_err(|e| StoreError::Generic(e.to_string()))?);
        }
        Ok(results)
    }

    async fn get_run(&self, id: &RunId) -> Result<RunRecord, StoreError> {
        let conn = self.conn.lock().await;
        let mut stmt = conn
            .prepare("SELECT record_json FROM runs WHERE run_id = ?1")
            .map_err(|e| StoreError::Generic(e.to_string()))?;
        let json: String = stmt
            .query_row(params![id.as_str()], |row| row.get(0))
            .optional()
            .map_err(|e| StoreError::Generic(e.to_string()))?
            .ok_or_else(|| StoreError::Generic(format!("Run {} not found", id)))?;
        serde_json::from_str(&json).map_err(|e| StoreError::Generic(e.to_string()))
    }

    async fn get_dispatch(&self, id: &DispatchId) -> Result<DispatchRecord, StoreError> {
        let conn = self.conn.lock().await;
        let mut stmt = conn
            .prepare("SELECT record_json FROM dispatches WHERE dispatch_id = ?1")
            .map_err(|e| StoreError::Generic(e.to_string()))?;
        let json: String = stmt
            .query_row(params![id.as_str()], |row| row.get(0))
            .optional()
            .map_err(|e| StoreError::Generic(e.to_string()))?
            .ok_or_else(|| StoreError::Generic(format!("Dispatch {} not found", id)))?;
        serde_json::from_str(&json).map_err(|e| StoreError::Generic(e.to_string()))
    }

    async fn get_packet(&self, id: &PacketId) -> Result<PacketRecord, StoreError> {
        let conn = self.conn.lock().await;
        let mut stmt = conn
            .prepare("SELECT record_json FROM packets WHERE packet_id = ?1")
            .map_err(|e| StoreError::Generic(e.to_string()))?;
        let json: String = stmt
            .query_row(params![id.as_str()], |row| row.get(0))
            .optional()
            .map_err(|e| StoreError::Generic(e.to_string()))?
            .ok_or_else(|| StoreError::Generic(format!("Packet {} not found", id)))?;
        serde_json::from_str(&json).map_err(|e| StoreError::Generic(e.to_string()))
    }

    async fn get_handoff(
        &self,
        id: &HandoffManifestId,
    ) -> Result<HandoffManifestRecord, StoreError> {
        let conn = self.conn.lock().await;
        let mut stmt = conn
            .prepare("SELECT record_json FROM handoffs WHERE handoff_id = ?1")
            .map_err(|e| StoreError::Generic(e.to_string()))?;
        let json: String = stmt
            .query_row(params![id.as_str()], |row| row.get(0))
            .optional()
            .map_err(|e| StoreError::Generic(e.to_string()))?
            .ok_or_else(|| StoreError::Generic(format!("Handoff {} not found", id)))?;
        serde_json::from_str(&json).map_err(|e| StoreError::Generic(e.to_string()))
    }

    async fn get_review(&self, id: &ReviewId) -> Result<ReviewRecord, StoreError> {
        let conn = self.conn.lock().await;
        let mut stmt = conn
            .prepare("SELECT record_json FROM reviews WHERE review_id = ?1")
            .map_err(|e| StoreError::Generic(e.to_string()))?;
        let json: String = stmt
            .query_row(params![id.as_str()], |row| row.get(0))
            .optional()
            .map_err(|e| StoreError::Generic(e.to_string()))?
            .ok_or_else(|| StoreError::Generic(format!("Review {} not found", id)))?;
        serde_json::from_str(&json).map_err(|e| StoreError::Generic(e.to_string()))
    }

    async fn list_active_claims(&self) -> Result<Vec<DispatchRecord>, StoreError> {
        let conn = self.conn.lock().await;
        let mut stmt = conn
            .prepare("SELECT record_json FROM active_dispatch_claims WHERE lease_expires_at > ?1")
            .map_err(|e| StoreError::Generic(e.to_string()))?;
        let now = chrono::Utc::now().to_rfc3339();
        let rows = stmt
            .query_map(params![now], |row| {
                let json: String = row.get(0)?;
                Ok(json)
            })
            .map_err(|e| StoreError::Generic(e.to_string()))?;
        let mut results = Vec::new();
        for row in rows {
            let json = row.map_err(|e| StoreError::Generic(e.to_string()))?;
            results
                .push(serde_json::from_str(&json).map_err(|e| StoreError::Generic(e.to_string()))?);
        }
        Ok(results)
    }

    async fn list_expired_leases(&self) -> Result<Vec<DispatchRecord>, StoreError> {
        let conn = self.conn.lock().await;
        let mut stmt = conn
            .prepare("SELECT record_json FROM active_dispatch_claims WHERE lease_expires_at <= ?1")
            .map_err(|e| StoreError::Generic(e.to_string()))?;
        let now = chrono::Utc::now().to_rfc3339();
        let rows = stmt
            .query_map(params![now], |row| {
                let json: String = row.get(0)?;
                Ok(json)
            })
            .map_err(|e| StoreError::Generic(e.to_string()))?;
        let mut results = Vec::new();
        for row in rows {
            let json = row.map_err(|e| StoreError::Generic(e.to_string()))?;
            results
                .push(serde_json::from_str(&json).map_err(|e| StoreError::Generic(e.to_string()))?);
        }
        Ok(results)
    }

    async fn upsert_object(&self, object: &ObjectRecord) -> Result<(), StoreError> {
        let conn = self.conn.lock().await;
        let obj_json =
            serde_json::to_string(object).map_err(|e| StoreError::Generic(e.to_string()))?;
        conn.execute(
            "INSERT OR REPLACE INTO objects (object_id, class_id, latest_version_id, updated_at, record_json) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                object.id.as_str(),
                object.class_id.as_ref().map(|c| c.as_str()),
                object.latest_version_id.as_str(),
                chrono::Utc::now().to_rfc3339(),
                obj_json
            ],
        ).map_err(|e| StoreError::Generic(e.to_string()))?;
        Ok(())
    }

    async fn upsert_relation(&self, relation: &RelationRecord) -> Result<(), StoreError> {
        let conn = self.conn.lock().await;
        let rel_json =
            serde_json::to_string(relation).map_err(|e| StoreError::Generic(e.to_string()))?;
        conn.execute(
            "INSERT OR REPLACE INTO relations (relation_id, source_id, target_id, relation_type, record_json) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                relation.id.as_str(),
                relation.source_id.as_str(),
                relation.target_id.as_str(),
                relation.relation_type,
                rel_json
            ],
        ).map_err(|e| StoreError::Generic(e.to_string()))?;
        Ok(())
    }
}
