/*
 * Copyright (c) 2026 Mikhail Shakhnazarov. Dual-licensed under AGPL-3.0-or-later or commercial terms.
 * PROPRIETARY AND INTERNAL. ONLY LOCALLY COMMITTED.
 * v0.1_internal kernel.
 */

use crate::errors::StoreError;
use earmark_core::{
    ActorId, ChangeSetId, ChangeSetRecord, CheckResultId, CheckResultRecord, DispatchId,
    DispatchRecord, ExternalConnectionRecord, HandoffManifestId, HandoffManifestRecord, ObjectId,
    ObjectRecord, PacketId, PacketRecord, ProviderProfile, ProviderProfileId, ProviderRecord,
    RelationId, RelationRecord, ReviewId, ReviewRecord, RunId, RunRecord, StandingTransitionRecord,
    SystemDeclaration, SystemId, UndoRecord, VersionId, VersionRecord, WorkerProfile,
    WorkerProfileId, WorkflowDeclaration, WorkflowId,
};

pub trait CanonicalStore {
    // Workspace Management
    fn is_initialized(&self) -> bool;
    fn init(&self) -> Result<(), StoreError>;
    fn verify_consistency(&self) -> Result<Vec<String>, StoreError>;
    fn verify_maturity_gate(&self) -> Result<Vec<String>, StoreError>;
    fn verify_regression_gate(&self) -> Result<Vec<String>, StoreError>;

    // Object Operations
    fn deposit_object(
        &self,
        record: ObjectRecord,
        version: VersionRecord,
    ) -> Result<(), StoreError>;
    fn get_object(&self, id: &ObjectId) -> Result<ObjectRecord, StoreError>;
    fn get_version(
        &self,
        id: &ObjectId,
        version_id: &VersionId,
    ) -> Result<VersionRecord, StoreError>;
    fn update_version(&self, record: VersionRecord) -> Result<(), StoreError>;
    fn list_versions(&self, id: &ObjectId) -> Result<Vec<VersionId>, StoreError>;
    fn list_objects(&self) -> Result<Vec<ObjectId>, StoreError>;
    fn list_objects_by_class(
        &self,
        class_id: &earmark_core::ClassId,
    ) -> Result<Vec<ObjectId>, StoreError>;

    // Declaration Management
    fn register_class(&self, record: earmark_core::ClassDeclaration) -> Result<(), StoreError>;
    fn get_class(
        &self,
        id: &earmark_core::ClassId,
    ) -> Result<earmark_core::ClassDeclaration, StoreError>;
    fn list_classes(&self) -> Result<Vec<earmark_core::ClassId>, StoreError>;

    fn register_system(&self, record: SystemDeclaration) -> Result<(), StoreError>;
    fn get_system(&self, id: &SystemId) -> Result<SystemDeclaration, StoreError>;
    fn list_systems(&self) -> Result<Vec<SystemId>, StoreError>;

    fn register_workflow(&self, record: WorkflowDeclaration) -> Result<(), StoreError>;
    fn get_workflow(&self, id: &WorkflowId) -> Result<WorkflowDeclaration, StoreError>;
    fn list_workflows(&self) -> Result<Vec<WorkflowId>, StoreError>;

    fn register_packet_template(
        &self,
        record: earmark_core::PacketTemplateDeclaration,
    ) -> Result<(), StoreError>;
    fn get_packet_template(
        &self,
        id: &earmark_core::PacketTemplateId,
    ) -> Result<earmark_core::PacketTemplateDeclaration, StoreError>;
    fn list_packet_templates(&self) -> Result<Vec<earmark_core::PacketTemplateId>, StoreError>;

    fn register_runtime_protocol(&self, record: earmark_core::RuntimeProtocol) -> Result<(), StoreError>;
    fn get_runtime_protocol(
        &self,
        id: &earmark_core::RuntimeProtocolId,
    ) -> Result<earmark_core::RuntimeProtocol, StoreError>;
    fn list_runtime_protocols(&self) -> Result<Vec<earmark_core::RuntimeProtocolId>, StoreError>;

    fn register_selection_policy(&self, record: earmark_core::SelectionPolicy) -> Result<(), StoreError>;
    fn get_selection_policy(
        &self,
        id: &earmark_core::SelectionPolicyId,
    ) -> Result<earmark_core::SelectionPolicy, StoreError>;
    fn list_selection_policies(&self) -> Result<Vec<earmark_core::SelectionPolicyId>, StoreError>;

    // Relation Operations
    fn create_relation(&self, record: RelationRecord) -> Result<(), StoreError>;
    fn get_relation(&self, id: &RelationId) -> Result<RelationRecord, StoreError>;
    fn list_relations(&self, source_id: &ObjectId) -> Result<Vec<RelationRecord>, StoreError>;
    fn list_all_relations(&self) -> Result<Vec<RelationId>, StoreError>;

