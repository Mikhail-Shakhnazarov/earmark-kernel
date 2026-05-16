use crate::error::ExecError;
use crate::helpers::dedupe_strings;
use crate::ir::{ExecutionIr, ExecutionTransition, SuccessorHandoffSpec};
use crate::relation::persist_relation_canonical;
use crate::resolution::load_class_definition;
use earmark_core::{
    HandoffManifest, HandoffManifestId, Kind, ObjectId, ObjectRef, RelationCreationMode,
    RelationPayload, RequiredCheck, StandingConstraint, REL_TYPE_DERIVED_FROM,
    REL_TYPE_REQUESTS_STANDING, REL_TYPE_USED_COMPILED_CONTEXT, REL_TYPE_USED_INSTRUCTION,
};
use earmark_index::{DerivedIndex, RelationEdge};
use earmark_store::{CanonicalStore, StoredObject};
use std::collections::{BTreeSet, VecDeque};

/// Base set of relation types that are always carried across handoffs.
pub const HANDOFF_BASE_RELATION_TYPES: &[&str] = &[
    REL_TYPE_DERIVED_FROM,
    REL_TYPE_USED_INSTRUCTION,
    REL_TYPE_USED_COMPILED_CONTEXT,
    REL_TYPE_REQUESTS_STANDING,
];

const MAX_EXPANSION_DEPTH: usize = 2;
const MAX_EXPANSION_OBJECTS: usize = 100;
const MAX_EXPANSION_RELATIONS: usize = 500;

pub(crate) fn load_handoff<S: CanonicalStore>(
    store: &S,
    handoff_manifest_id: &HandoffManifestId,
) -> Result<HandoffManifest, ExecError> {
    for object in store.scan_objects()?.scanned_objects {
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
                for (dim_id, tokens) in &class.standing_rules.allowed_standing {
                    let reqs: Vec<String> = tokens.iter().map(|t| t.as_str().to_string()).collect();
                    if !reqs.is_empty() {
                        standing_constraints.push(StandingConstraint {
                            constraint_type: dim_id.as_str().to_string(),
                            requirements: reqs,
                        });
                    }
                }
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

pub fn reconstruct_successor_inputs_from_handoff<S: CanonicalStore>(
    store: &S,
    index: &DerivedIndex,
    handoff: &HandoffManifest,
) -> Result<Vec<ObjectRef>, ExecError> {
    let mut inputs = Vec::new();
    let mut seen_ids = BTreeSet::<ObjectId>::new();

    let mut pending = VecDeque::new();
    for id in &handoff.root_object_ids {
        pending.push_back((id.clone(), 0));
    }
    for id in &handoff.newly_created_object_ids {
        pending.push_back((id.clone(), 0));
    }
    for id in &handoff.inherited_input_object_ids {
        pending.push_back((id.clone(), 0));
    }

    let mut objects_processed = 0;
    let mut relations_processed = 0;

    while let Some((current_id, depth)) = pending.pop_front() {
        if !seen_ids.insert(current_id.clone()) {
            continue;
        }

        objects_processed += 1;
        if objects_processed > MAX_EXPANSION_OBJECTS {
            return Err(ExecError::HandoffReconstruction(format!(
                "expansion object limit reached ({})",
                MAX_EXPANSION_OBJECTS
            )));
        }

        if let Some(head) = store.read_head(&current_id)? {
            // Check admissibility
            if !handoff_object_admissible(&head, handoff)? {
                continue;
            }

            inputs.push(head.object_ref());

            if depth >= MAX_EXPANSION_DEPTH {
                continue;
            }

            let edges: Vec<RelationEdge> = index.relation_adjacency(&current_id, false)?;
            for edge in edges {
                relations_processed += 1;
                if relations_processed > MAX_EXPANSION_RELATIONS {
                    return Err(ExecError::HandoffReconstruction(format!(
                        "expansion relation limit reached ({})",
                        MAX_EXPANSION_RELATIONS
                    )));
                }

                if !handoff.allowed_relation_types.contains(&edge.relation_type) {
                    continue;
                }
                let related_id_str = if edge.source_object_id == current_id.as_str() {
                    &edge.target_object_id
                } else {
                    &edge.source_object_id
                };
                let related_id = ObjectId::parse(related_id_str)?;
                pending.push_back((related_id, depth + 1));
            }
        }
    }
    Ok(inputs)
}

fn handoff_object_admissible(
    object: &StoredObject,
    handoff: &HandoffManifest,
) -> Result<bool, ExecError> {
    // 1. Kind check
    if object.envelope.kind != Kind::Object {
        return Ok(false);
    }

    // 2. Class check
    if !handoff.allowed_input_classes.is_empty() {
        let class = object.envelope.class.as_deref().unwrap_or("");
        if !handoff.allowed_input_classes.iter().any(|c| c == class) {
            return Ok(false);
        }
    }

    // 3. Standing check
    for constraint in &handoff.standing_constraints {
        let dim_id =
            earmark_core::DimensionId::parse(&constraint.constraint_type).map_err(|e| {
                ExecError::HandoffReconstruction(format!(
                    "invalid standing constraint dimension '{}': {}",
                    constraint.constraint_type, e
                ))
            })?;
        let actual_value = object
            .envelope
            .standing
            .get(&dim_id)
            .map(earmark_core::TokenId::as_str)
            .unwrap_or("unknown")
            .to_string();

        if !constraint.requirements.contains(&actual_value) {
            return Ok(false);
        }
    }

    Ok(true)
}

pub(crate) fn create_lineage_relations<S: CanonicalStore>(
    store: &S,
    index: &DerivedIndex,
    output: &ObjectRef,
    inputs: &[ObjectRef],
    instruction_ref: &earmark_core::VersionRef,
) -> Result<Vec<ObjectId>, ExecError> {
    let instruction_stored = store.read_version(instruction_ref)?;
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
            RelationCreationMode::PrivilegedSystem,
            None,
        )?;
        relation_ids.push(relation_ref.id);
    }

    let instruction_relation = RelationPayload {
        source: output.clone(),
        target: ObjectRef::new(
            instruction_ref.id.clone(),
            instruction_ref.version_id.clone(),
            instruction_stored.envelope.kind.clone(),
            instruction_stored.envelope.class.clone(),
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
