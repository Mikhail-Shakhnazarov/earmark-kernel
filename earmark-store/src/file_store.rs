/*
 * Copyright (c) 2026 Mikhail Shakhnazarov.
 * Dual-licensed under AGPL-3.0-or-later or commercial terms.
 */

use crate::errors::StoreError;
use crate::traits::CanonicalStore;
use chrono::Utc;
use earmark_core::{
    ActorId, ChangeSetId, ChangeSetRecord, CheckResultId, CheckResultRecord, DispatchId,
    DispatchRecord, DispatchStatus, ExternalConnectionRecord, HandoffManifestId,
    HandoffManifestRecord, ObjectId, ObjectRecord, PacketId, PacketRecord, ProviderProfile,
    ProviderProfileId, ProviderRecord, RelationId, RelationRecord, ReviewId, ReviewRecord, RunId,
    RunRecord, StandingTransitionRecord, UndoRecord, VersionId, VersionRecord, WorkerProfile,
    WorkerProfileId,
};
use serde::Serialize;
use std::fs;
use std::path::PathBuf;

pub struct FileStore {
    root: PathBuf,
}

impl FileStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn dot_earmark(&self) -> PathBuf {
        self.root.join(".earmark")
    }

    fn objects_dir(&self) -> PathBuf {
        self.dot_earmark().join("objects")
    }
    fn relations_dir(&self) -> PathBuf {
        self.dot_earmark().join("relations")
    }
    fn runs_dir(&self) -> PathBuf {
        self.dot_earmark().join("runs")
    }
    fn packets_dir(&self) -> PathBuf {
        self.dot_earmark().join("packets")
    }
    fn dispatches_dir(&self) -> PathBuf {
        self.dot_earmark().join("dispatches")
    }
    fn change_sets_dir(&self) -> PathBuf {
        self.dot_earmark().join("change_sets")
    }
    fn check_results_dir(&self) -> PathBuf {
        self.dot_earmark().join("check_results")
    }
    fn handoff_manifests_dir(&self) -> PathBuf {
        self.dot_earmark().join("handoff_manifests")
    }
    fn reviews_dir(&self) -> PathBuf {
        self.dot_earmark().join("reviews")
    }
    fn standing_transitions_dir(&self) -> PathBuf {
        self.dot_earmark().join("standing_transitions")
    }
    fn provider_profiles_dir(&self) -> PathBuf {
        self.dot_earmark().join("provider_profiles")
    }
    fn provider_records_dir(&self) -> PathBuf {
        self.dot_earmark().join("provider_records")
    }
    fn external_connections_dir(&self) -> PathBuf {
        self.dot_earmark().join("external_connections")
    }
    fn worker_profiles_dir(&self) -> PathBuf {
        self.dot_earmark().join("worker_profiles")
    }
    fn system_packs_dir(&self) -> PathBuf {
        self.root.join(".earmark/declarations/packs")
    }

    fn classes_dir(&self) -> PathBuf {
        self.root.join(".earmark/declarations/classes")
    }

    fn systems_dir(&self) -> PathBuf {
        self.root.join(".earmark/declarations/systems")
    }

    fn workflows_dir(&self) -> PathBuf {
        self.root.join(".earmark/declarations/workflows")
    }

    fn selection_policies_dir(&self) -> PathBuf {
        self.root.join(".earmark/declarations/selection_policies")
    }

    fn packet_templates_dir(&self) -> PathBuf {
        self.root.join(".earmark/declarations/packet_templates")
    }

    fn runtime_protocols_dir(&self) -> PathBuf {
        self.root.join(".earmark/declarations/runtime_protocols")
    }

    fn pack_activations_dir(&self) -> PathBuf {
        self.dot_earmark().join("pack_activations")
    }
    fn undo_history_dir(&self) -> PathBuf {
        self.dot_earmark().join("undo_history")
    }
    fn migrations_dir(&self) -> PathBuf {
        self.dot_earmark().join("migrations")
    }

    fn save<T: Serialize>(&self, path: PathBuf, record: &T) -> Result<(), StoreError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(record)?;
        fs::write(path, json)?;
        Ok(())
    }

    fn sanction_write(&self) -> Result<(), StoreError> {
        // In a production system, this would also check actor permissions.
        // For the kernel foundation, we focus on the record-integrity gate.
        let violations = self.verify_regression_gate()?;
        if !violations.is_empty() {
            return Err(StoreError::Regression(violations.join("; ")));
        }
        Ok(())
    }
}

impl CanonicalStore for FileStore {
    fn is_initialized(&self) -> bool {
        self.dot_earmark().exists()
    }

