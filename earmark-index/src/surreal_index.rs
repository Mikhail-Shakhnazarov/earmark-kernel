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
use earmark_store::StoreError;
use std::path::PathBuf;
use surrealdb::engine::local::SurrealKv;
use surrealdb::Surreal;
use tracing::{debug, info, instrument};

pub struct SurrealIndex {
    db: Surreal<surrealdb::engine::local::Db>,
}

impl SurrealIndex {
    pub async fn open(path: impl Into<PathBuf>) -> Result<Self, earmark_store::StoreError> {
        let path: PathBuf = path.into();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(earmark_store::StoreError::Io)?;
        }

        let db = Surreal::new::<SurrealKv>(path)
            .await
            .map_err(|e| earmark_store::StoreError::Generic(e.to_string()))?;
        db.use_ns("earmark")
            .use_db("live")
            .await
            .map_err(|e| earmark_store::StoreError::Generic(e.to_string()))?;

        let index = Self { db };
        index.init_schema().await?;
        Ok(index)
    }

    async fn init_schema(&self) -> Result<(), earmark_store::StoreError> {
        // Use SCHEMALESS for now to ensure flexibility during migration,
        // while still providing indexes for performance.
        let queries = vec![
            "DEFINE TABLE objects SCHEMALESS",
            "DEFINE TABLE relations SCHEMALESS",
            "DEFINE TABLE runs SCHEMALESS",
            "DEFINE TABLE dispatches SCHEMALESS",
        ];

        for q in queries {
            if let Err(e) = self
                .db
                .query(q)
                .await
                .map_err(|e| earmark_store::StoreError::Generic(e.to_string()))?
                .check()
            {
                let err_msg = e.to_string();
                if !err_msg.contains("already exists") {
                    return Err(earmark_store::StoreError::Generic(err_msg));
                }
            }
        }

        info!("SurrealDB schema initialized");
        Ok(())
    }

    async fn upsert_object_internal(
        &self,
        object: &ObjectRecord,
    ) -> Result<(), earmark_store::StoreError> {
        info!(id = %object.id, "Upserting object into SurrealDB");
        let val = serde_json::to_value(object)
            .map_err(|e| earmark_store::StoreError::Generic(e.to_string()))?;
        self.db
            .upsert::<Option<serde_json::Value>>(("objects", object.id.as_str()))
            .content(val)
            .await
            .map_err(|e| earmark_store::StoreError::Generic(e.to_string()))?;

        debug!(object_id = %object.id, "Upserted object in SurrealDB");
        Ok(())
    }

    async fn upsert_relation_internal(
        &self,
        relation: &RelationRecord,
    ) -> Result<(), earmark_store::StoreError> {
        let val = serde_json::to_value(relation)
            .map_err(|e| earmark_store::StoreError::Generic(e.to_string()))?;
        // Use RELATE to create an edge between objects
        let query = format!(
            "RELATE objects:{} -> {} -> objects:{} CONTENT $content",
            relation.source_id.as_str(),
            relation.relation_type,
            relation.target_id.as_str()
        );

        self.db
            .query(query)
            .bind(("content", val))
            .await
            .map_err(|e| earmark_store::StoreError::Generic(e.to_string()))?;

        debug!(relation_id = %relation.id, "Upserted relation graph edge in SurrealDB");
        Ok(())
    }

    async fn upsert_run_internal(&self, run: &RunRecord) -> Result<(), earmark_store::StoreError> {
        let val = serde_json::to_value(run)
            .map_err(|e| earmark_store::StoreError::Generic(e.to_string()))?;
        self.db
            .upsert::<Option<serde_json::Value>>(("runs", run.run_id.as_str()))
            .content(val)
            .await
            .map_err(|e| earmark_store::StoreError::Generic(e.to_string()))?;
        Ok(())
    }

    async fn upsert_dispatch_internal(
        &self,
        record: &DispatchRecord,
    ) -> Result<(), earmark_store::StoreError> {
        let val = serde_json::to_value(record)
            .map_err(|e| earmark_store::StoreError::Generic(e.to_string()))?;
        self.db
            .upsert::<Option<serde_json::Value>>(("dispatches", record.dispatch_id.as_str()))
            .content(val)
            .await
            .map_err(|e| earmark_store::StoreError::Generic(e.to_string()))?;
        Ok(())
    }

    async fn upsert_packet_internal(
        &self,
        record: &PacketRecord,
    ) -> Result<(), earmark_store::StoreError> {
        let val = serde_json::to_value(record)
            .map_err(|e| earmark_store::StoreError::Generic(e.to_string()))?;
        self.db
            .upsert::<Option<serde_json::Value>>(("packets", record.packet_id.as_str()))
            .content(val)
            .await
            .map_err(|e| earmark_store::StoreError::Generic(e.to_string()))?;
        Ok(())
    }

    async fn upsert_review_internal(
        &self,
        record: &ReviewRecord,
    ) -> Result<(), earmark_store::StoreError> {
        let val = serde_json::to_value(record)
            .map_err(|e| earmark_store::StoreError::Generic(e.to_string()))?;
        self.db
            .upsert::<Option<serde_json::Value>>(("reviews", record.review_id.as_str()))
            .content(val)
            .await
            .map_err(|e| earmark_store::StoreError::Generic(e.to_string()))?;
        Ok(())
    }
}

