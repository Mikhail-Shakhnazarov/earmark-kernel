use std::collections::{BTreeMap, BTreeSet, VecDeque};

use earmark_core::{
    AssignmentStatus, ChangeSet, ChangeSetDraft, ClassFilter,
    ConnectedContextManifest, HeaderValue, Kind, ObjectId, ObjectRef, Provenance, RelationFilter,
    RuntimeProvenance, Standing, StandingFilter, TransitionAssignment, TransitionAssignmentId,
    VersionRef,
};
use earmark_declarations::activate_system_definition;
use earmark_exec::{ExecutionEngine, ProviderRegistry, WorkflowRunOutcome, WorkflowRunRequest};
use earmark_governance::GovernanceService;
use earmark_index::{DerivedIndex, ObjectSummary, QueryFilter};
use earmark_connected_context::{CompiledContextService, WorkSurfaceManifest};
use earmark_store::{CanonicalStore, StoredObject, StoredPayload};
use serde_json::Value;
use std::time::Duration;
use thiserror::Error;

pub struct RuntimeToolSurface<'a, S: CanonicalStore> {
    pub store: &'a S,
    pub index: &'a DerivedIndex,
    pub provider_registry: &'a ProviderRegistry,
}

impl<'a, S: CanonicalStore> RuntimeToolSurface<'a, S> {
    pub fn deposit_prose(
        &self,
        class: &str,
        title: Option<String>,
        body: String,
    ) -> Result<VersionRef, RuntimeToolError> {
        let mut headers = BTreeMap::new();
        if let Some(title) = title {
            headers.insert("title".to_string(), HeaderValue::String(title));
        }
        let object = StoredObject::new(
            Kind::Object,
            Some(class.to_string()),
            Standing::default(),
            Provenance::direct_input("runtime"),
            headers,
            StoredPayload::from_markdown(body),
            vec![],
        );
        Ok(self.store.write_object(&object)?)
    }

    pub fn query(&self, filter: QueryFilter) -> Result<Vec<ObjectSummary>, RuntimeToolError> {
        self.index.rebuild_from_store(self.store)?;
        Ok(self.index.query_objects(&filter)?)
    }

    pub fn activate_system(
        &self,
        system_id: &str,
    ) -> Result<earmark_index::ActiveSystemRecord, RuntimeToolError> {
        self.index.rebuild_from_store(self.store)?;
        Ok(activate_system_definition(
            self.store, self.index, system_id,
        )?)
    }

    pub fn compile_work_surface(
        &self,
        compiled_context_ref: &VersionRef,
    ) -> Result<WorkSurfaceManifest, RuntimeToolError> {
        self.index.rebuild_from_store(self.store)?;
        Ok(CompiledContextService::compile(
            self.store,
            self.index,
            compiled_context_ref,
            None,
        )?)
    }

    pub fn propose_review(
        &self,
        target: ObjectRef,
        accepted: bool,
        rationale: Option<String>,
    ) -> Result<ObjectRef, RuntimeToolError> {
        let review =
            GovernanceService::create_review_object(self.store, target, accepted, rationale)?;
        Ok(review.object_ref())
    }

    pub fn run_workflow(
        &self,
        request: WorkflowRunRequest,
    ) -> Result<WorkflowRunOutcome, RuntimeToolError> {
        self.index.rebuild_from_store(self.store)?;
        let engine = ExecutionEngine {
            store: self.store,
            index: self.index,
            provider_registry: self.provider_registry,
        };
        Ok(engine.run_workflow(request)?)
    }

