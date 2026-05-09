use crate::modules::error::RuntimeToolError;
use crate::modules::surface::RuntimeToolSurface;
use earmark_core::{
    AssignmentStatus, ChangeSet, ChangeSetDraft, HeaderValue, Kind, ObjectId, Provenance,
    RelationCreationMode, Standing, TransitionAssignment, TransitionAssignmentId,
    REL_TYPE_REQUESTS_STANDING,
};
use earmark_exec::{ExecutionEngine, WorkflowRunOutcome, WorkflowRunRequest};
use earmark_store::{CanonicalStore, StoredObject, StoredPayload};
use std::collections::BTreeMap;
use std::time::Duration;

impl<'a, S: CanonicalStore> RuntimeToolSurface<'a, S> {
    pub fn run_workflow(
        &self,
        request: WorkflowRunRequest,
    ) -> Result<WorkflowRunOutcome, RuntimeToolError> {
        let engine = ExecutionEngine {
            store: self.store,
            index: self.index,
            provider_service: self.provider_service,
        };
        Ok(engine.run_workflow(request)?)
    }

    pub fn assign_transition(
        &self,
        run_id: String,
        transition_id: String,
        agent_id: String,
        input_object_ids: Vec<ObjectId>,
        lease: Option<Duration>,
    ) -> Result<TransitionAssignment, RuntimeToolError> {
        let assignment_id = TransitionAssignmentId::new();
        self.index
            .claim_active_assignment(&run_id, &transition_id, assignment_id.as_str())
            .map_err(|e| RuntimeToolError::Conflict(e.to_string()))?;
        let now = chrono::Utc::now();
        let expires_at = match lease {
            Some(d) => Some(
                now + chrono::Duration::from_std(d)
                    .map_err(|e| RuntimeToolError::Conflict(format!("Invalid duration: {}", e)))?,
            ),
            None => None,
        };

        let assignment = TransitionAssignment {
            id: assignment_id.clone(),
            run_id,
            transition_id,
            assigned_to: agent_id.clone(),
            status: AssignmentStatus::Assigned,
            input_object_ids,
            handoff_manifest_id: None,
            event_ids: vec![],
            blocked_reason: None,
            completion_change_set_id: None,
            assigned_at: now,
            updated_at: now,
            expires_at,
            completed_at: None,
        };

        let stored = StoredObject::new(
            Kind::TransitionAssignment,
            Some("transition_assignment".to_string()),
            Standing::default(),
            Provenance::direct_input(agent_id),
            BTreeMap::from([(
                "title".to_string(),
                HeaderValue::String(format!("Assignment {}", assignment_id.as_str())),
            )]),
            StoredPayload::from_json_bytes(serde_json::to_vec_pretty(&assignment)?),
            vec![],
        );
        if let Err(err) = self.store.write_object(&stored) {
            let _ = self.index.release_active_assignment(
                &assignment.run_id,
                &assignment.transition_id,
                assignment_id.as_str(),
            );
            return Err(err.into());
        }
        self.index
            .upsert_head_object_from_store(self.store, &stored.envelope.id)?;
        Ok(assignment)
    }

