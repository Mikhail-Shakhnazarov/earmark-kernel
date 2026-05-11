use crate::error::ExecError;
use crate::handoff::{load_handoff, reconstruct_successor_inputs_from_handoff};
use crate::ir::WorkflowRunRequest;
use earmark_core::{
    ClassDefinition, InstructionPayload, Kind, ObjectRef, ProviderProfile, StandingPolicy,
    SystemDefinition, VersionRef, WorkflowDefinition,
};
use earmark_index::DerivedIndex;
use earmark_store::CanonicalStore;

pub(crate) fn resolve_version<S: CanonicalStore>(
    store: &S,
    version: &VersionRef,
) -> Result<VersionRef, ExecError> {
    if version.version_id.is_latest_sentinel() || version.version_id.as_str() == "latest" {
        store.read_head_ref(&version.id)?.ok_or_else(|| {
            ExecError::IncompleteExecution(format!(
                "latest version not found for object {}",
                version.id.as_str()
            ))
        })
    } else {
        Ok(version.clone())
    }
}

pub(crate) fn resolve_version_for_kind<S: CanonicalStore>(
    store: &S,
    index: &DerivedIndex,
    version: &VersionRef,
    expected_kind: Kind,
) -> Result<VersionRef, ExecError> {
    if let Ok(resolved) = resolve_version(store, version) {
        return Ok(resolved);
    }

    let symbolic = version.id.as_str();
    let resolved = match expected_kind {
        Kind::Workflow => index.resolve_workflow_symbolic_latest(symbolic)?,
        Kind::Instruction => index.resolve_instruction_symbolic_latest(symbolic)?,
        Kind::Object => index.resolve_class_definition_symbolic_latest(symbolic)?,
        Kind::CompiledContextTemplate => {
            index.resolve_compiled_context_symbolic_latest(symbolic)?
        }
        Kind::ProviderProfile => index.resolve_provider_profile_symbolic_latest(symbolic)?,
        Kind::Policy => index.resolve_standing_policy_symbolic_latest(symbolic)?,
        Kind::SystemDefinition => index.resolve_system_definition_symbolic_latest(symbolic)?,
        _ => None,
    };
    resolved.ok_or_else(|| {
        ExecError::IncompleteExecution(format!(
            "latest version not found for {} {}",
            expected_kind.as_str(),
            symbolic
        ))
    })
}

pub(crate) fn load_instruction<S: CanonicalStore>(
    store: &S,
    index: &DerivedIndex,
    version: &VersionRef,
) -> Result<InstructionPayload, ExecError> {
    let resolved = resolve_version_for_kind(store, index, version, Kind::Instruction)?;
    let stored = store.read_version(&resolved)?;
    Ok(InstructionPayload::parse_markdown(
        &stored.payload.as_utf8()?,
    )?)
}

pub(crate) fn load_provider_profile<S: CanonicalStore>(
    store: &S,
    index: &DerivedIndex,
    version: &VersionRef,
) -> Result<ProviderProfile, ExecError> {
    let resolved = resolve_version_for_kind(store, index, version, Kind::ProviderProfile)?;
    let stored = store.read_version(&resolved)?;
    Ok(earmark_core::parse_yaml(&stored.payload.as_utf8()?)?)
}

pub(crate) fn load_standing_policy<S: CanonicalStore>(
    store: &S,
    index: &DerivedIndex,
    version: &VersionRef,
) -> Result<StandingPolicy, ExecError> {
    let resolved = resolve_version_for_kind(store, index, version, Kind::Policy)?;
    let stored = store.read_version(&resolved)?;
    Ok(earmark_core::parse_yaml(&stored.payload.as_utf8()?)?)
}

pub(crate) fn load_system_definition<S: CanonicalStore>(
    store: &S,
    index: &DerivedIndex,
    version: &VersionRef,
) -> Result<SystemDefinition, ExecError> {
    let resolved = resolve_version_for_kind(store, index, version, Kind::SystemDefinition)?;
    let stored = store.read_version(&resolved)?;
    Ok(earmark_core::parse_yaml(&stored.payload.as_utf8()?)?)
}

pub(crate) fn load_class_definition<S: CanonicalStore>(
    store: &S,
    index: &DerivedIndex,
    version: &VersionRef,
) -> Result<ClassDefinition, ExecError> {
    let resolved = resolve_version_for_kind(store, index, version, Kind::Object)?;
    let stored = store.read_version(&resolved)?;
    Ok(earmark_core::parse_yaml(&stored.payload.as_utf8()?)?)
}

pub(crate) fn load_workflow<S: CanonicalStore>(
    store: &S,
    index: &DerivedIndex,
    version: &VersionRef,
) -> Result<WorkflowDefinition, ExecError> {
    let resolved = resolve_version_for_kind(store, index, version, Kind::Workflow)?;
    let stored = store.read_version(&resolved)?;
    Ok(earmark_core::parse_yaml(&stored.payload.as_utf8()?)?)
}

pub(crate) fn resolve_continuation_inputs<S: CanonicalStore>(
    store: &S,
    index: &DerivedIndex,
    request: &WorkflowRunRequest,
) -> Result<Vec<ObjectRef>, ExecError> {
    let sources = usize::from(!request.inputs.is_empty())
        + usize::from(request.handoff_manifest.is_some())
        + usize::from(request.transition_assignment.is_some());
    if sources == 0 {
        return Err(ExecError::MissingInput(
            "workflow run requires inputs, handoff manifest, or transition assignment".to_string(),
        ));
    }
    if sources > 1 {
        return Err(ExecError::ConflictingContinuationSources(
            "provide exactly one continuation source".to_string(),
        ));
    }

    if !request.inputs.is_empty() {
        return Ok(request.inputs.clone());
    }

    if let Some(handoff_id) = &request.handoff_manifest {
        let handoff = load_handoff(store, handoff_id)?;
        return reconstruct_successor_inputs_from_handoff(store, index, &handoff);
    }

    if let Some(assignment_id) = &request.transition_assignment {
        let (_stored, assignment) =
            crate::helpers::load_current_transition_assignment(store, assignment_id)?;
        if let Some(handoff_id) = &assignment.handoff_manifest_id {
            let handoff = load_handoff(store, handoff_id)?;
            return reconstruct_successor_inputs_from_handoff(store, index, &handoff);
        }
        let mut inputs = Vec::new();
        for id in &assignment.input_object_ids {
            if let Some(head) = store.read_head(id)? {
                inputs.push(head.object_ref());
            }
        }
        return Ok(inputs);
    }

    Ok(vec![])
}
