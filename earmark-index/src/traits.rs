/*
 * Copyright (c) 2026 Mikhail Shakhnazarov.
 * Dual-licensed under AGPL-3.0-or-later or commercial terms.
 */

use crate::errors::IndexError;
use earmark_core::{
    DispatchId, DispatchRecord, HandoffManifestId, HandoffManifestRecord, ObjectId, ObjectRecord,
    PacketId, PacketRecord, RelationId, RelationRecord, ReviewId, ReviewRecord, RunId, RunRecord,
    VersionId,
};

pub trait DerivedIndex {
    fn rebuild_from_store(
        &mut self,
        store: &dyn earmark_store::CanonicalStore,
    ) -> Result<(), IndexError>;

    fn get_object(&self, id: &ObjectId) -> Result<ObjectRecord, IndexError>;
    fn get_head_version_id(&self, id: &ObjectId) -> Result<VersionId, IndexError>;
    fn get_relation(&self, id: &RelationId) -> Result<RelationRecord, IndexError>;

    fn find_objects(&self, query: ObjectQuery) -> Result<Vec<ObjectRecord>, IndexError>;
    fn find_relations_by_source(
        &self,
        source_id: &ObjectId,
    ) -> Result<Vec<RelationRecord>, IndexError>;

    fn get_run(&self, id: &RunId) -> Result<RunRecord, IndexError>;
    fn get_dispatch(&self, id: &DispatchId) -> Result<DispatchRecord, IndexError>;
    fn get_packet(&self, id: &PacketId) -> Result<PacketRecord, IndexError>;
    fn get_handoff(&self, id: &HandoffManifestId) -> Result<HandoffManifestRecord, IndexError>;
    fn get_review(&self, id: &ReviewId) -> Result<ReviewRecord, IndexError>;

    fn list_active_claims(&self) -> Result<Vec<DispatchRecord>, IndexError>;
    fn list_expired_leases(&self) -> Result<Vec<DispatchRecord>, IndexError>;
}

#[derive(Debug, Clone, Default)]
pub struct ObjectQuery {
    pub class_id: Option<earmark_core::ClassId>,
}