    pub fn complete_transition_assignment(
        &self,
        assignment_id: TransitionAssignmentId,
        draft: ChangeSetDraft,
        agent_id: String,
    ) -> Result<ChangeSet, RuntimeToolError> {
        let (old_obj, mut assignment) = self.find_head_assignment(&assignment_id)?;

        if assignment.status != earmark_core::AssignmentStatus::Assigned {
            return Err(RuntimeToolError::Conflict(format!(
                "assignment {} is not in active state (status: {:?})",
                assignment_id.as_str(),
                assignment.status
            )));
        }

        let change_set_id = earmark_core::ChangeSetId::new();
        let now = chrono::Utc::now();
        let standing_requests = draft.standing_requests.clone();
        let change_set = ChangeSet {
            id: change_set_id.clone(),
            run_id: assignment.run_id.clone(),
            transition_id: assignment.transition_id.clone(),
            assignment_id: Some(assignment_id.clone()),
            agent_id: Some(agent_id.clone()),
            input_object_ids: assignment.input_object_ids.clone(),
            created_object_ids: draft.created_objects,
            created_relation_ids: draft.created_relations,
            updated_object_ids: draft.updated_objects,
            governance_event_ids: draft.governance_events,
            blocked_operations: draft.blocked_operations,
            unresolved_ambiguities: draft.unresolved_ambiguities,
            rejected_candidates: draft.rejected_candidates,
            validation_results: vec![],
            work_packet_id: None,
            handoff_manifest_id: None,
            created_at: now,
        };

        let stored_change_set = StoredObject::new(
            Kind::ChangeSet,
            Some("change_set".to_string()),
            Standing::default(),
            Provenance::direct_input(agent_id.clone()),
            BTreeMap::from([(
                "title".to_string(),
                HeaderValue::String(format!("Change Set {}", change_set_id.as_str())),
            )]),
            StoredPayload::from_json_bytes(serde_json::to_vec_pretty(&change_set)?),
            vec![],
        );
        self.store.write_object(&stored_change_set)?;
        self.index
            .upsert_head_object_from_store(self.store, &stored_change_set.envelope.id)?;

        for request in &standing_requests {
            if earmark_core::validate_standing_request(request).is_err() {
                continue;
            }

            let stored_request = StoredObject::new(
                Kind::Object,
                Some("standing_transition_request".to_string()),
                Standing::default(),
                Provenance::direct_input(agent_id.clone()),
                BTreeMap::from([(
                    "title".to_string(),
                    HeaderValue::String(format!(
                        "Standing Request for {}",
                        request.target_object_id.as_str()
                    )),
                )]),
                StoredPayload::from_json_bytes(serde_json::to_vec_pretty(request)?),
                vec![],
            );
            let request_ref = self.store.write_object(&stored_request)?;
            self.index
                .upsert_head_object_from_store(self.store, &stored_request.envelope.id)?;

            let rel_payload = earmark_core::RelationPayload {
                source: earmark_core::ObjectRef::new(
                    earmark_core::ObjectId::parse(change_set_id.as_str().to_string())?,
                    stored_change_set.envelope.version_id.clone(),
                    earmark_core::Kind::ChangeSet,
                    None,
                ),
                target: earmark_core::ObjectRef::new(
                    request_ref.id,
                    request_ref.version_id,
                    Kind::Object,
                    Some("standing_transition_request".to_string()),
                ),
                relation_type: REL_TYPE_REQUESTS_STANDING.to_string(),
                qualifiers: BTreeMap::new(),
                scope: None,
            };
            earmark_exec::persist_relation_canonical(
                self.store,
                self.index,
                rel_payload,
                Provenance::direct_input(agent_id.clone()),
                RelationCreationMode::PrivilegedSystem,
                None,
            )?;
        }
        assignment.status = earmark_core::AssignmentStatus::Completed;
        assignment.completion_change_set_id = Some(change_set_id);
        assignment.completed_at = Some(now);
        assignment.updated_at = now;

        let stored_assignment_update = StoredObject::with_parent(
            &old_obj,
            Standing::default(),
            old_obj.envelope.headers.clone(),
            StoredPayload::from_json_bytes(serde_json::to_vec_pretty(&assignment)?),
        );
        self.store.write_object(&stored_assignment_update)?;
        self.index
            .upsert_head_object_from_store(self.store, &old_obj.envelope.id)?;
        self.index.release_active_assignment(
            &assignment.run_id,
            &assignment.transition_id,
            assignment.id.as_str(),
        )?;

        Ok(change_set)
    }