    fn init(&self) -> Result<(), StoreError> {
        fs::create_dir_all(self.objects_dir())?;
        fs::create_dir_all(self.relations_dir())?;
        fs::create_dir_all(self.runs_dir())?;
        fs::create_dir_all(self.packets_dir())?;
        fs::create_dir_all(self.dispatches_dir())?;
        fs::create_dir_all(self.change_sets_dir())?;
        fs::create_dir_all(self.check_results_dir())?;
        fs::create_dir_all(self.handoff_manifests_dir())?;
        fs::create_dir_all(self.reviews_dir())?;
        fs::create_dir_all(self.standing_transitions_dir())?;
        fs::create_dir_all(self.provider_profiles_dir())?;
        fs::create_dir_all(self.provider_records_dir())?;
        fs::create_dir_all(self.external_connections_dir())?;
        fs::create_dir_all(self.worker_profiles_dir())?;
        fs::create_dir_all(self.system_packs_dir())?;
        fs::create_dir_all(self.selection_policies_dir())?;
        fs::create_dir_all(self.packet_templates_dir())?;
        fs::create_dir_all(self.runtime_protocols_dir())?;
        fs::create_dir_all(self.pack_activations_dir())?;
        fs::create_dir_all(self.undo_history_dir())?;
        fs::create_dir_all(self.migrations_dir())?;
        Ok(())
    }

    fn verify_maturity_gate(&self) -> Result<Vec<String>, StoreError> {
        let mut violations = Vec::new();

        let object_ids = self.list_objects()?;
        let task_class_id = earmark_core::ClassId::parse("cls_work_item").unwrap();

        for id in object_ids {
            let obj = self.get_object(&id)?;
            if obj.class_id.as_ref() == Some(&task_class_id) {
                // Check current standing
                let standing_recs =
                    self.get_standing(&earmark_core::StandingTargetRef::Object(id.clone()))?;
                let is_completed = standing_recs
                    .iter()
                    .rfind(|r| r.dimension == "status")
                    .map(|r| r.to_token == "Completed")
                    .unwrap_or(false);

                if is_completed {
                    let version = self.get_version(&id, &obj.latest_version_id)?;
                    let has_accepted_signal = version
                        .signal
                        .as_ref()
                        .map(|s| s.signal_type == earmark_core::SignalType::Accepted)
                        .unwrap_or(false);

                    if !has_accepted_signal {
                        // Check for administrative override in rationale
                        let has_admin_bypass = standing_recs
                            .iter()
                            .rfind(|r| r.dimension == "status" && r.to_token == "Completed")
                            .map(|r| {
                                r.rationale.to_lowercase().contains("[admin]")
                                    || r.rationale.to_lowercase().contains("[maintenance]")
                            })
                            .unwrap_or(false);

                        if !has_admin_bypass {
                            violations.push(format!("Task {} is status:Completed but lacks Accepted signal and administrative override rationale", id.as_str()));
                        }
                    }
                }
            }
        }

        Ok(violations)
    }

    fn verify_regression_gate(&self) -> Result<Vec<String>, StoreError> {
        let mut violations = self.verify_consistency()?;
        violations.extend(self.verify_maturity_gate()?);
        Ok(violations)
    }

    fn verify_consistency(&self) -> Result<Vec<String>, StoreError> {
        let mut violations = Vec::new();

        // 1. Audit Objects and Versions
        let object_ids = self.list_objects()?;
        for id in object_ids {
            let obj = match self.get_object(&id) {
                Ok(obj) => obj,
                Err(e) => {
                    violations.push(format!(
                        "Object {} record missing or corrupt: {}",
                        id.as_str(),
                        e
                    ));
                    continue;
                }
            };

            // Check latest version existence
            let ver_path = self
                .objects_dir()
                .join(id.as_str())
                .join("versions")
                .join(obj.latest_version_id.as_str())
                .join("record.json");
            if !ver_path.exists() {
                violations.push(format!(
                    "Object {} latest version {} missing at {:?}",
                    id.as_str(),
                    obj.latest_version_id.as_str(),
                    ver_path
                ));
            }
        }

        // 2. Audit Relations
        let relation_ids = self.list_all_relations()?;
        for rid in relation_ids {
            let rel = match self.get_relation(&rid) {
                Ok(rel) => rel,
                Err(e) => {
                    violations.push(format!(
                        "Relation {} record missing or corrupt: {}",
                        rid.as_str(),
                        e
                    ));
                    continue;
                }
            };

            // Check source
            let source_path = self
                .objects_dir()
                .join(rel.source_id.as_str())
                .join("record.json");
            if !source_path.exists() {
                violations.push(format!(
                    "Relation {} points to missing source {}",
                    rid.as_str(),
                    rel.source_id.as_str()
                ));
            }

            // Check target
            let target_path = self
                .objects_dir()
                .join(rel.target_id.as_str())
                .join("record.json");
            if !target_path.exists() {
                violations.push(format!(
                    "Relation {} points to missing target {}",
                    rid.as_str(),
                    rel.target_id.as_str()
                ));
            }
        }

        // 3. Audit Runs, Packets, Dispatches
        for rid in self.list_runs()? {
            if let Err(e) = self.get_run(&rid) {
                violations.push(format!("Run {} corrupt: {}", rid.as_str(), e));
            }
        }
        for pid in self.list_packets()? {
            if let Err(e) = self.get_packet(&pid) {
                violations.push(format!("Packet {} corrupt: {}", pid.as_str(), e));
            }
        }
        for did in self.list_dispatches()? {
            if let Err(e) = self.get_dispatch(&did) {
                violations.push(format!("Dispatch {} corrupt: {}", did.as_str(), e));
            }
        }
        for rid in self.list_reviews()? {
            if let Err(e) = self.get_review(&rid) {
                violations.push(format!("Review {} corrupt: {}", rid.as_str(), e));
            }
        }

        Ok(violations)
    }

