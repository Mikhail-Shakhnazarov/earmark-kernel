use crate::error::ExecError;
use crate::handoff::create_lineage_relations;
use crate::ir::TransformArtifacts;
use crate::persistence_helpers::write_object_and_index;
use crate::provider::{provider_metadata_synthetic_source, provider_response_is_synthetic};
use crate::relation::persist_relation_canonical;
use chrono::Utc;
use earmark_core::{
    ChangeSetDraft, ChangeSetId, ChangeSetValidationResult, HandoffManifestId, InstructionPayload,
    Kind, ObjectId, ObjectRef, Provenance, ProviderResponse, RelationCreationMode, RelationPayload,
    RunRecord, Standing, TransformationFailure, TransitionAssignment, REL_TYPE_REQUESTS_STANDING,
    REL_TYPE_RESULTED_IN_FAILURE,
};
use earmark_index::DerivedIndex;
use earmark_store::{CanonicalStore, StoredObject, StoredPayload};
use std::collections::BTreeMap;

pub(crate) struct ChangeSetPersistence<'a> {
    pub(crate) record: &'a mut RunRecord,
    pub(crate) change_set_id: ChangeSetId,
    pub(crate) assignment: &'a TransitionAssignment,
    pub(crate) transition_id: &'a str,
    pub(crate) draft: &'a ChangeSetDraft,
    pub(crate) validation_results: Vec<ChangeSetValidationResult>,
    pub(crate) handoff_manifest_id: Option<HandoffManifestId>,
    pub(crate) index: &'a DerivedIndex,
}

pub(crate) fn persist_change_set<S: CanonicalStore>(
    store: &S,
    persistence: ChangeSetPersistence<'_>,
) -> Result<ChangeSetId, ExecError> {
    let ChangeSetPersistence {
        record,
        change_set_id,
        assignment,
        transition_id,
        draft,
        validation_results,
        handoff_manifest_id,
        index,
    } = persistence;

    let change_set = earmark_core::ChangeSet {
        id: change_set_id.clone(),
        run_id: record.run_id.clone(),
        transition_id: transition_id.to_string(),
        assignment_id: Some(assignment.id.clone()),
        agent_id: Some("execution_engine".to_string()),
        input_object_ids: assignment.input_object_ids.clone(),
        created_object_ids: draft.created_objects.clone(),
        created_relation_ids: draft.created_relations.clone(),
        updated_object_ids: draft.updated_objects.clone(),
        governance_event_ids: draft.governance_events.clone(),
        blocked_operations: draft.blocked_operations.clone(),
        unresolved_ambiguities: draft.unresolved_ambiguities.clone(),
        rejected_candidates: draft.rejected_candidates.clone(),
        validation_results,
        work_packet_id: None,
        handoff_manifest_id,
        created_at: Utc::now(),
    };

    let mut stored_change_set = StoredObject::new(
        Kind::ChangeSet,
        Some("change_set".to_string()),
        Standing::default(),
        Provenance::direct_input("execution_engine"),
        BTreeMap::from([(
            "title".to_string(),
            earmark_core::HeaderValue::String(format!("ChangeSet {}", change_set_id.as_str())),
        )]),
        StoredPayload::from_json_bytes(serde_json::to_vec_pretty(&change_set)?),
        vec![],
    );
    stored_change_set.envelope.id = ObjectId::parse(change_set_id.as_str()).unwrap();
    write_object_and_index(store, index, &stored_change_set)?;

    // Link standing requests via Relations
    for request in &draft.standing_requests {
        // Validate structural legality
        if let Err(_err) = earmark_core::validate_standing_request(request) {
            continue;
        }

        let stored_request = StoredObject::new(
            Kind::Object,
            Some("standing_transition_request".to_string()),
            Standing::default(),
            Provenance::direct_input("execution_engine"),
            BTreeMap::from([(
                "title".to_string(),
                earmark_core::HeaderValue::String(format!(
                    "Standing Request for {}",
                    request.target_object_id.as_str()
                )),
            )]),
            StoredPayload::from_json_bytes(serde_json::to_vec_pretty(request)?),
            vec![],
        );
        let request_ref = write_object_and_index(store, index, &stored_request)?;

        let rel_payload = RelationPayload {
            source: ObjectRef::new(
                ObjectId::parse(change_set_id.as_str()).unwrap(),
                stored_change_set.envelope.version_id.clone(),
                Kind::ChangeSet,
                None,
            ),
            target: ObjectRef::new(
                request_ref.id,
                request_ref.version_id,
                Kind::Object,
                Some("standing_transition_request".to_string()),
            ),
            relation_type: REL_TYPE_REQUESTS_STANDING.to_string(),
            qualifiers: BTreeMap::new(),
            scope: None,
        };
        persist_relation_canonical(
            store,
            index,
            rel_payload,
            Provenance::direct_input("execution_engine"),
            RelationCreationMode::PrivilegedSystem,
            None,
        )?;
    }
    record.change_sets.push(change_set_id.clone());
    Ok(change_set_id)
}