    pub fn release_assignment(
        &self,
        assignment_id: TransitionAssignmentId,
    ) -> Result<(), RuntimeToolError> {
        let (old_obj, mut assignment) = self.find_head_assignment(&assignment_id)?;
        if assignment.status != earmark_core::AssignmentStatus::Assigned {
            return Err(RuntimeToolError::Conflict(format!(
                "assignment {} is not in active state (status: {:?})",
                assignment_id.as_str(),
                assignment.status
            )));
        }
        assignment.status = earmark_core::AssignmentStatus::Released;
        assignment.updated_at = chrono::Utc::now();

        let update = StoredObject::with_parent(
            &old_obj,
            Standing::default(),
            old_obj.envelope.headers.clone(),
            StoredPayload::from_json_bytes(serde_json::to_vec_pretty(&assignment)?),
        );
        self.store.write_object(&update)?;
        self.index
            .upsert_head_object_from_store(self.store, &old_obj.envelope.id)?;
        self.index.release_active_assignment(
            &assignment.run_id,
            &assignment.transition_id,
            assignment.id.as_str(),
        )?;
        Ok(())
    }

    pub fn expire_assignment(
        &self,
        assignment_id: TransitionAssignmentId,
    ) -> Result<(), RuntimeToolError> {
        let (old_obj, mut assignment) = self.find_head_assignment(&assignment_id)?;
        if assignment.status != earmark_core::AssignmentStatus::Assigned {
            return Err(RuntimeToolError::Conflict(format!(
                "assignment {} is not in active state (status: {:?})",
                assignment_id.as_str(),
                assignment.status
            )));
        }
        assignment.status = earmark_core::AssignmentStatus::Expired;
        assignment.updated_at = chrono::Utc::now();

        let update = StoredObject::with_parent(
            &old_obj,
            Standing::default(),
            old_obj.envelope.headers.clone(),
            StoredPayload::from_json_bytes(serde_json::to_vec_pretty(&assignment)?),
        );
        self.store.write_object(&update)?;
        self.index
            .upsert_head_object_from_store(self.store, &old_obj.envelope.id)?;
        self.index.release_active_assignment(
            &assignment.run_id,
            &assignment.transition_id,
            assignment.id.as_str(),
        )?;
        Ok(())
    }

    pub fn supersede_assignment(
        &self,
        assignment_id: TransitionAssignmentId,
        _successor_assignment_id: TransitionAssignmentId,
    ) -> Result<(), RuntimeToolError> {
        let (old_obj, mut assignment) = self.find_head_assignment(&assignment_id)?;
        if assignment.status != earmark_core::AssignmentStatus::Assigned
            && assignment.status != earmark_core::AssignmentStatus::Blocked
        {
            return Err(RuntimeToolError::Conflict(format!(
                "assignment {} is not in a supersedable state (status: {:?})",
                assignment_id.as_str(),
                assignment.status
            )));
        }
        assignment.status = earmark_core::AssignmentStatus::Superseded;
        assignment.updated_at = chrono::Utc::now();

        let update = StoredObject::with_parent(
            &old_obj,
            Standing::default(),
            old_obj.envelope.headers.clone(),
            StoredPayload::from_json_bytes(serde_json::to_vec_pretty(&assignment)?),
        );
        self.store.write_object(&update)?;
        self.index
            .upsert_head_object_from_store(self.store, &old_obj.envelope.id)?;
        self.index.release_active_assignment(
            &assignment.run_id,
            &assignment.transition_id,
            assignment.id.as_str(),
        )?;
        Ok(())
    }