    // Runtime Records
    fn create_run(&self, record: RunRecord) -> Result<(), StoreError>;
    fn update_run(&self, record: RunRecord) -> Result<(), StoreError>;
    fn get_run(&self, id: &RunId) -> Result<RunRecord, StoreError>;
    fn list_runs(&self) -> Result<Vec<RunId>, StoreError>;

    fn create_packet(&self, record: PacketRecord) -> Result<(), StoreError>;
    fn get_packet(&self, id: &PacketId) -> Result<PacketRecord, StoreError>;
    fn list_packets(&self) -> Result<Vec<PacketId>, StoreError>;

    fn create_dispatch(&self, record: DispatchRecord) -> Result<(), StoreError>;
    fn update_dispatch(&self, record: DispatchRecord) -> Result<(), StoreError>;
    fn get_dispatch(&self, id: &DispatchId) -> Result<DispatchRecord, StoreError>;
    fn list_dispatches(&self) -> Result<Vec<DispatchId>, StoreError>;

    fn create_change_set(&self, record: ChangeSetRecord) -> Result<(), StoreError>;
    fn get_change_set(&self, id: &ChangeSetId) -> Result<ChangeSetRecord, StoreError>;

    fn create_check_result(&self, record: CheckResultRecord) -> Result<(), StoreError>;
    fn get_check_result(&self, id: &CheckResultId) -> Result<CheckResultRecord, StoreError>;

    fn create_handoff_manifest(&self, record: HandoffManifestRecord) -> Result<(), StoreError>;
    fn get_handoff_manifest(
        &self,
        id: &HandoffManifestId,
    ) -> Result<HandoffManifestRecord, StoreError>;

    // Governance
    fn create_review(&self, record: ReviewRecord) -> Result<(), StoreError>;
    fn get_review(&self, id: &ReviewId) -> Result<ReviewRecord, StoreError>;
    fn list_reviews(&self) -> Result<Vec<ReviewId>, StoreError>;
    fn record_standing_transition(
        &self,
        record: StandingTransitionRecord,
    ) -> Result<(), StoreError>;
    fn get_standing(
        &self,
        target: &earmark_core::StandingTargetRef,
    ) -> Result<Vec<StandingTransitionRecord>, StoreError>;

    // Provider / External
    fn register_provider_profile(&self, record: ProviderProfile) -> Result<(), StoreError>;
    fn get_provider_profile(&self, id: &ProviderProfileId) -> Result<ProviderProfile, StoreError>;
    fn list_provider_profiles(&self) -> Result<Vec<ProviderProfileId>, StoreError>;
    fn record_provider_call(&self, record: ProviderRecord) -> Result<(), StoreError>;
    fn create_external_connection(
        &self,
        record: ExternalConnectionRecord,
    ) -> Result<(), StoreError>;

    // Worker Profiles
    fn register_worker_profile(&self, record: WorkerProfile) -> Result<(), StoreError>;
    fn get_worker_profile(&self, id: &WorkerProfileId) -> Result<WorkerProfile, StoreError>;
    fn list_worker_profiles(&self) -> Result<Vec<WorkerProfileId>, StoreError>;

    // System Packs
    fn register_system_pack(
        &self,
        record: earmark_core::SystemPackManifest,
    ) -> Result<(), StoreError>;
    fn get_system_pack(
        &self,
        id: &earmark_core::SystemPackId,
    ) -> Result<earmark_core::SystemPackManifest, StoreError>;
    fn list_system_packs(&self) -> Result<Vec<earmark_core::SystemPackId>, StoreError>;
    fn record_pack_activation(
        &self,
        record: earmark_core::PackActivationRecord,
    ) -> Result<(), StoreError>;
    fn get_pack_activation_history(
        &self,
        id: &earmark_core::SystemPackId,
    ) -> Result<Vec<earmark_core::PackActivationRecord>, StoreError>;

    // Claim & Lease Operations
    fn claim_dispatch(
        &self,
        id: &DispatchId,
        claimer: ActorId,
        lease_duration_secs: i64,
    ) -> Result<DispatchRecord, StoreError>;

    fn release_dispatch_claim(
        &self,
        id: &DispatchId,
        claimer: &ActorId,
    ) -> Result<DispatchRecord, StoreError>;

    fn renew_lease(
        &self,
        id: &DispatchId,
        claimer: &ActorId,
        lease_duration_secs: i64,
    ) -> Result<DispatchRecord, StoreError>;

    fn reap_expired_leases(&self) -> Result<Vec<DispatchId>, StoreError>;

    // Maintenance & Ledger Closure
    fn record_undo(&self, record: UndoRecord) -> Result<(), StoreError>;
    fn get_undo_history(
        &self,
        target: &earmark_core::ObjectRef,
    ) -> Result<Vec<UndoRecord>, StoreError>;
    fn record_migration(&self, record: earmark_core::MigrationRecord) -> Result<(), StoreError>;
    fn get_migration_history(&self) -> Result<Vec<earmark_core::MigrationRecord>, StoreError>;

    fn import_archive(
        &self,
        archive: earmark_core::records::archive::WorkspaceArchive,
        overwrite: bool,
    ) -> Result<(), StoreError>;
}
