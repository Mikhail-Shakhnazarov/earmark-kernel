use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use earmark_core::{FlexibleVersionRef, VersionRef, WorkflowDeclaration, WorkflowDefinition, WorkflowOperation};
use crate::DeriveError;

pub fn resolve_workflow_declaration(
    workflow_path: &Path,
    declaration: WorkflowDeclaration,
    registry: &BTreeMap<PathBuf, VersionRef>,
) -> Result<WorkflowDefinition, DeriveError> {
    let mut operations = Vec::new();
    for op in declaration.operations {
        let op_id = op.id.clone();
        operations.push(WorkflowOperation {
            id: op.id.clone(),
            kind: op.kind.clone(),
            input_contracts: op.input_contracts.clone(),
            output_contracts: op.output_contracts.clone(),
            instruction: resolve_flex_ref(workflow_path, &op_id, "instruction", op.instruction, registry)?,
            compiled_context: resolve_flex_ref(workflow_path, &op_id, "compiled_context", op.compiled_context, registry)?,
            policy: resolve_flex_ref(workflow_path, &op_id, "policy", op.policy, registry)?,
            provider_profile: resolve_flex_ref(workflow_path, &op_id, "provider_profile", op.provider_profile, registry)?,
        });
    }

    Ok(WorkflowDefinition {
        name: declaration.name,
        version: declaration.version,
        description: declaration.description,
        operations,
        edges: declaration.edges,
        guards: declaration.guards,
        output_contracts: declaration.output_contracts,
    })
}

fn resolve_flex_ref(
    workflow_path: &Path,
    op_id: &str,
    field_name: &str,
    flex: Option<FlexibleVersionRef>,
    registry: &BTreeMap<PathBuf, VersionRef>,
) -> Result<Option<VersionRef>, DeriveError> {
    match flex {
        None => Ok(None),
        Some(FlexibleVersionRef::Ref(r)) => Ok(Some(r)),
        Some(FlexibleVersionRef::Path(p)) => {
            let rel_path = PathBuf::from(&p);
            let parent = workflow_path.parent().unwrap_or(workflow_path);
            let abs_path = parent.join(&rel_path);

            // Try to canonicalize for robust matching, but fall back to joined path
            let lookup_path = abs_path.canonicalize().unwrap_or_else(|_| abs_path.clone());

            if let Some(vref) = registry.get(&lookup_path) {
                Ok(Some(vref.clone()))
            } else {
                Err(DeriveError::Validation(format!(
                    "invalid workflow reference at operation '{}'.{}: unresolved path reference '{}' in workflow '{}'. Referenced declaration must be included in the system manifest.",
                    op_id,
                    field_name,
                    p,
                    workflow_path.display()
                )))
            }
        }
    }
}