    fn deposit_object(
        &self,
        record: ObjectRecord,
        version: VersionRecord,
    ) -> Result<(), StoreError> {
        self.sanction_write()?;
        let obj_dir = self.objects_dir().join(record.id.as_str());
        let ver_dir = obj_dir.join("versions").join(version.version_id.as_str());
        fs::create_dir_all(&ver_dir)?;
        self.save(obj_dir.join("record.json"), &record)?;
        self.save(ver_dir.join("record.json"), &version)?;
        Ok(())
    }

    fn get_object(&self, id: &ObjectId) -> Result<ObjectRecord, StoreError> {
        let path = self.objects_dir().join(id.as_str()).join("record.json");
        if !path.exists() {
            return Err(StoreError::ObjectNotFound(id.as_str().to_string()));
        }
        let json = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&json)?)
    }

    fn get_version(
        &self,
        id: &ObjectId,
        version_id: &VersionId,
    ) -> Result<VersionRecord, StoreError> {
        let path = self
            .objects_dir()
            .join(id.as_str())
            .join("versions")
            .join(version_id.as_str())
            .join("record.json");
        if !path.exists() {
            return Err(StoreError::VersionNotFound(version_id.as_str().to_string()));
        }
        let json = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&json)?)
    }

    fn update_version(&self, version: VersionRecord) -> Result<(), StoreError> {
        self.sanction_write()?;
        let path = self
            .objects_dir()
            .join(version.object_id.as_str())
            .join("versions")
            .join(version.version_id.as_str())
            .join("record.json");
        if !path.exists() {
            return Err(StoreError::VersionNotFound(
                version.version_id.as_str().to_string(),
            ));
        }
        self.save(path, &version)?;
        Ok(())
    }

    fn list_versions(&self, id: &ObjectId) -> Result<Vec<VersionId>, StoreError> {
        let versions_dir = self.objects_dir().join(id.as_str()).join("versions");
        if !versions_dir.exists() {
            return Ok(vec![]);
        }
        let mut versions = Vec::new();
        for entry in fs::read_dir(versions_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                versions.push(VersionId::parse(&name)?);
            }
        }
        Ok(versions)
    }

    fn list_objects(&self) -> Result<Vec<ObjectId>, StoreError> {
        let mut objects = Vec::new();
        if !self.objects_dir().exists() {
            return Ok(objects);
        }
        for entry in fs::read_dir(self.objects_dir())? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with("obj_") {
                    objects.push(ObjectId::parse(&name)?);
                }
            }
        }
        Ok(objects)
    }

    fn list_objects_by_class(
        &self,
        class_id: &earmark_core::ClassId,
    ) -> Result<Vec<ObjectId>, StoreError> {
        let mut objects = Vec::new();
        if !self.objects_dir().exists() {
            return Ok(objects);
        }
        for entry in fs::read_dir(self.objects_dir())? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with("obj_") {
                    let id = ObjectId::parse(&name)?;
                    let record = self.get_object(&id)?;
                    if record.class_id.as_ref() == Some(class_id) {
                        objects.push(id);
                    }
                }
            }
        }
        Ok(objects)
    }

    fn create_relation(&self, record: RelationRecord) -> Result<(), StoreError> {
        self.sanction_write()?;
        let path = self
            .relations_dir()
            .join(format!("{}.json", record.id.as_str()));
        self.save(path, &record)
    }

    fn get_relation(&self, id: &RelationId) -> Result<RelationRecord, StoreError> {
        let path = self.relations_dir().join(format!("{}.json", id.as_str()));
        let json = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&json)?)
    }

    fn list_relations(&self, _source_id: &ObjectId) -> Result<Vec<RelationRecord>, StoreError> {
        // Simple scan for now, WP4 will optimize with indexes
        let mut relations = Vec::new();
        if !self.relations_dir().exists() {
            return Ok(relations);
        }
        for entry in fs::read_dir(self.relations_dir())? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                let json = fs::read_to_string(entry.path())?;
                let record: RelationRecord = serde_json::from_str(&json)?;
                if record.source_id == *_source_id {
                    relations.push(record);
                }
            }
        }
        Ok(relations)
    }

    fn list_all_relations(&self) -> Result<Vec<RelationId>, StoreError> {
        let mut relations = Vec::new();
        if !self.relations_dir().exists() {
            return Ok(relations);
        }
        for entry in fs::read_dir(self.relations_dir())? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.ends_with(".json") {
                    let id = name.trim_end_matches(".json");
                    relations.push(RelationId::parse(id)?);
                }
            }
        }
        Ok(relations)
    }

    fn create_run(&self, record: RunRecord) -> Result<(), StoreError> {
        self.sanction_write()?;
        let path = self
            .runs_dir()
            .join(format!("{}.json", record.run_id.as_str()));
        self.save(path, &record)
    }

    fn update_run(&self, record: RunRecord) -> Result<(), StoreError> {
        self.create_run(record)
    }

    fn get_run(&self, id: &RunId) -> Result<RunRecord, StoreError> {
        let path = self.runs_dir().join(format!("{}.json", id.as_str()));
        let json = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&json)?)
    }

    fn list_runs(&self) -> Result<Vec<RunId>, StoreError> {
        let mut runs = Vec::new();
        if !self.runs_dir().exists() {
            return Ok(runs);
        }
        for entry in fs::read_dir(self.runs_dir())? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.ends_with(".json") {
                    let id = name.trim_end_matches(".json");
                    runs.push(RunId::parse(id)?);
                }
            }
        }
        Ok(runs)
    }

    fn create_packet(&self, record: PacketRecord) -> Result<(), StoreError> {
        self.sanction_write()?;
        let path = self
            .packets_dir()
            .join(format!("{}.json", record.packet_id.as_str()));
        self.save(path, &record)
    }

    fn get_packet(&self, id: &PacketId) -> Result<PacketRecord, StoreError> {
        let path = self.packets_dir().join(format!("{}.json", id.as_str()));
        let json = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&json)?)
    }

    fn list_packets(&self) -> Result<Vec<PacketId>, StoreError> {
        let mut packets = Vec::new();
        if !self.packets_dir().exists() {
            return Ok(packets);
        }
        for entry in fs::read_dir(self.packets_dir())? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.ends_with(".json") {
                    let id = name.trim_end_matches(".json");
                    packets.push(PacketId::parse(id)?);
                }
            }
        }
        Ok(packets)
    }

    fn create_dispatch(&self, record: DispatchRecord) -> Result<(), StoreError> {
        self.sanction_write()?;
        let path = self
            .dispatches_dir()
            .join(format!("{}.json", record.dispatch_id.as_str()));
        self.save(path, &record)
    }

    fn update_dispatch(&self, record: DispatchRecord) -> Result<(), StoreError> {
        self.create_dispatch(record)
    }

    fn get_dispatch(&self, id: &DispatchId) -> Result<DispatchRecord, StoreError> {
        let path = self.dispatches_dir().join(format!("{}.json", id.as_str()));
        let json = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&json)?)
    }

    fn list_dispatches(&self) -> Result<Vec<DispatchId>, StoreError> {
        let mut dispatches = Vec::new();
        if !self.dispatches_dir().exists() {
            return Ok(dispatches);
        }
        for entry in fs::read_dir(self.dispatches_dir())? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.ends_with(".json") {
                    let id = name.trim_end_matches(".json");
                    dispatches.push(DispatchId::parse(id)?);
                }
            }
        }
        Ok(dispatches)
    }

    fn create_change_set(&self, record: ChangeSetRecord) -> Result<(), StoreError> {
        self.sanction_write()?;
        let path = self
            .change_sets_dir()
            .join(format!("{}.json", record.change_set_id.as_str()));
        self.save(path, &record)
    }

    fn get_change_set(&self, id: &ChangeSetId) -> Result<ChangeSetRecord, StoreError> {
        let path = self.change_sets_dir().join(format!("{}.json", id.as_str()));
        let json = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&json)?)
    }

    fn create_check_result(&self, record: CheckResultRecord) -> Result<(), StoreError> {
        self.sanction_write()?;
        let path = self
            .check_results_dir()
            .join(format!("{}.json", record.check_result_id.as_str()));
        self.save(path, &record)
    }

    fn get_check_result(&self, id: &CheckResultId) -> Result<CheckResultRecord, StoreError> {
        let path = self
            .check_results_dir()
            .join(format!("{}.json", id.as_str()));
        let json = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&json)?)
    }

    fn create_handoff_manifest(&self, record: HandoffManifestRecord) -> Result<(), StoreError> {
        self.sanction_write()?;
        let path = self
            .handoff_manifests_dir()
            .join(format!("{}.json", record.handoff_manifest_id.as_str()));
        self.save(path, &record)
    }

    fn get_handoff_manifest(
        &self,
        id: &HandoffManifestId,
    ) -> Result<HandoffManifestRecord, StoreError> {
        let path = self
            .handoff_manifests_dir()
            .join(format!("{}.json", id.as_str()));
        let json = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&json)?)
    }

    fn create_review(&self, record: ReviewRecord) -> Result<(), StoreError> {
        self.sanction_write()?;
        let path = self
            .reviews_dir()
            .join(format!("{}.json", record.review_id.as_str()));
        self.save(path, &record)
    }

    fn get_review(&self, id: &ReviewId) -> Result<ReviewRecord, StoreError> {
        let path = self.reviews_dir().join(format!("{}.json", id.as_str()));
        let json = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&json)?)
    }

    fn list_reviews(&self) -> Result<Vec<ReviewId>, StoreError> {
        let mut reviews = Vec::new();
        if !self.reviews_dir().exists() {
            return Ok(reviews);
        }
        for entry in fs::read_dir(self.reviews_dir())? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.ends_with(".json") {
                    let id = name.trim_end_matches(".json");
                    reviews.push(ReviewId::parse(id)?);
                }
            }
        }
        Ok(reviews)
    }

    fn record_standing_transition(
        &self,
        record: StandingTransitionRecord,
    ) -> Result<(), StoreError> {
        self.sanction_write()?;
        let path = self
            .standing_transitions_dir()
            .join(format!("{}.json", record.transition_record_id));
        self.save(path, &record)
    }

    fn get_standing(
        &self,
        target: &earmark_core::StandingTargetRef,
    ) -> Result<Vec<StandingTransitionRecord>, StoreError> {
        let mut transitions = Vec::new();
        if !self.standing_transitions_dir().exists() {
            return Ok(transitions);
        }

        for entry in fs::read_dir(self.standing_transitions_dir())? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                let json = fs::read_to_string(entry.path())?;
                let record: StandingTransitionRecord = serde_json::from_str(&json)?;
                if record.target_ref == *target {
                    transitions.push(record);
                }
            }
        }

        // Filter for latest per dimension
        use std::collections::HashMap;
        let mut latest_per_dim: HashMap<String, StandingTransitionRecord> = HashMap::new();
        for trans in transitions {
            let entry = latest_per_dim
                .entry(trans.dimension.clone())
                .or_insert_with(|| trans.clone());
            if trans.created_at > entry.created_at {
                *entry = trans;
            }
        }

        let mut result: Vec<StandingTransitionRecord> = latest_per_dim.into_values().collect();
        result.sort_by(|a, b| a.dimension.cmp(&b.dimension));
        Ok(result)
    }

    fn register_provider_profile(&self, record: ProviderProfile) -> Result<(), StoreError> {
        self.sanction_write()?;
        let path = self
            .provider_profiles_dir()
            .join(format!("{}.json", record.provider_profile_id.as_str()));
        self.save(path, &record)
    }

    fn get_provider_profile(&self, id: &ProviderProfileId) -> Result<ProviderProfile, StoreError> {
        let path = self
            .provider_profiles_dir()
            .join(format!("{}.json", id.as_str()));
        let json = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&json)?)
    }

    fn list_provider_profiles(&self) -> Result<Vec<ProviderProfileId>, StoreError> {
        let mut profiles = Vec::new();
        if !self.provider_profiles_dir().exists() {
            return Ok(profiles);
        }
        for entry in fs::read_dir(self.provider_profiles_dir())? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.ends_with(".json") {
                    let id = name.trim_end_matches(".json");
                    profiles.push(ProviderProfileId::parse(id)?);
                }
            }
        }
        Ok(profiles)
    }

    fn record_provider_call(&self, record: ProviderRecord) -> Result<(), StoreError> {
        self.sanction_write()?;
        let path = self
            .provider_records_dir()
            .join(format!("{}.json", record.provider_record_id));
        self.save(path, &record)
    }

    fn create_external_connection(
        &self,
        record: ExternalConnectionRecord,
    ) -> Result<(), StoreError> {
        self.sanction_write()?;
        let path = self
            .external_connections_dir()
            .join(format!("{}.json", record.connection_id.as_str()));
        self.save(path, &record)
    }

    fn register_worker_profile(&self, record: WorkerProfile) -> Result<(), StoreError> {
        self.sanction_write()?;
        let path = self
            .worker_profiles_dir()
            .join(format!("{}.json", record.worker_profile_id.as_str()));
        self.save(path, &record)
    }

    fn get_worker_profile(&self, id: &WorkerProfileId) -> Result<WorkerProfile, StoreError> {
        let path = self
            .worker_profiles_dir()
            .join(format!("{}.json", id.as_str()));
        let json = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&json)?)
    }

    fn list_worker_profiles(&self) -> Result<Vec<WorkerProfileId>, StoreError> {
        let mut profiles = Vec::new();
        if !self.worker_profiles_dir().exists() {
            return Ok(profiles);
        }
        for entry in fs::read_dir(self.worker_profiles_dir())? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.ends_with(".json") {
                    let id = name.trim_end_matches(".json");
                    profiles.push(WorkerProfileId::parse(id)?);
                }
            }
        }
        Ok(profiles)
    }

    // Declaration Management
    fn register_class(&self, record: earmark_core::ClassDeclaration) -> Result<(), StoreError> {
        self.sanction_write()?;
        let path = self
            .classes_dir()
            .join(format!("{}.json", record.class_id.as_str()));
        self.save(path, &record)
    }

    fn get_class(
        &self,
        id: &earmark_core::ClassId,
    ) -> Result<earmark_core::ClassDeclaration, StoreError> {
        let path = self.classes_dir().join(format!("{}.json", id.as_str()));
        let json = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&json)?)
    }

    fn list_classes(&self) -> Result<Vec<earmark_core::ClassId>, StoreError> {
        let mut ids = Vec::new();
        if !self.classes_dir().exists() {
            return Ok(ids);
        }
        for entry in fs::read_dir(self.classes_dir())? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.ends_with(".json") {
                    let id = name.trim_end_matches(".json");
                    ids.push(earmark_core::ClassId::parse(id)?);
                }
            }
        }
        Ok(ids)
    }

    fn register_system(&self, record: earmark_core::SystemDeclaration) -> Result<(), StoreError> {
        self.sanction_write()?;
        let path = self
            .systems_dir()
            .join(format!("{}.json", record.system_id.as_str()));
        self.save(path, &record)
    }

    fn get_system(
        &self,
        id: &earmark_core::SystemId,
    ) -> Result<earmark_core::SystemDeclaration, StoreError> {
        let path = self.systems_dir().join(format!("{}.json", id.as_str()));
        let json = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&json)?)
    }

    fn list_systems(&self) -> Result<Vec<earmark_core::SystemId>, StoreError> {
        let mut ids = Vec::new();
        if !self.systems_dir().exists() {
            return Ok(ids);
        }
        for entry in fs::read_dir(self.systems_dir())? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.ends_with(".json") {
                    let id = name.trim_end_matches(".json");
                    ids.push(earmark_core::SystemId::parse(id)?);
                }
            }
        }
        Ok(ids)
    }

    fn register_workflow(
        &self,
        record: earmark_core::WorkflowDeclaration,
    ) -> Result<(), StoreError> {
        self.sanction_write()?;
        let path = self
            .workflows_dir()
            .join(format!("{}.json", record.workflow_id.as_str()));
        self.save(path, &record)
    }

    fn get_workflow(
        &self,
        id: &earmark_core::WorkflowId,
    ) -> Result<earmark_core::WorkflowDeclaration, StoreError> {
        let path = self.workflows_dir().join(format!("{}.json", id.as_str()));
        let json = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&json)?)
    }

    fn list_workflows(&self) -> Result<Vec<earmark_core::WorkflowId>, StoreError> {
        let mut ids = Vec::new();
        if !self.workflows_dir().exists() {
            return Ok(ids);
        }
        for entry in fs::read_dir(self.workflows_dir())? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.ends_with(".json") {
                    let id = name.trim_end_matches(".json");
                    ids.push(earmark_core::WorkflowId::parse(id)?);
                }
            }
        }
        Ok(ids)
    }

    fn register_packet_template(
        &self,
        record: earmark_core::PacketTemplateDeclaration,
    ) -> Result<(), StoreError> {
        self.sanction_write()?;
        let path = self
            .packet_templates_dir()
            .join(format!("{}.json", record.packet_template_id.as_str()));
        self.save(path, &record)
    }

    fn get_packet_template(
        &self,
        id: &earmark_core::PacketTemplateId,
    ) -> Result<earmark_core::PacketTemplateDeclaration, StoreError> {
        let path = self
            .packet_templates_dir()
            .join(format!("{}.json", id.as_str()));
        let json = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&json)?)
    }

    fn list_packet_templates(&self) -> Result<Vec<earmark_core::PacketTemplateId>, StoreError> {
        let mut ids = Vec::new();
        if !self.packet_templates_dir().exists() {
            return Ok(ids);
        }
        for entry in fs::read_dir(self.packet_templates_dir())? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.ends_with(".json") {
                    let id = name.trim_end_matches(".json");
                    ids.push(earmark_core::PacketTemplateId::parse(id)?);
                }
            }
        }
        Ok(ids)
    }

    fn register_runtime_protocol(
        &self,
        record: earmark_core::RuntimeProtocol,
    ) -> Result<(), StoreError> {
        self.sanction_write()?;
        let path = self
            .runtime_protocols_dir()
            .join(format!("{}.json", record.protocol_id.as_str()));
        self.save(path, &record)
    }

    fn get_runtime_protocol(
        &self,
        id: &earmark_core::RuntimeProtocolId,
    ) -> Result<earmark_core::RuntimeProtocol, StoreError> {
        let path = self
            .runtime_protocols_dir()
            .join(format!("{}.json", id.as_str()));
        let json = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&json)?)
    }

    fn list_runtime_protocols(&self) -> Result<Vec<earmark_core::RuntimeProtocolId>, StoreError> {
        let mut ids = Vec::new();
        if !self.runtime_protocols_dir().exists() {
            return Ok(ids);
        }
        for entry in fs::read_dir(self.runtime_protocols_dir())? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.ends_with(".json") {
                    let id = name.trim_end_matches(".json");
                    ids.push(earmark_core::RuntimeProtocolId::parse(id)?);
                }
            }
        }
        Ok(ids)
    }

    fn register_selection_policy(
        &self,
        record: earmark_core::SelectionPolicy,
    ) -> Result<(), StoreError> {
        self.sanction_write()?;
        let path = self
            .selection_policies_dir()
            .join(format!("{}.json", record.selection_id.as_str()));
        self.save(path, &record)
    }

    fn get_selection_policy(
        &self,
        id: &earmark_core::SelectionPolicyId,
    ) -> Result<earmark_core::SelectionPolicy, StoreError> {
        let path = self
            .selection_policies_dir()
            .join(format!("{}.json", id.as_str()));
        let json = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&json)?)
    }

    fn list_selection_policies(&self) -> Result<Vec<earmark_core::SelectionPolicyId>, StoreError> {
        let mut ids = Vec::new();
        if !self.selection_policies_dir().exists() {
            return Ok(ids);
        }
        for entry in fs::read_dir(self.selection_policies_dir())? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.ends_with(".json") {
                    let id = name.trim_end_matches(".json");
                    ids.push(earmark_core::SelectionPolicyId::parse(id)?);
                }
            }
        }
        Ok(ids)
    }

    fn register_system_pack(
        &self,
        record: earmark_core::SystemPackManifest,
    ) -> Result<(), StoreError> {
        self.sanction_write()?;
        let path = self
            .system_packs_dir()
            .join(format!("{}.json", record.pack_id.as_str()));
        self.save(path, &record)
    }

    fn get_system_pack(
        &self,
        id: &earmark_core::SystemPackId,
    ) -> Result<earmark_core::SystemPackManifest, StoreError> {
        let path = self
            .system_packs_dir()
            .join(format!("{}.json", id.as_str()));
        let json = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&json)?)
    }

    fn list_system_packs(&self) -> Result<Vec<earmark_core::SystemPackId>, StoreError> {
        let mut packs = Vec::new();
        if !self.system_packs_dir().exists() {
            return Ok(packs);
        }
        for entry in fs::read_dir(self.system_packs_dir())? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.ends_with(".json") {
                    let id = name.trim_end_matches(".json");
                    packs.push(earmark_core::SystemPackId::parse(id)?);
                }
            }
        }
        Ok(packs)
    }

    fn record_pack_activation(
        &self,
        record: earmark_core::PackActivationRecord,
    ) -> Result<(), StoreError> {
        self.sanction_write()?;
        let dir = self.pack_activations_dir().join(record.pack_id.as_str());
        fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{}.json", record.created_at.timestamp()));
        self.save(path, &record)
    }

    fn get_pack_activation_history(
        &self,
        id: &earmark_core::SystemPackId,
    ) -> Result<Vec<earmark_core::PackActivationRecord>, StoreError> {
        let dir = self.pack_activations_dir().join(id.as_str());
        if !dir.exists() {
            return Ok(vec![]);
        }
        let mut history = Vec::new();
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                let json = fs::read_to_string(entry.path())?;
                let record: earmark_core::PackActivationRecord = serde_json::from_str(&json)?;
                history.push(record);
            }
        }
        history.sort_by_key(|a| a.created_at);
        Ok(history)
    }

    fn record_undo(&self, record: UndoRecord) -> Result<(), StoreError> {
        self.sanction_write()?;
        let dir = self.undo_history_dir().join(record.target_ref.id.as_str());
        fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{}.json", record.undo_id));
        self.save(path, &record)
    }

    fn get_undo_history(
        &self,
        target: &earmark_core::ObjectRef,
    ) -> Result<Vec<UndoRecord>, StoreError> {
        let dir = self.undo_history_dir().join(target.id.as_str());
        if !dir.exists() {
            return Ok(vec![]);
        }
        let mut history = Vec::new();
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                let json = fs::read_to_string(entry.path())?;
                let record: UndoRecord = serde_json::from_str(&json)?;
                history.push(record);
            }
        }
        Ok(history)
    }

    fn record_migration(&self, record: earmark_core::MigrationRecord) -> Result<(), StoreError> {
        self.sanction_write()?;
        let dir = self.migrations_dir();
        fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{}.json", record.migration_id));
        self.save(path, &record)
    }

    fn get_migration_history(&self) -> Result<Vec<earmark_core::MigrationRecord>, StoreError> {
        let mut history = Vec::new();
        if !self.migrations_dir().exists() {
            return Ok(history);
        }
        for entry in fs::read_dir(self.migrations_dir())? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                let json = fs::read_to_string(entry.path())?;
                let record: earmark_core::MigrationRecord = serde_json::from_str(&json)?;
                history.push(record);
            }
        }
        Ok(history)
    }

    fn claim_dispatch(
        &self,
        id: &DispatchId,
        claimer: ActorId,
        lease_duration_secs: i64,
    ) -> Result<DispatchRecord, StoreError> {
        let mut dispatch = self.get_dispatch(id)?;

        if dispatch.status != DispatchStatus::Queued {
            return Err(StoreError::AlreadyClaimed(format!(
                "Dispatch {} is not in Queued state (status: {:?})",
                id.as_str(),
                dispatch.status
            )));
        }

        dispatch.status = DispatchStatus::Running;
        dispatch.claimed_by = Some(claimer);
        dispatch.claimed_at = Some(Utc::now());
        dispatch.lease_expires_at =
            Some(Utc::now() + chrono::Duration::seconds(lease_duration_secs));
        dispatch.updated_at = Utc::now();

        self.update_dispatch(dispatch.clone())?;
        Ok(dispatch)
    }

    fn release_dispatch_claim(
        &self,
        id: &DispatchId,
        claimer: &ActorId,
    ) -> Result<DispatchRecord, StoreError> {
        let mut dispatch = self.get_dispatch(id)?;

        if dispatch.claimed_by.as_ref() != Some(claimer) {
            return Err(StoreError::NotClaimed(format!(
                "Dispatch {} is not claimed by {}",
                id.as_str(),
                claimer.as_str()
            )));
        }

        dispatch.claimed_by = None;
        dispatch.claimed_at = None;
        dispatch.lease_expires_at = None;
        dispatch.updated_at = Utc::now();

        self.update_dispatch(dispatch.clone())?;
        Ok(dispatch)
    }

    fn renew_lease(
        &self,
        id: &DispatchId,
        claimer: &ActorId,
        lease_duration_secs: i64,
    ) -> Result<DispatchRecord, StoreError> {
        let mut dispatch = self.get_dispatch(id)?;

        if dispatch.claimed_by.as_ref() != Some(claimer) {
            return Err(StoreError::NotClaimed(format!(
                "Dispatch {} is not claimed by {}",
                id.as_str(),
                claimer.as_str()
            )));
        }

        if let Some(expires) = dispatch.lease_expires_at {
            if expires < Utc::now() {
                return Err(StoreError::LeaseExpired(format!(
                    "Lease for dispatch {} expired at {}",
                    id.as_str(),
                    expires
                )));
            }
        }

        dispatch.lease_expires_at =
            Some(Utc::now() + chrono::Duration::seconds(lease_duration_secs));
        dispatch.updated_at = Utc::now();

        self.update_dispatch(dispatch.clone())?;
        Ok(dispatch)
    }

    fn reap_expired_leases(&self) -> Result<Vec<DispatchId>, StoreError> {
        let mut reaped = Vec::new();
        let now = Utc::now();

        for did in self.list_dispatches()? {
            let dispatch = self.get_dispatch(&did)?;
            if dispatch.status == earmark_core::DispatchStatus::Running {
                if let Some(expires) = dispatch.lease_expires_at {
                    if expires < now {
                        let mut dispatch = dispatch;
                        dispatch.status = earmark_core::DispatchStatus::Queued;
                        dispatch.claimed_by = None;
                        dispatch.claimed_at = None;
                        dispatch.lease_expires_at = None;
                        dispatch.updated_at = now;
                        self.update_dispatch(dispatch)?;
                        reaped.push(did);
                    }
                }
            }
        }

        Ok(reaped)
    }


    fn import_archive(
        &self,
        archive: earmark_core::records::archive::WorkspaceArchive,
        overwrite: bool,
    ) -> Result<(), StoreError> {
        // 1. Declarations
        for class in archive.classes {
            let path = self.classes_dir().join(format!("{}.json", class.class_id.as_str()));
            if overwrite || !path.exists() {
                self.save(path, &class)?;
            }
        }
        for system in archive.systems {
            let path = self.systems_dir().join(format!("{}.json", system.system_id.as_str()));
            if overwrite || !path.exists() {
                self.save(path, &system)?;
            }
        }
        for workflow in archive.workflows {
            let path = self.workflows_dir().join(format!("{}.json", workflow.workflow_id.as_str()));
            if overwrite || !path.exists() {
                self.save(path, &workflow)?;
            }
        }
        for pack in archive.system_packs {
            let path = self.system_packs_dir().join(format!("{}.json", pack.pack_id.as_str()));
            if overwrite || !path.exists() {
                self.save(path, &pack)?;
            }
        }

        // 2. Objects and Versions - Direct Save to bypass sanction_write O(N^2)
        for (obj, versions) in archive.objects {
            if !overwrite && self.objects_dir().join(obj.id.as_str()).exists() {
                continue;
            }
            let obj_dir = self.objects_dir().join(obj.id.as_str());
            self.save(obj_dir.join("record.json"), &obj)?;
            for version in versions {
                let ver_dir = obj_dir.join("versions").join(version.version_id.as_str());
                self.save(ver_dir.join("record.json"), &version)?;
            }
        }

        // 3. Relations
        for rel in archive.relations {
            self.save(self.relations_dir().join(format!("{}.json", rel.id.as_str())), &rel)?;
        }

        // 4. Runtime Records
        for run in archive.runs {
            self.save(self.runs_dir().join(format!("{}.json", run.run_id.as_str())), &run)?;
        }
        for packet in archive.packets {
            self.save(
                self.packets_dir().join(format!("{}.json", packet.packet_id.as_str())),
                &packet,
            )?;
        }
        for dispatch in archive.dispatches {
            self.save(
                self.dispatches_dir()
                    .join(format!("{}.json", dispatch.dispatch_id.as_str())),
                &dispatch,
            )?;
        }
        for review in archive.reviews {
            self.save(
                self.reviews_dir().join(format!("{}.json", review.review_id.as_str())),
                &review,
            )?;
        }
        for st in archive.standing {
            self.save(
                self.standing_transitions_dir()
                    .join(format!("{}.json", st.transition_record_id)),
                &st,
            )?;
        }
        for cs in archive.change_sets {
            self.save(
                self.change_sets_dir()
                    .join(format!("{}.json", cs.change_set_id.as_str())),
                &cs,
            )?;
        }
        for handoff in archive.handoffs {
            self.save(
                self.handoff_manifests_dir()
                    .join(format!("{}.json", handoff.handoff_manifest_id.as_str())),
                &handoff,
            )?;
        }
        for migration in archive.migrations {
            self.save(
                self.migrations_dir()
                    .join(format!("{}.json", migration.migration_id)),
                &migration,
            )?;
        }

        // 5. Global Verification at the end
        let violations = self.verify_regression_gate()?;
        if !violations.is_empty() {
            return Err(StoreError::Regression(violations.join("; ")));
        }

        Ok(())
    }
}
