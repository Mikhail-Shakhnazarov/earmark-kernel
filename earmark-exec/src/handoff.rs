use crate::error::ExecError;
use crate::helpers::dedupe_strings;
use crate::ir::{ExecutionIr, ExecutionTransition, SuccessorHandoffSpec};
use crate::resolution::load_class_definition;
use crate::relation::persist_relation_canonical;
use earmark_core::{
    HandoffManifest, HandoffManifestId, Kind, ObjectId, ObjectRef, RelationCreationMode,
    RelationPayload, RequiredCheck, StandingConstraint, REL_TYPE_DERIVED_FROM,
    REL_TYPE_REQUESTS_STANDING, REL_TYPE_USED_COMPILED_CONTEXT, REL_TYPE_USED_INSTRUCTION,
};
use earmark_index::{DerivedIndex, RelationEdge};
use earmark_store::CanonicalStore;
use std::collections::{BTreeSet, VecDeque};

/// Base set of relation types that are always carried across handoffs.
pub const HANDOFF_BASE_RELATION_TYPES: &[&str] = &[
    REL_TYPE_DERIVED_FROM,
    REL_TYPE_USED_INSTRUCTION,
    REL_TYPE_USED_COMPILED_CONTEXT,
    REL_TYPE_REQUESTS_STANDING,
];

pub(crate) fn load_handoff<S: CanonicalStore>(
    store: &S,
    handoff_manifest_id: &HandoffManifestId,
) -> Result<HandoffManifest, ExecError> {
    for object in store.scan_objects()? {
        if object.envelope.kind != Kind::HandoffManifest {
            continue;
        }
        let manifest: HandoffManifest = serde_json::from_slice(&object.payload.bytes)?;
        if &manifest.id == handoff_manifest_id {
            return Ok(manifest);
        }
    }
    Err(ExecError::MissingHandoffManifest(
        handoff_manifest_id.as_str().to_string(),
    ))
}

pub(crate) fn derive_successor_handoff<S: CanonicalStore>(
    store: &S,
    index: &DerivedIndex,
    system: &earmark_core::SystemDefinition,
    ir: &ExecutionIr,
    transition: &ExecutionTransition,
) -> Result<Vec<SuccessorHandoffSpec>, ExecError> {
    let mut specs = Vec::new();
    for edge in ir.edges.iter().filter(|e| e.from == transition.id) {
        let successor = ir
            .transitions
            .iter()
            .find(|t| t.id == edge.to)
            .ok_or_else(|| {
                ExecError::InvalidWorkflow(format!("successor transition {} not found", edge.to))
            })?;

        let mut allowed_input_classes = Vec::new();
        let allowed_output_classes = successor.output_contracts.clone();
        let mut allowed_relation_types: Vec<String> = HANDOFF_BASE_RELATION_TYPES
            .iter()
            .map(|s| s.to_string())
            .collect();
        let mut standing_constraints = Vec::new();
        let mut required_checks = Vec::new();

        for contract in &successor.input_contracts {
            allowed_input_classes.push(contract.clone());
            for class_ref in &system.classes {
                let class = load_class_definition(store, index, class_ref)?;
                if &class.name != contract {
                    continue;
                }
                standing_constraints.push(StandingConstraint {
                    constraint_type: "allowed_epistemic".to_string(),
                    requirements: class
                        .standing_rules
                        .allowed_epistemic
                        .iter()
                        .map(|e| e.as_str().to_string())
                        .collect(),
                });
                standing_constraints.push(StandingConstraint {
                    constraint_type: "allowed_review".to_string(),
                    requirements: class
                        .standing_rules
                        .allowed_review
                        .iter()
                        .map(|r| r.as_str().to_string())
                        .collect(),
                });
                standing_constraints.push(StandingConstraint {
                    constraint_type: "allowed_process".to_string(),
                    requirements: class
                        .standing_rules
                        .allowed_process
                        .iter()
                        .map(|p| p.as_str().to_string())
                        .collect(),
                });
                for rule in &class.relation_rules {
                    allowed_relation_types.push(rule.relation_type.clone());
                }
            }
        }
        if !standing_constraints.is_empty() {
            required_checks.push(RequiredCheck {
                check_type: "standing_constraint_check".to_string(),
                description: "Verify all input objects meet the class standing rules of the target transition".to_string(),
            });
        }
        let allowed_relation_types_deduped = dedupe_strings(allowed_relation_types);

        specs.push(SuccessorHandoffSpec {
            to_transition_id: Some(successor.id.clone()),
            allowed_input_classes,
            allowed_output_classes,
            allowed_relation_types: allowed_relation_types_deduped,
            standing_constraints,
            required_checks,
            compiled_context_template_id: successor
                .compiled_context
                .as_ref()
                .map(|compiled_context| compiled_context.id.clone()),
        });
    }
    if specs.is_empty() {
        // For terminal transitions, we produce a neutral handoff spec
        // so that the output can be used by other workflows.
        specs.push(SuccessorHandoffSpec {
            to_transition_id: None,
            allowed_input_classes: transition.output_contracts.clone(),
            allowed_output_classes: Vec::new(),
            allowed_relation_types: HANDOFF_BASE_RELATION_TYPES
                .iter()
                .map(|s| s.to_string())
                .collect(),
            standing_constraints: Vec::new(),
            required_checks: Vec::new(),
            compiled_context_template_id: None,
        });
    }
    Ok(specs)
}