    pub fn deposit_object(
        &self,
        class: String,
        kind: Option<String>,
        title: Option<String>,
        payload: Value,
        provenance: RuntimeProvenance,
    ) -> Result<ObjectRef, RuntimeToolError> {
        let mut headers = BTreeMap::new();
        if let Some(title) = title {
            headers.insert("title".to_string(), HeaderValue::String(title));
        }
        let k = kind
            .and_then(|k| k.parse::<Kind>().ok())
            .unwrap_or(Kind::Object);

        let stored_payload = match k {
            Kind::Instruction | Kind::Object if payload.is_string() => {
                StoredPayload::from_markdown(
                    payload
                        .as_str()
                        .ok_or_else(|| {
                            RuntimeToolError::InvalidPayloadShape(
                                "Expected string payload for markdown".to_string(),
                            )
                        })?
                        .to_string(),
                )
            }
            Kind::Workflow
            | Kind::Policy
            | Kind::CompiledContextTemplate
            | Kind::ProviderProfile
            | Kind::SystemDefinition => {
                if payload.is_string() {
                    StoredPayload::from_yaml(
                        payload
                            .as_str()
                            .ok_or_else(|| {
                                RuntimeToolError::InvalidPayloadShape(
                                    "Expected string payload for yaml".to_string(),
                                )
                            })?
                            .to_string(),
                    )
                } else {
                    StoredPayload::from_json_bytes(serde_json::to_vec_pretty(&payload)?)
                }
            }
            _ => StoredPayload::from_json_bytes(serde_json::to_vec_pretty(&payload)?),
        };

        let object = StoredObject::new(
            k.clone(),
            Some(class.clone()),
            Standing::default(),
            Provenance {
                actor: provenance.actor,
                source_type: provenance.source_type,
                source_ref: None,
                lineage: vec![],
                import_path: None,
                captured_at: chrono::Utc::now(),
            },
            headers,
            stored_payload,
            vec![],
        );
        let version_ref = self.store.write_object(&object)?;
        Ok(ObjectRef::new(
            version_ref.id,
            version_ref.version_id,
            k,
            Some(class),
        ))
    }

    pub fn create_relation(
        &self,
        source_id: ObjectId,
        target_id: ObjectId,
        relation_type: String,
        metadata: Value,
        provenance: RuntimeProvenance,
    ) -> Result<ObjectRef, RuntimeToolError> {
        let source_head = self
            .store
            .read_head(&source_id)?
            .ok_or_else(|| RuntimeToolError::MissingObject(source_id.0.clone()))?;
        let target_head = self
            .store
            .read_head(&target_id)?
            .ok_or_else(|| RuntimeToolError::MissingObject(target_id.0.clone()))?;

        let mut qualifiers = BTreeMap::new();
        if let Value::Object(map) = metadata {
            for (k, v) in map {
                let scalar = serde_json::from_value(v)?;
                qualifiers.insert(k, scalar);
            }
        }

        let relation = earmark_core::RelationPayload {
            source: source_head.object_ref(),
            target: target_head.object_ref(),
            relation_type,
            qualifiers,
            scope: None,
        };

        let stored = StoredObject::new(
            Kind::Relation,
            None,
            Standing::default(),
            Provenance {
                actor: provenance.actor,
                source_type: provenance.source_type,
                source_ref: None,
                lineage: vec![],
                import_path: None,
                captured_at: chrono::Utc::now(),
            },
            BTreeMap::new(),
            StoredPayload::from_json_bytes(serde_json::to_vec_pretty(&relation)?),
            vec![],
        );
        let version_ref = self.store.write_object(&stored)?;
        Ok(ObjectRef::new(
            version_ref.id,
            version_ref.version_id,
            Kind::Relation,
            None,
        ))
    }