pub(crate) fn persist_assignment_update<S: CanonicalStore>(
    store: &S,
    index: &DerivedIndex,
    previous: &StoredObject,
    assignment: &TransitionAssignment,
) -> Result<(), ExecError> {
    let updated = StoredObject::with_parent(
        previous,
        Standing::default(),
        previous.envelope.headers.clone(),
        StoredPayload::from_json_bytes(serde_json::to_vec_pretty(assignment)?),
    );
    write_object_and_index(store, index, &updated)?;
    Ok(())
}

pub(crate) fn persist_transformation_failure<S: CanonicalStore>(
    store: &S,
    index: &DerivedIndex,
    assignment_head: &StoredObject,
    assignment: &TransitionAssignment,
    failed_change_set_id: Option<ChangeSetId>,
    error: &ExecError,
) -> Result<ObjectRef, ExecError> {
    let failure = TransformationFailure {
        run_id: assignment.run_id.clone(),
        transition_id: assignment.transition_id.clone(),
        assignment_id: assignment.id.clone(),
        failed_change_set_id,
        error_type: "execution_error".to_string(),
        message: error.to_string(),
        stack_trace: None,
        created_at: Utc::now(),
    };

    let stored = StoredObject::new(
        Kind::TransformationFailure,
        Some("transformation_failure".to_string()),
        Standing::default(),
        Provenance::direct_input("execution_engine"),
        BTreeMap::from([(
            "title".to_string(),
            earmark_core::HeaderValue::String(format!("Failure {}", assignment.transition_id)),
        )]),
        StoredPayload::from_json_bytes(serde_json::to_vec_pretty(&failure)?),
        vec![],
    );
    let version_ref = write_object_and_index(store, index, &stored)?;
    let failure_ref = ObjectRef::new(
        version_ref.id.clone(),
        version_ref.version_id.clone(),
        Kind::TransformationFailure,
        None,
    );

    // Link assignment head to failure via relation
    let rel_payload = RelationPayload {
        source: assignment_head.object_ref(),
        target: failure_ref.clone(),
        relation_type: REL_TYPE_RESULTED_IN_FAILURE.to_string(),
        qualifiers: BTreeMap::new(),
        scope: None,
    };
    persist_relation_canonical(
        store,
        index,
        rel_payload,
        Provenance::direct_input("execution_engine"),
        RelationCreationMode::PrivilegedSystem,
        None,
    )?;

    Ok(failure_ref)
}