pub(crate) fn reconstruct_successor_inputs_from_handoff<S: CanonicalStore>(
    store: &S,
    index: &DerivedIndex,
    handoff: &HandoffManifest,
) -> Result<Vec<ObjectRef>, ExecError> {
    let mut inputs = Vec::new();
    let mut seen_ids = BTreeSet::<ObjectId>::new();

    let mut pending_ids = VecDeque::new();
    for id in &handoff.root_object_ids {
        pending_ids.push_back(id.clone());
    }
    for id in &handoff.newly_created_object_ids {
        pending_ids.push_back(id.clone());
    }
    for id in &handoff.inherited_input_object_ids {
        pending_ids.push_back(id.clone());
    }

    while let Some(current_id) = pending_ids.pop_front() {
        if !seen_ids.insert(current_id.clone()) {
            continue;
        }

        if let Some(head) = store.read_head(&current_id)? {
            inputs.push(head.object_ref());

            let edges: Vec<RelationEdge> = index.relation_adjacency(&current_id)?;
            for edge in edges {
                if !handoff.allowed_relation_types.contains(&edge.relation_type) {
                    continue;
                }
                let related_id_str = if edge.source_object_id == current_id.as_str() {
                    &edge.target_object_id
                } else {
                    &edge.source_object_id
                };
                let related_id = ObjectId::parse(related_id_str)?;
                pending_ids.push_back(related_id);
            }
        }
    }
    Ok(inputs)
}

pub(crate) fn create_lineage_relations<S: CanonicalStore>(
    store: &S,
    index: &DerivedIndex,
    output: &ObjectRef,
    inputs: &[ObjectRef],
    instruction_ref: &earmark_core::VersionRef,
) -> Result<Vec<ObjectId>, ExecError> {
    let mut relation_ids = Vec::new();
    for input in inputs {
        if input.kind != Kind::Object {
            continue;
        }
        let relation = RelationPayload {
            source: output.clone(),
            target: input.clone(),
            relation_type: REL_TYPE_DERIVED_FROM.to_string(),
            qualifiers: std::collections::BTreeMap::new(),
            scope: None,
        };
        let relation_ref = persist_relation_canonical(
            store,
            index,
            relation,
            earmark_core::Provenance::direct_input("runtime"),
            RelationCreationMode::Declared,
            None,
        )?;
        relation_ids.push(relation_ref.id);
    }

    let instruction_relation = RelationPayload {
        source: output.clone(),
        target: ObjectRef::new(
            instruction_ref.id.clone(),
            instruction_ref.version_id.clone(),
            Kind::Instruction,
            None,
        ),
        relation_type: REL_TYPE_USED_INSTRUCTION.to_string(),
        qualifiers: std::collections::BTreeMap::new(),
        scope: None,
    };
    let relation_ref = persist_relation_canonical(
        store,
        index,
        instruction_relation,
        earmark_core::Provenance::direct_input("runtime"),
        RelationCreationMode::PrivilegedSystem,
        None,
    )?;
    relation_ids.push(relation_ref.id);

    Ok(relation_ids)
}