#[async_trait]
impl DerivedIndex for SurrealIndex {
    #[instrument(skip(self, store))]
    async fn rebuild_from_store(
        &mut self,
        store: &(dyn earmark_store::CanonicalStore + Sync),
    ) -> Result<(), earmark_store::StoreError> {
        let start = std::time::Instant::now();
        info!("Starting full SurrealDB index rebuild from store");

        let _: Vec<serde_json::Value> = self
            .db
            .query("DELETE objects, relations")
            .await
            .map_err(|e| earmark_store::StoreError::Generic(e.to_string()))?
            .take(0)
            .map_err(|e| earmark_store::StoreError::Generic(e.to_string()))?;

        let object_ids = store
            .list_objects()
            .map_err(|e| earmark_store::StoreError::Generic(e.to_string()))?;
        for id in object_ids {
            let obj = store
                .get_object(&id)
                .map_err(|e| earmark_store::StoreError::Generic(e.to_string()))?;
            self.upsert_object_internal(&obj).await?;
        }

        let relation_ids = store
            .list_all_relations()
            .map_err(|e| earmark_store::StoreError::Generic(e.to_string()))?;
        for id in relation_ids {
            let rel = store
                .get_relation(&id)
                .map_err(|e| earmark_store::StoreError::Generic(e.to_string()))?;
            self.upsert_relation_internal(&rel).await?;
        }

        let run_ids = store
            .list_runs()
            .map_err(|e| earmark_store::StoreError::Generic(e.to_string()))?;
        for id in run_ids {
            let run = store
                .get_run(&id)
                .map_err(|e| earmark_store::StoreError::Generic(e.to_string()))?;
            self.upsert_run_internal(&run).await?;
        }

        let dispatch_ids = store
            .list_dispatches()
            .map_err(|e| earmark_store::StoreError::Generic(e.to_string()))?;
        for id in dispatch_ids {
            let dispatch = store
                .get_dispatch(&id)
                .map_err(|e| earmark_store::StoreError::Generic(e.to_string()))?;
            self.upsert_dispatch_internal(&dispatch).await?;
        }

        let packet_ids = store
            .list_packets()
            .map_err(|e| earmark_store::StoreError::Generic(e.to_string()))?;
        for id in packet_ids {
            let packet = store
                .get_packet(&id)
                .map_err(|e| earmark_store::StoreError::Generic(e.to_string()))?;
            self.upsert_packet_internal(&packet).await?;
        }

        let review_ids = store
            .list_reviews()
            .map_err(|e| earmark_store::StoreError::Generic(e.to_string()))?;
        for id in review_ids {
            let review = store
                .get_review(&id)
                .map_err(|e| earmark_store::StoreError::Generic(e.to_string()))?;
            self.upsert_review_internal(&review).await?;
        }

        let duration = start.elapsed();
        metrics::describe_histogram!("index.rebuild.duration", "Duration of full index rebuild");
        metrics::histogram!("index.rebuild.duration").record(duration.as_secs_f64());
        Ok(())
    }

    async fn get_object(&self, id: &ObjectId) -> Result<ObjectRecord, StoreError> {
        let res: Option<serde_json::Value> = self
            .db
            .select(("objects", id.as_str()))
            .await
            .map_err(|e| StoreError::Generic(e.to_string()))?;
        let val = res.ok_or_else(|| StoreError::ObjectNotFound(id.as_str().to_string()))?;
        serde_json::from_value(val).map_err(|e| StoreError::Generic(e.to_string()))
    }

    async fn get_head_version_id(&self, id: &ObjectId) -> Result<VersionId, StoreError> {
        let obj = self.get_object(id).await?;
        Ok(obj.latest_version_id)
    }

    async fn get_relation(&self, id: &RelationId) -> Result<RelationRecord, StoreError> {
        let res = self
            .db
            .query("SELECT * FROM relations WHERE id = $id")
            .bind(("id", id.as_str()))
            .await
            .map_err(|e| StoreError::Generic(e.to_string()))?;
        let mut res = res
            .check()
            .map_err(|e| StoreError::Generic(e.to_string()))?;
        let values: Vec<serde_json::Value> = res
            .take(0)
            .map_err(|e| StoreError::Generic(e.to_string()))?;
        let val = values
            .into_iter()
            .next()
            .ok_or_else(|| StoreError::Generic(format!("Relation {} not found", id)))?;
        serde_json::from_value(val).map_err(|e| StoreError::Generic(e.to_string()))
    }