pub(crate) fn create_local_transform_output<S: CanonicalStore>(
    store: &S,
    index: &DerivedIndex,
    instruction: &InstructionPayload,
    output_class: &str,
    inputs: &[ObjectRef],
    instruction_ref: &earmark_core::VersionRef,
) -> Result<TransformArtifacts, ExecError> {
    let body = format!(
        "# Candidate Output\n\nInstruction: {}\n\nPurpose: {}\n\nInputs:\n{}\n",
        instruction.name,
        instruction.purpose,
        inputs
            .iter()
            .map(|input| format!("- {}", input.id.as_str()))
            .collect::<Vec<_>>()
            .join("\n")
    );
    let stored = StoredObject::new(
        Kind::Object,
        Some(output_class.to_string()),
        Standing::default(),
        Provenance {
            actor: "runtime".to_string(),
            source_type: "local_transform".to_string(),
            source_ref: None,
            lineage: inputs
                .iter()
                .filter(|obj| obj.kind == Kind::Object)
                .cloned()
                .map(|object| earmark_core::LineageLink {
                    rel: "derived_from".to_string(),
                    object,
                })
                .chain(std::iter::once(earmark_core::LineageLink {
                    rel: "used_instruction".to_string(),
                    object: ObjectRef::new(
                        instruction_ref.id.clone(),
                        instruction_ref.version_id.clone(),
                        Kind::Instruction,
                        None,
                    ),
                }))
                .collect(),
            import_path: None,
            captured_at: Utc::now(),
        },
        BTreeMap::from([(
            "title".to_string(),
            earmark_core::HeaderValue::String(format!("{} candidate", instruction.name)),
        )]),
        StoredPayload::from_markdown(body),
        vec![],
    );
    write_object_and_index(store, index, &stored)?;
    let relation_ids =
        create_lineage_relations(store, index, &stored.object_ref(), inputs, instruction_ref)?;
    Ok(TransformArtifacts {
        output: stored.object_ref(),
        relation_ids,
    })
}

pub(crate) fn create_delegated_transform_output<S: CanonicalStore>(
    store: &S,
    index: &DerivedIndex,
    instruction: &InstructionPayload,
    output_class: &str,
    inputs: &[ObjectRef],
    instruction_ref: &earmark_core::VersionRef,
    response: ProviderResponse,
) -> Result<TransformArtifacts, ExecError> {
    let is_synthetic = provider_response_is_synthetic(&response);
    let synthetic_source = provider_metadata_synthetic_source(&response.metadata)
        .unwrap_or_else(|| "mock_provider".to_string());
    let mut headers = BTreeMap::from([
        (
            "title".to_string(),
            earmark_core::HeaderValue::String(format!("{} candidate", instruction.name)),
        ),
        (
            "provider".to_string(),
            earmark_core::HeaderValue::String(response.provider.clone()),
        ),
        (
            "model".to_string(),
            earmark_core::HeaderValue::String(response.model.clone()),
        ),
    ]);
    if is_synthetic {
        headers.insert(
            "synthetic".to_string(),
            earmark_core::HeaderValue::Bool(true),
        );
        headers.insert(
            "synthetic_source".to_string(),
            earmark_core::HeaderValue::String(synthetic_source),
        );
        headers.insert(
            "production_eligible".to_string(),
            earmark_core::HeaderValue::Bool(false),
        );
    }

    let stored = StoredObject::new(
        Kind::Object,
        Some(output_class.to_string()),
        Standing::default(),
        Provenance {
            actor: "runtime".to_string(),
            source_type: "delegated_transform".to_string(),
            source_ref: None,
            lineage: inputs
                .iter()
                .filter(|obj| obj.kind == Kind::Object)
                .cloned()
                .map(|object| earmark_core::LineageLink {
                    rel: "derived_from".to_string(),
                    object,
                })
                .collect(),
            import_path: None,
            captured_at: Utc::now(),
        },
        headers,
        StoredPayload::from_json_bytes(response.candidate_payload.into_bytes()),
        vec![],
    );
    write_object_and_index(store, index, &stored)?;
    let relation_ids =
        create_lineage_relations(store, index, &stored.object_ref(), inputs, instruction_ref)?;
    Ok(TransformArtifacts {
        output: stored.object_ref(),
        relation_ids,
    })
}

pub(crate) fn persist_run_record<S: CanonicalStore>(
    store: &S,
    index: &DerivedIndex,
    record: &RunRecord,
) -> Result<(), ExecError> {
    let stored = StoredObject::new(
        Kind::RunRecord,
        Some("run_record".to_string()),
        Standing::default(),
        Provenance::direct_input("runtime"),
        BTreeMap::from([(
            "title".to_string(),
            earmark_core::HeaderValue::String(format!("Run {}", record.run_id)),
        )]),
        StoredPayload::from_json_bytes(earmark_core::to_json_pretty(record)?.into_bytes()),
        vec![],
    );
    // RunRecord indexing?
    // Let's check if RunRecord should be indexed.
    // Usually yes, so we can list runs.
    write_object_and_index(store, index, &stored)?;
    Ok(())
}