    pub fn assign_transition(
        &self,
        run_id: String,
        transition_id: String,
        agent_id: String,
        input_object_ids: Vec<ObjectId>,
        lease: Option<Duration>,
    ) -> Result<TransitionAssignment, RuntimeToolError> {
        self.reject_duplicate_active_assignment(&run_id, &transition_id)?;

        let assignment_id =
            TransitionAssignmentId(format!("assignment_{}", earmark_core::ObjectId::new().0));
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
                HeaderValue::String(format!("Assignment {}", assignment_id.0)),
            )]),
            StoredPayload::from_json_bytes(serde_json::to_vec_pretty(&assignment)?),
            vec![],
        );
        self.store.write_object(&stored)?;
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
                assignment_id.0, assignment.status
            )));
        }

        let change_set_id = earmark_core::ChangeSetId(format!(
            "change_set_{}",
            earmark_core::ObjectId::new().0
        ));
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
                HeaderValue::String(format!("Change Set {}", change_set_id.0)),
            )]),
            StoredPayload::from_json_bytes(serde_json::to_vec_pretty(&change_set)?),
            vec![],
        );
        self.store.write_object(&stored_change_set)?;

        // Task 4.C: Link standing requests via Relations
        for request in &standing_requests {
            // Validate structural legality
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
                        request.target_object_id.0
                    )),
                )]),
                StoredPayload::from_json_bytes(serde_json::to_vec_pretty(request)?),
                vec![],
            );
            let request_ref = self.store.write_object(&stored_request)?;

            let rel_payload = earmark_core::RelationPayload {
                source: ObjectRef::new(
                    earmark_core::ObjectId(change_set_id.0.clone()),
                    stored_change_set.envelope.version_id.clone(),
                    earmark_core::Kind::ChangeSet,
                    None,
                ),
                target: ObjectRef::new(
                    request_ref.id,
                    request_ref.version_id,
                    Kind::Object,
                    Some("standing_transition_request".to_string()),
                ),
                relation_type: "requests_standing".to_string(),
                qualifiers: BTreeMap::new(),
                scope: None,
            };
            let stored_rel = StoredObject::new(
                Kind::Relation,
                None,
                Standing::default(),
                Provenance::direct_input(agent_id.clone()),
                BTreeMap::new(),
                StoredPayload::from_json_bytes(serde_json::to_vec_pretty(&rel_payload)?),
                vec![],
            );
            self.store.write_object(&stored_rel)?;
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
                assignment_id.0, assignment.status
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
                assignment_id.0, assignment.status
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
                assignment_id.0, assignment.status
            )));
        }
        assignment.status = earmark_core::AssignmentStatus::Superseded;
        assignment.updated_at = chrono::Utc::now();
        // Option: add successor_assignment_id to blocked_reason or a new field if we want to trace it

        let update = StoredObject::with_parent(
            &old_obj,
            Standing::default(),
            old_obj.envelope.headers.clone(),
            StoredPayload::from_json_bytes(serde_json::to_vec_pretty(&assignment)?),
        );
        self.store.write_object(&update)?;
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
                assignment_id.0, old_assignment.status
            )));
        }

        // Verify required canonical heads still exist if no handoff manifest
        if let Some(handoff_id) = &old_assignment.handoff_manifest_id {
            // Verify handoff manifest exists
            let _ = self.load_handoff(handoff_id.clone())?;
        } else {
            for object_id in &old_assignment.input_object_ids {
                if self.store.read_head(object_id)?.is_none() {
                    return Err(RuntimeToolError::Conflict(format!(
                        "cannot resume assignment {}; required input object {} no longer exists as a head",
                        assignment_id.0, object_id.0
                    )));
                }
            }
        }

        self.reject_duplicate_active_assignment(&old_assignment.run_id, &old_assignment.transition_id)?;

        let new_assignment_id =
            TransitionAssignmentId(format!("assignment_{}", earmark_core::ObjectId::new().0));
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
                    new_assignment_id.0, assignment_id.0
                )),
            )]),
            StoredPayload::from_json_bytes(serde_json::to_vec_pretty(&new_assignment)?),
            vec![],
        );
        self.store.write_object(&stored)?;
        Ok(new_assignment)
    }

    fn reject_duplicate_active_assignment(
        &self,
        run_id: &str,
        transition_id: &str,
    ) -> Result<(), RuntimeToolError> {
        // Detect duplicate active assignments by scanning store.
        // Terminal statuses (Completed, Released, Superseded, Expired) do not block new assignments.
        // Blocked assignments do not block new assignments (they must be resumed or a new fresh assignment created).
        for obj in self.store.scan_objects()? {
            if obj.envelope.kind == Kind::TransitionAssignment {
                if let Ok(assignment) =
                    serde_json::from_slice::<TransitionAssignment>(&obj.payload.bytes)
                {
                    if assignment.run_id == run_id && assignment.transition_id == transition_id {
                        // IMPORTANT: Only consider the current head version of the assignment
                        if let Some(head_ref) = self.store.read_head_ref(&obj.envelope.id)? {
                            if head_ref.version_id != obj.envelope.version_id {
                                continue;
                            }
                        }

                        if assignment.status == earmark_core::AssignmentStatus::Assigned {
                            // Check lease expiration if applicable
                            let now = chrono::Utc::now();
                            let is_active = match assignment.expires_at {
                                Some(expires) => expires > now,
                                None => true,
                            };
                            if is_active {
                                return Err(RuntimeToolError::Conflict(format!(
                                    "transition {} in run {} is already assignmented by {}",
                                    transition_id, run_id, assignment.assigned_to
                                )));
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn find_head_assignment(
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
        Err(RuntimeToolError::MissingObject(assignment_id.0.clone()))
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
        Err(RuntimeToolError::MissingObject(handoff_manifest_id.0))
    }

    pub fn compile_connected_context(
        &self,
        root_object_ids: Vec<ObjectId>,
        max_depth: usize,
        relation_filter: Option<RelationFilter>,
        class_filter: Option<ClassFilter>,
        standing_filter: Option<StandingFilter>,
    ) -> Result<ConnectedContextManifest, RuntimeToolError> {
        self.index.rebuild_from_store(self.store)?;
        let heads = current_head_objects(self.store)?;

        let mut queue = VecDeque::new();
        let mut seen_objects = BTreeSet::new();
        let mut seen_relations = BTreeSet::new();
        let mut object_refs = Vec::new();
        let mut relation_refs = Vec::new();

        for root_id in &root_object_ids {
            let stored = heads
                .get(root_id)
                .ok_or_else(|| RuntimeToolError::MissingObject(root_id.0.clone()))?;
            seen_objects.insert(root_id.clone());
            object_refs.push(stored.object_ref());
            queue.push_back((root_id.clone(), 0usize));
        }

        while let Some((current_id, depth)) = queue.pop_front() {
            if depth >= max_depth {
                continue;
            }

            for relation in heads
                .values()
                .filter(|obj| obj.envelope.kind == Kind::Relation)
            {
                let payload: earmark_core::RelationPayload =
                    serde_json::from_slice(&relation.payload.bytes)?;
                if !relation_type_allowed(&payload.relation_type, relation_filter.as_ref()) {
                    continue;
                }

                let neighbor_id = if payload.source.id == current_id {
                    Some(payload.target.id.clone())
                } else if payload.target.id == current_id {
                    Some(payload.source.id.clone())
                } else {
                    None
                };

                let Some(neighbor_id) = neighbor_id else {
                    continue;
                };
                let Some(neighbor) = heads.get(&neighbor_id) else {
                    continue;
                };
                if !object_allowed(neighbor, class_filter.as_ref(), standing_filter.as_ref()) {
                    continue;
                }

                if seen_relations.insert(relation.envelope.id.clone()) {
                    relation_refs.push(relation.object_ref());
                }
                if seen_objects.insert(neighbor_id.clone()) {
                    object_refs.push(neighbor.object_ref());
                    queue.push_back((neighbor_id, depth + 1));
                }
            }
        }

        Ok(ConnectedContextManifest {
            root_object_ids,
            object_refs,
            relation_refs,
            max_depth,
            generated_at: chrono::Utc::now(),
        })
    }
}

fn current_head_objects<S: CanonicalStore>(
    store: &S,
) -> Result<BTreeMap<ObjectId, StoredObject>, RuntimeToolError> {
    let mut heads = BTreeMap::new();
    for object in store.scan_objects()? {
        if let Some(head_ref) = store.read_head_ref(&object.envelope.id)? {
            if head_ref.version_id == object.envelope.version_id {
                heads.insert(object.envelope.id.clone(), object);
            }
        }
    }
    Ok(heads)
}

fn relation_type_allowed(relation_type: &str, filter: Option<&RelationFilter>) -> bool {
    filter
        .map(|filter| {
            filter.allowed_types.is_empty()
                || filter
                    .allowed_types
                    .iter()
                    .any(|allowed| allowed == relation_type)
        })
        .unwrap_or(true)
}

fn object_allowed(
    object: &StoredObject,
    class_filter: Option<&ClassFilter>,
    standing_filter: Option<&StandingFilter>,
) -> bool {
    let class_ok = class_filter
        .map(|filter| {
            filter.allowed_classes.is_empty()
                || object
                    .envelope
                    .class
                    .as_ref()
                    .map(|class| {
                        filter
                            .allowed_classes
                            .iter()
                            .any(|allowed| allowed == class)
                    })
                    .unwrap_or(false)
        })
        .unwrap_or(true);
    let standing_ok = standing_filter
        .map(|filter| {
            filter.allowed_epistemic.is_empty()
                || filter
                    .allowed_epistemic
                    .iter()
                    .any(|allowed| allowed == &object.envelope.standing.epistemic)
        })
        .unwrap_or(true);
    class_ok && standing_ok
}

#[derive(Debug, Error)]
pub enum RuntimeToolError {
    #[error("store error: {0}")]
    Store(#[from] earmark_store::StoreError),
    #[error("index error: {0}")]
    Index(#[from] earmark_index::IndexError),
    #[error("derive error: {0}")]
    Derive(#[from] earmark_declarations::DeriveError),
    #[error("project error: {0}")]
    Project(#[from] earmark_connected_context::ProjectError),
    #[error("governance error: {0}")]
    Governance(#[from] earmark_governance::GovernanceError),
    #[error("execution error: {0}")]
    Exec(#[from] earmark_exec::ExecError),
    #[error("missing object: {0}")]
    MissingObject(String),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("invalid payload shape: {0}")]
    InvalidPayloadShape(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use earmark_exec::ProviderRegistry;
    use earmark_index::DerivedIndex;
    use earmark_store::GitCanonicalStore;
    use serde_json::json;
    use tempfile::tempdir;

    fn setup_surface(dir: &std::path::Path) -> (GitCanonicalStore, DerivedIndex, ProviderRegistry) {
        let store = GitCanonicalStore::new(dir);
        store.init_layout().unwrap();
        let index = DerivedIndex::open(dir).unwrap();
        let registry = ProviderRegistry::default();
        (store, index, registry)
    }

    #[test]
    fn test_duplicate_active_assignment_rejection() {
        let dir = tempdir().unwrap();
        let (store, index, registry) = setup_surface(dir.path());
        let surface = RuntimeToolSurface {
            store: &store,
            index: &index,
            provider_registry: &registry,
        };

        let _assignment1 = surface
            .assign_transition(
                "run1".to_string(),
                "trans1".to_string(),
                "agentA".to_string(),
                vec![],
                None,
            )
            .unwrap();

        let err = surface
            .assign_transition(
                "run1".to_string(),
                "trans1".to_string(),
                "agentB".to_string(),
                vec![],
                None,
            )
            .unwrap_err();
        assert!(matches!(err, RuntimeToolError::Conflict(_)));
    }

    #[test]
    fn test_assignment_completion_creating_a_change_set() {
        let dir = tempdir().unwrap();
        let (store, index, registry) = setup_surface(dir.path());
        let surface = RuntimeToolSurface {
            store: &store,
            index: &index,
            provider_registry: &registry,
        };

        let assignment = surface
            .assign_transition(
                "run1".to_string(),
                "trans1".to_string(),
                "agentA".to_string(),
                vec![],
                None,
            )
            .unwrap();

        let draft = ChangeSetDraft {
            created_objects: vec![],
            created_relations: vec![],
            updated_objects: vec![],
            governance_events: vec![],
            standing_requests: vec![],
            blocked_operations: vec![],
            unresolved_ambiguities: vec![],
            rejected_candidates: vec![],
        };

        let change_set = surface
            .complete_transition_assignment(assignment.id.clone(), draft, "agentA".to_string())
            .unwrap();
        assert_eq!(change_set.assignment_id, Some(assignment.id));
        assert_eq!(change_set.run_id, "run1");
    }

    #[test]
    fn test_loading_missing_handoff_manifest() {
        let dir = tempdir().unwrap();
        let (store, index, registry) = setup_surface(dir.path());
        let surface = RuntimeToolSurface {
            store: &store,
            index: &index,
            provider_registry: &registry,
        };

        let err = surface
            .load_handoff(earmark_core::HandoffManifestId("missing_1".to_string()))
            .unwrap_err();
        assert!(matches!(err, RuntimeToolError::MissingObject(_)));
    }

    #[test]
    fn test_relation_qualifier_json_conversion_failure() {
        let dir = tempdir().unwrap();
        let (store, index, registry) = setup_surface(dir.path());
        let surface = RuntimeToolSurface {
            store: &store,
            index: &index,
            provider_registry: &registry,
        };

        // Deposit two objects to be able to relate them
        let obj1 = surface
            .deposit_object(
                "test".to_string(),
                None,
                None,
                json!("body"),
                RuntimeProvenance {
                    actor: "test".to_string(),
                    source_type: "test".to_string(),
                },
            )
            .unwrap();
        let obj2 = surface
            .deposit_object(
                "test".to_string(),
                None,
                None,
                json!("body"),
                RuntimeProvenance {
                    actor: "test".to_string(),
                    source_type: "test".to_string(),
                },
            )
            .unwrap();

        // Pass invalid JSON value (array instead of scalar for qualifier)
        let metadata = json!({
            "bad_qualifier": ["nested", "array"]
        });

        let err = surface
            .create_relation(
                obj1.id.clone(),
                obj2.id.clone(),
                "rel".to_string(),
                metadata,
                RuntimeProvenance {
                    actor: "test".to_string(),
                    source_type: "test".to_string(),
                },
            )
            .unwrap_err();
        assert!(matches!(err, RuntimeToolError::Json(_)));
    }

    #[test]
    fn test_compile_connected_context_honors_depth_and_filters() {
        let dir = tempdir().unwrap();
        let (store, index, registry) = setup_surface(dir.path());
        let surface = RuntimeToolSurface {
            store: &store,
            index: &index,
            provider_registry: &registry,
        };

        let root = surface
            .deposit_object(
                "root".to_string(),
                None,
                Some("Root".to_string()),
                json!("root body"),
                RuntimeProvenance {
                    actor: "test".to_string(),
                    source_type: "test".to_string(),
                },
            )
            .unwrap();
        let mid = surface
            .deposit_object(
                "mid".to_string(),
                None,
                Some("Mid".to_string()),
                json!("mid body"),
                RuntimeProvenance {
                    actor: "test".to_string(),
                    source_type: "test".to_string(),
                },
            )
            .unwrap();
        let far = surface
            .deposit_object(
                "far".to_string(),
                None,
                Some("Far".to_string()),
                json!("far body"),
                RuntimeProvenance {
                    actor: "test".to_string(),
                    source_type: "test".to_string(),
                },
            )
            .unwrap();

        let rel1 = surface
            .create_relation(
                root.id.clone(),
                mid.id.clone(),
                "supports".to_string(),
                json!({}),
                RuntimeProvenance {
                    actor: "test".to_string(),
                    source_type: "test".to_string(),
                },
            )
            .unwrap();
        surface
            .create_relation(
                mid.id.clone(),
                far.id.clone(),
                "blocks".to_string(),
                json!({}),
                RuntimeProvenance {
                    actor: "test".to_string(),
                    source_type: "test".to_string(),
                },
            )
            .unwrap();

        let manifest = surface
            .compile_connected_context(
                vec![root.id.clone()],
                1,
                Some(RelationFilter {
                    allowed_types: vec!["supports".to_string()],
                }),
                Some(ClassFilter {
                    allowed_classes: vec!["mid".to_string()],
                }),
                None,
            )
            .unwrap();

        assert_eq!(manifest.root_object_ids, vec![root.id.clone()]);
        assert_eq!(manifest.object_refs.len(), 2);
        assert!(manifest
            .object_refs
            .iter()
            .any(|object| object.id == root.id));
        assert!(manifest
            .object_refs
            .iter()
            .any(|object| object.id == mid.id));
        assert!(!manifest
            .object_refs
            .iter()
            .any(|object| object.id == far.id));
        assert_eq!(manifest.relation_refs.len(), 1);
        assert_eq!(manifest.relation_refs[0].id, rel1.id);
    }

    #[test]
    fn test_compile_connected_context_respects_standing_filters() {
        let dir = tempdir().unwrap();
        let (store, index, registry) = setup_surface(dir.path());
        let surface = RuntimeToolSurface {
            store: &store,
            index: &index,
            provider_registry: &registry,
        };

        let root = surface
            .deposit_object(
                "root".to_string(),
                None,
                None,
                json!("root"),
                RuntimeProvenance {
                    actor: "test".to_string(),
                    source_type: "test".to_string(),
                },
            )
            .unwrap();

        let supported = StoredObject::new(
            Kind::Object,
            Some("neighbor".to_string()),
            Standing {
                epistemic: earmark_core::EpistemicStanding::Supported,
                ..Standing::default()
            },
            Provenance::direct_input("test"),
            BTreeMap::new(),
            StoredPayload::from_markdown("supported"),
            vec![],
        );
        let supported_ref = store.write_object(&supported).unwrap();

        let contested = StoredObject::new(
            Kind::Object,
            Some("neighbor".to_string()),
            Standing {
                epistemic: earmark_core::EpistemicStanding::Contested,
                ..Standing::default()
            },
            Provenance::direct_input("test"),
            BTreeMap::new(),
            StoredPayload::from_markdown("contested"),
            vec![],
        );
        let contested_ref = store.write_object(&contested).unwrap();

        surface
            .create_relation(
                root.id.clone(),
                supported_ref.id.clone(),
                "supports".to_string(),
                json!({}),
                RuntimeProvenance {
                    actor: "test".to_string(),
                    source_type: "test".to_string(),
                },
            )
            .unwrap();
        surface
            .create_relation(
                root.id.clone(),
                contested_ref.id.clone(),
                "supports".to_string(),
                json!({}),
                RuntimeProvenance {
                    actor: "test".to_string(),
                    source_type: "test".to_string(),
                },
            )
            .unwrap();

        let manifest = surface
            .compile_connected_context(
                vec![root.id.clone()],
                1,
                Some(RelationFilter {
                    allowed_types: vec!["supports".to_string()],
                }),
                None,
                Some(StandingFilter {
                    allowed_epistemic: vec![earmark_core::EpistemicStanding::Supported],
                }),
            )
            .unwrap();

        assert!(manifest
            .object_refs
            .iter()
            .any(|object| object.id == root.id));
        assert!(manifest
            .object_refs
            .iter()
            .any(|object| object.id == supported_ref.id));
        assert!(!manifest
            .object_refs
            .iter()
            .any(|object| object.id == contested_ref.id));
    }

    #[test]
    fn test_assignment_lifecycle_release() {
        let dir = tempdir().unwrap();
        let (store, index, registry) = setup_surface(dir.path());
        let surface = RuntimeToolSurface {
            store: &store,
            index: &index,
            provider_registry: &registry,
        };

        let assignment = surface
            .assign_transition(
                "run1".to_string(),
                "trans1".to_string(),
                "agentA".to_string(),
                vec![],
                None,
            )
            .unwrap();
        surface.release_assignment(assignment.id.clone()).unwrap();

        let (_, updated) = surface.find_head_assignment(&assignment.id).unwrap();
        assert_eq!(updated.status, earmark_core::AssignmentStatus::Released);
    }

    #[test]
    fn test_assignment_lifecycle_expire() {
        let dir = tempdir().unwrap();
        let (store, index, registry) = setup_surface(dir.path());
        let surface = RuntimeToolSurface {
            store: &store,
            index: &index,
            provider_registry: &registry,
        };

        let assignment = surface
            .assign_transition(
                "run1".to_string(),
                "trans1".to_string(),
                "agentA".to_string(),
                vec![],
                None,
            )
            .unwrap();
        surface.expire_assignment(assignment.id.clone()).unwrap();

        let (_, updated) = surface.find_head_assignment(&assignment.id).unwrap();
        assert_eq!(updated.status, earmark_core::AssignmentStatus::Expired);
    }

    #[test]
    fn test_assignment_lifecycle_supersede() {
        let dir = tempdir().unwrap();
        let (store, index, registry) = setup_surface(dir.path());
        let surface = RuntimeToolSurface {
            store: &store,
            index: &index,
            provider_registry: &registry,
        };

        let assignment = surface
            .assign_transition(
                "run1".to_string(),
                "trans1".to_string(),
                "agentA".to_string(),
                vec![],
                None,
            )
            .unwrap();
        let successor_id = TransitionAssignmentId("successor".to_string());
        surface
            .supersede_assignment(assignment.id.clone(), successor_id)
            .unwrap();

        let (_, updated) = surface.find_head_assignment(&assignment.id).unwrap();
        assert_eq!(updated.status, earmark_core::AssignmentStatus::Superseded);
    }

    #[test]
    fn test_assignment_lifecycle_resume() {
        let dir = tempdir().unwrap();
        let (store, index, registry) = setup_surface(dir.path());
        let surface = RuntimeToolSurface {
            store: &store,
            index: &index,
            provider_registry: &registry,
        };

        // 1. Resume from Expired
        let assignment1 = surface
            .assign_transition(
                "run1".to_string(),
                "trans1".to_string(),
                "agentA".to_string(),
                vec![],
                None,
            )
            .unwrap();
        surface.expire_assignment(assignment1.id.clone()).unwrap();

        let resumed = surface
            .resume_assignment(assignment1.id.clone(), "agentB".to_string(), None)
            .unwrap();
        assert_eq!(resumed.status, earmark_core::AssignmentStatus::Assigned);
        assert_eq!(resumed.assigned_to, "agentB");
        assert_eq!(resumed.run_id, "run1");
        assert_eq!(resumed.transition_id, "trans1");

        // 2. Resume from Blocked
        // Manually move to blocked since engine isn't here
        let (old_obj, mut assignment2) = surface.find_head_assignment(&resumed.id).unwrap();
        assignment2.status = earmark_core::AssignmentStatus::Blocked;
        let update = StoredObject::with_parent(
            &old_obj,
            Standing::default(),
            old_obj.envelope.headers.clone(),
            StoredPayload::from_json_bytes(serde_json::to_vec_pretty(&assignment2).unwrap()),
        );
        store.write_object(&update).unwrap();

        let resumed2 = surface
            .resume_assignment(resumed.id.clone(), "agentC".to_string(), None)
            .unwrap();
        assert_eq!(resumed2.status, earmark_core::AssignmentStatus::Assigned);
        assert_eq!(resumed2.assigned_to, "agentC");
    }

    #[test]
    fn test_resume_fails_if_active_duplicate_exists() {
        let dir = tempdir().unwrap();
        let (store, index, registry) = setup_surface(dir.path());
        let surface = RuntimeToolSurface {
            store: &store,
            index: &index,
            provider_registry: &registry,
        };

        // Create expired assignment
        let assignment1 = surface
            .assign_transition(
                "run1".to_string(),
                "trans1".to_string(),
                "agentA".to_string(),
                vec![],
                None,
            )
            .unwrap();
        surface.expire_assignment(assignment1.id.clone()).unwrap();

        // Create parallel active assignment (same run/trans) - this shouldn't be blocked by expired
        let _assignment2 = surface
            .assign_transition(
                "run1".to_string(),
                "trans1".to_string(),
                "agentB".to_string(),
                vec![],
                None,
            )
            .unwrap();

        // Now try to resume assignment1 - should fail because assignment2 is active
        let err = surface
            .resume_assignment(assignment1.id.clone(), "agentC".to_string(), None)
            .unwrap_err();
        assert!(matches!(err, RuntimeToolError::Conflict(_)));
    }
}