    pub fn resume_assignment(
        &self,
        assignment_id: TransitionAssignmentId,
        agent_id: String,
        lease: Option<Duration>,
    ) -> Result<TransitionAssignment, RuntimeToolError> {
        let (_, old_assignment) = self.find_head_assignment(&assignment_id)?;
        if old_assignment.status != earmark_core::AssignmentStatus::Blocked
            && old_assignment.status != earmark_core::AssignmentStatus::Expired
        {
            return Err(RuntimeToolError::Conflict(format!(
                "assignment {} is not in a resumable state (status: {:?})",
                assignment_id.as_str(),
                old_assignment.status
            )));
        }

        if let Some(handoff_id) = &old_assignment.handoff_manifest_id {
            let _ = self.load_handoff(handoff_id.clone())?;
        } else {
            for object_id in &old_assignment.input_object_ids {
                if self.store.read_head(object_id)?.is_none() {
                    return Err(RuntimeToolError::Conflict(format!(
                        "cannot resume assignment {}; required input object {} no longer exists as a head",
                        assignment_id.as_str(), object_id.as_str()
                    )));
                }
            }
        }

        let new_assignment_id = TransitionAssignmentId::new();
        let _ = self.index.release_active_assignment(
            &old_assignment.run_id,
            &old_assignment.transition_id,
            old_assignment.id.as_str(),
        );
        self.index
            .claim_active_assignment(
                &old_assignment.run_id,
                &old_assignment.transition_id,
                new_assignment_id.as_str(),
            )
            .map_err(|e| RuntimeToolError::Conflict(e.to_string()))?;
        let now = chrono::Utc::now();
        let expires_at = match lease {
            Some(d) => Some(
                now + chrono::Duration::from_std(d)
                    .map_err(|e| RuntimeToolError::Conflict(format!("Invalid duration: {}", e)))?,
            ),
            None => None,
        };

        let new_assignment = TransitionAssignment {
            id: new_assignment_id.clone(),
            run_id: old_assignment.run_id.clone(),
            transition_id: old_assignment.transition_id.clone(),
            assigned_to: agent_id.clone(),
            status: earmark_core::AssignmentStatus::Assigned,
            input_object_ids: old_assignment.input_object_ids.clone(),
            handoff_manifest_id: old_assignment.handoff_manifest_id.clone(),
            event_ids: vec![],
            blocked_reason: None,
            completion_change_set_id: None,
            assigned_at: now,
            updated_at: now,
            expires_at,
            completed_at: None,
        };

        let stored = StoredObject::new(
            Kind::TransitionAssignment,
            Some("transition_assignment".to_string()),
            Standing::default(),
            Provenance::direct_input(agent_id),
            BTreeMap::from([(
                "title".to_string(),
                HeaderValue::String(format!(
                    "Claim {} (Resumed from {})",
                    new_assignment_id.as_str(),
                    assignment_id.as_str()
                )),
            )]),
            StoredPayload::from_json_bytes(serde_json::to_vec_pretty(&new_assignment)?),
            vec![],
        );
        if let Err(err) = self.store.write_object(&stored) {
            let _ = self.index.release_active_assignment(
                &new_assignment.run_id,
                &new_assignment.transition_id,
                new_assignment_id.as_str(),
            );
            return Err(err.into());
        }
        self.index
            .upsert_head_object_from_store(self.store, &stored.envelope.id)?;
        Ok(new_assignment)
    }

    pub(crate) fn find_head_assignment(
        &self,
        assignment_id: &TransitionAssignmentId,
    ) -> Result<(StoredObject, TransitionAssignment), RuntimeToolError> {
        for obj in self.store.scan_objects()? {
            if obj.envelope.kind == Kind::TransitionAssignment {
                if let Ok(assignment) =
                    serde_json::from_slice::<TransitionAssignment>(&obj.payload.bytes)
                {
                    if &assignment.id == assignment_id {
                        if let Some(head_ref) = self.store.read_head_ref(&obj.envelope.id)? {
                            if head_ref.version_id == obj.envelope.version_id {
                                return Ok((obj, assignment));
                            }
                        }
                    }
                }
            }
        }
        Err(RuntimeToolError::MissingObject(
            assignment_id.as_str().to_string(),
        ))
    }

    pub fn load_handoff(
        &self,
        handoff_manifest_id: earmark_core::HandoffManifestId,
    ) -> Result<earmark_core::HandoffManifest, RuntimeToolError> {
        for obj in self.store.scan_objects()? {
            if obj.envelope.kind == Kind::HandoffManifest {
                if let Ok(manifest) =
                    serde_json::from_slice::<earmark_core::HandoffManifest>(&obj.payload.bytes)
                {
                    if manifest.id == handoff_manifest_id {
                        return Ok(manifest);
                    }
                }
            }
        }
        Err(RuntimeToolError::MissingObject(
            handoff_manifest_id.as_str().to_string(),
        ))
    }
}