    async fn find_objects(&self, query: ObjectQuery) -> Result<Vec<ObjectRecord>, StoreError> {
        let res = self
            .db
            .query("SELECT * FROM objects")
            .await
            .map_err(|e| earmark_store::StoreError::Generic(e.to_string()))?;
        let mut res = res
            .check()
            .map_err(|e| earmark_store::StoreError::Generic(e.to_string()))?;

        let values: Vec<serde_json::Value> = res
            .take(0)
            .map_err(|e| earmark_store::StoreError::Generic(e.to_string()))?;

        let mut objects = Vec::new();
        for v in values {
            let obj: ObjectRecord = serde_json::from_value(v)
                .map_err(|e| earmark_store::StoreError::Generic(e.to_string()))?;
            objects.push(obj);
        }

        if let Some(ref class_id) = query.class_id {
            objects.retain(|o| o.class_id.as_ref() == Some(class_id));
        }

        Ok(objects)
    }

    async fn find_relations_by_source(
        &self,
        source_id: &ObjectId,
    ) -> Result<Vec<RelationRecord>, StoreError> {
        let res = self
            .db
            .query("SELECT * FROM relations WHERE source_id = $source_id")
            .bind(("source_id", source_id.as_str()))
            .await
            .map_err(|e| StoreError::Generic(e.to_string()))?;
        let mut res = res
            .check()
            .map_err(|e| StoreError::Generic(e.to_string()))?;
        let values: Vec<serde_json::Value> = res
            .take(0)
            .map_err(|e| StoreError::Generic(e.to_string()))?;
        let mut relations = Vec::new();
        for v in values {
            let rel: RelationRecord =
                serde_json::from_value(v).map_err(|e| StoreError::Generic(e.to_string()))?;
            relations.push(rel);
        }
        Ok(relations)
    }

    async fn get_run(&self, id: &RunId) -> Result<RunRecord, StoreError> {
        let res: Option<serde_json::Value> = self
            .db
            .select(("runs", id.as_str()))
            .await
            .map_err(|e| StoreError::Generic(e.to_string()))?;
        let val = res.ok_or_else(|| StoreError::Generic(format!("Run {} not found", id)))?;
        serde_json::from_value(val).map_err(|e| StoreError::Generic(e.to_string()))
    }

    async fn get_dispatch(&self, id: &DispatchId) -> Result<DispatchRecord, StoreError> {
        let res: Option<serde_json::Value> = self
            .db
            .select(("dispatches", id.as_str()))
            .await
            .map_err(|e| StoreError::Generic(e.to_string()))?;
        let val = res.ok_or_else(|| StoreError::Generic(format!("Dispatch {} not found", id)))?;
        serde_json::from_value(val).map_err(|e| StoreError::Generic(e.to_string()))
    }

    async fn get_packet(&self, id: &PacketId) -> Result<PacketRecord, StoreError> {
        let res: Option<serde_json::Value> = self
            .db
            .select(("packets", id.as_str()))
            .await
            .map_err(|e| StoreError::Generic(e.to_string()))?;
        let val = res.ok_or_else(|| StoreError::Generic(format!("Packet {} not found", id)))?;
        serde_json::from_value(val).map_err(|e| StoreError::Generic(e.to_string()))
    }

    async fn get_handoff(
        &self,
        id: &HandoffManifestId,
    ) -> Result<HandoffManifestRecord, StoreError> {
        let res: Option<serde_json::Value> = self
            .db
            .select(("handoffs", id.as_str()))
            .await
            .map_err(|e| StoreError::Generic(e.to_string()))?;
        let val = res.ok_or_else(|| StoreError::Generic(format!("Handoff {} not found", id)))?;
        serde_json::from_value(val).map_err(|e| StoreError::Generic(e.to_string()))
    }

    async fn get_review(&self, id: &ReviewId) -> Result<ReviewRecord, StoreError> {
        let res: Option<serde_json::Value> = self
            .db
            .select(("reviews", id.as_str()))
            .await
            .map_err(|e| StoreError::Generic(e.to_string()))?;
        let val = res.ok_or_else(|| StoreError::Generic(format!("Review {} not found", id)))?;
        serde_json::from_value(val).map_err(|e| StoreError::Generic(e.to_string()))
    }

    async fn list_active_claims(&self) -> Result<Vec<DispatchRecord>, StoreError> {
        let mut res = self
            .db
            .query("SELECT * FROM dispatches WHERE claimed_by IS NOT NULL")
            .await
            .map_err(|e| StoreError::Generic(e.to_string()))?;
        let values: Vec<serde_json::Value> = res
            .take(0)
            .map_err(|e| StoreError::Generic(e.to_string()))?;
        let mut results = Vec::new();
        for v in values {
            results
                .push(serde_json::from_value(v).map_err(|e| StoreError::Generic(e.to_string()))?);
        }
        Ok(results)
    }

    async fn list_expired_leases(&self) -> Result<Vec<DispatchRecord>, StoreError> {
        // Simple implementation for now
        Ok(vec![])
    }

    async fn upsert_object(&self, object: &ObjectRecord) -> Result<(), StoreError> {
        self.upsert_object_internal(object).await
    }

    async fn upsert_relation(&self, relation: &RelationRecord) -> Result<(), StoreError> {
        self.upsert_relation_internal(relation).await
    }
}
