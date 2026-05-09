use std::collections::{BTreeSet, HashMap, VecDeque};
use earmark_core::{
    ChangeSetDraft, ChangeSetValidationResult, ClassDefinition, Kind, ObjectId, ObjectRef,
    Standing, SystemDefinition, WorkflowGuard,
};
use earmark_store::{CanonicalStore, StoredObject};
use earmark_index::DerivedIndex;

use crate::error::ExecError;
use crate::resolution::load_class_definition;
use crate::ir::{
    ExecutionEdge, ExecutionIr, ExecutionTransition, WorkflowRunRequest,
};

pub fn reachability_warnings(ir: &ExecutionIr) -> Vec<String> {
    if ir.transitions.is_empty() {
        return vec!["workflow has no transitions".to_string()];
    }

    let entries = entry_transition_ids(ir);
    if entries.is_empty() {
        return vec![
            "workflow has no entry transition (all transitions have predecessors)".to_string(),
        ];
    }

    let mut queue = VecDeque::from(entries.clone());
    let mut seen = entries.into_iter().collect::<BTreeSet<_>>();
    while let Some(node) = queue.pop_front() {
        for edge in ir.edges.iter().filter(|edge| edge.from == node) {
            if seen.insert(edge.to.clone()) {
                queue.push_back(edge.to.clone());
            }
        }
    }
    ir.transitions
        .iter()
        .filter(|transition| !seen.contains(&transition.id))
        .map(|transition| format!("unreachable transition: {}", transition.id))
        .collect()
}

pub fn deadlock_warnings(ir: &ExecutionIr) -> Vec<String> {
    ir.transitions
        .iter()
        .filter(|transition| {
            !ir.edges.iter().any(|edge| edge.from == transition.id)
                && ir.edges.iter().any(|edge| edge.to == transition.id)
        })
        .map(|transition| format!("transition has no outgoing edge: {}", transition.id))
        .collect()
}

pub(crate) fn entry_transition_ids(ir: &ExecutionIr) -> Vec<String> {
    ir.transitions
        .iter()
        .filter(|transition| !ir.edges.iter().any(|edge| edge.to == transition.id))
        .map(|transition| transition.id.clone())
        .collect()
}

pub(crate) fn incoming_edges(ir: &ExecutionIr) -> HashMap<String, Vec<ExecutionEdge>> {
    let mut incoming = HashMap::new();
    for edge in &ir.edges {
        incoming
            .entry(edge.to.clone())
            .or_insert_with(Vec::new)
            .push(edge.clone());
    }
    incoming
}

pub(crate) fn outgoing_edges(ir: &ExecutionIr) -> HashMap<String, Vec<ExecutionEdge>> {
    let mut outgoing = HashMap::new();
    for edge in &ir.edges {
        outgoing
            .entry(edge.from.clone())
            .or_insert_with(Vec::new)
            .push(edge.clone());
    }
    outgoing
}

pub(crate) fn initial_contracts(inputs: &[ObjectRef]) -> BTreeSet<String> {
    let mut contracts = BTreeSet::from(["input".to_string()]);
    for input in inputs {
        if let Some(class) = &input.class {
            contracts.insert(class.clone());
        }
        contracts.insert(format!("kind:{}", input.kind.as_str()));
    }
    contracts
}

pub(crate) fn transition_is_ready(
    transition: &ExecutionTransition,
    available_contracts: &BTreeSet<String>,
    executed: &BTreeSet<String>,
    incoming: &HashMap<String, Vec<ExecutionEdge>>,
    ir: &ExecutionIr,
    request: &WorkflowRunRequest,
    active_objects: &[ObjectRef],
) -> Result<bool, ExecError> {
    let contracts_ready = transition
        .input_contracts
        .iter()
        .all(|contract| available_contracts.contains(contract));
    if !contracts_ready {
        return Ok(false);
    }

    if let Some(predecessors) = incoming.get(&transition.id) {
        if !predecessors
            .iter()
            .all(|edge| executed.contains(&edge.from))
        {
            return Ok(false);
        }
    }

    transition_guards_allow(transition, ir, request, available_contracts, active_objects)
}

pub(crate) fn transition_guards_allow(
    transition: &ExecutionTransition,
    ir: &ExecutionIr,
    request: &WorkflowRunRequest,
    available_contracts: &BTreeSet<String>,
    active_objects: &[ObjectRef],
) -> Result<bool, ExecError> {
    for guard in ir.guards.iter().filter(|guard| {
        guard.id == transition.id || guard.id == format!("transition:{}", transition.id)
    }) {
        let expression = resolve_guard_expression(&guard.expression, &ir.guards)?;
        if !evaluate_guard_expression(&expression, request, available_contracts, active_objects)? {
            return Ok(false);
        }
    }
    Ok(true)
}

pub(crate) fn edge_condition_allows(
    edge: &ExecutionEdge,
    ir: &ExecutionIr,
    request: &WorkflowRunRequest,
    available_contracts: &BTreeSet<String>,
    active_objects: &[ObjectRef],
) -> Result<bool, ExecError> {
    match &edge.condition {
        None => Ok(true),
        Some(condition) => {
            let expression = resolve_guard_expression(condition, &ir.guards)?;
            evaluate_guard_expression(&expression, request, available_contracts, active_objects)
        }
    }
}

pub(crate) fn resolve_guard_expression(
    condition_or_guard: &str,
    guards: &[WorkflowGuard],
) -> Result<String, ExecError> {
    if let Some(guard) = guards.iter().find(|guard| guard.id == condition_or_guard) {
        return Ok(guard.expression.clone());
    }
    validate_guard_expression(condition_or_guard)?;
    Ok(condition_or_guard.to_string())
}

pub fn validate_guard_expression(expression: &str) -> Result<(), ExecError> {
    let expr = expression.trim();
    if matches!(
        expr,
        "true" | "false" | "always" | "never" | "operator_approved" | "!operator_approved"
    ) {
        return Ok(());
    }
    if expr == "has_active_object"
        || expr == "not has_active_object"
        || expr == "!has_active_object"
    {
        return Ok(());
    }
    if expr.starts_with("has_contract:")
        || expr.starts_with("!has_contract:")
        || expr.starts_with("missing_contract:")
        || (expr.starts_with("has_contract(") && expr.ends_with(')'))
    {
        return Ok(());
    }
    Err(ExecError::InvalidWorkflow(format!(
        "unsupported guard expression {}",
        expression
    )))
}

pub(crate) fn evaluate_guard_expression(
    expression: &str,
    request: &WorkflowRunRequest,
    available_contracts: &BTreeSet<String>,
    active_objects: &[ObjectRef],
) -> Result<bool, ExecError> {
    let expr = expression.trim();
    match expr {
        "true" | "always" => Ok(true),
        "false" | "never" => Ok(false),
        "operator_approved" => Ok(request.operator_approved),
        "!operator_approved" | "not operator_approved" => Ok(!request.operator_approved),
        "has_active_object" => Ok(!active_objects.is_empty()),
        "!has_active_object" | "not has_active_object" => Ok(active_objects.is_empty()),
        _ if expr.starts_with("has_contract:") => {
            let contract = expr.trim_start_matches("has_contract:").trim();
            Ok(available_contracts.contains(contract))
        }
        _ if expr.starts_with("!has_contract:") => {
            let contract = expr.trim_start_matches("!has_contract:").trim();
            Ok(!available_contracts.contains(contract))
        }
        _ if expr.starts_with("missing_contract:") => {
            let contract = expr.trim_start_matches("missing_contract:").trim();
            Ok(!available_contracts.contains(contract))
        }
        _ if expr.starts_with("has_contract(") && expr.ends_with(')') => {
            let contract = expr
                .trim_start_matches("has_contract(")
                .trim_end_matches(')')
                .trim();
            Ok(available_contracts.contains(contract))
        }
        _ => Err(ExecError::InvalidWorkflow(format!(
            "unsupported guard expression {}",
            expression
        ))),
    }
}

pub fn validate_transition_change_set<S: CanonicalStore>(
    store: &S,
    index: &DerivedIndex,
    system: &SystemDefinition,
    transition: &ExecutionTransition,
    assignment: &earmark_core::TransitionAssignment,
    change_set_draft: &ChangeSetDraft,
) -> Result<
    (
        ChangeSetValidationResult,
        Vec<earmark_core::StandingTransitionRequest>,
    ),
    ExecError,
> {
    let declared_classes = system
        .classes
        .iter()
        .map(|reference| {
            let class = load_class_definition(store, index, reference)?;
            Ok((class.name.clone(), class))
        })
        .collect::<Result<HashMap<_, _>, ExecError>>()?;

    let mut failures = Vec::new();
    let warnings = Vec::new();
    let info = Vec::new();
    let mut created_output_classes = Vec::new();
    let mut all_standing_requests = Vec::new();

    for object_id in &change_set_draft.created_objects {
        let stored = store.read_head(object_id)?.ok_or_else(|| {
            ExecError::IncompleteExecution(format!(
                "created object {} is missing from canonical store",
                object_id.as_str()
            ))
        })?;

        match stored.envelope.kind {
            Kind::Object => {
                let class = stored.envelope.class.clone().ok_or_else(|| {
                    ExecError::IncompleteExecution(format!(
                        "created object {} is missing class metadata",
                        object_id.as_str()
                    ))
                })?;
                created_output_classes.push(class.clone());
                if !declared_classes.contains_key(&class) {
                    failures.push(format!("created object uses undeclared class {}", class));
                } else if let Some(definition) = declared_classes.get(&class) {
                    let reqs = validate_standing_rules(
                        object_id,
                        &stored.envelope.standing,
                        &class,
                        &definition.standing_rules,
                        &mut failures,
                    );
                    all_standing_requests.extend(reqs);
                }
            }
            Kind::Relation => validate_relation_object(
                store,
                object_id,
                &stored,
                &declared_classes,
                &mut failures,
            )?,
            _ => {}
        }
    }

    for relation_id in &change_set_draft.created_relations {
        let stored = store.read_head(relation_id)?.ok_or_else(|| {
            ExecError::IncompleteExecution(format!(
                "created relation {} is missing from canonical store",
                relation_id.as_str()
            ))
        })?;
        validate_relation_object(
            store,
            relation_id,
            &stored,
            &declared_classes,
            &mut failures,
        )?;
    }

    if !transition.output_contracts.is_empty() {
        if created_output_classes.is_empty() && transition.operation == "transform" {
            failures.push(format!(
                "transition {} declared output contract(s) {:?} but produced no object-class outputs",
                transition.id, transition.output_contracts
            ));
        }
        for contract in &transition.output_contracts {
            if !created_output_classes.is_empty()
                && !created_output_classes.iter().any(|class| class == contract)
            {
                failures.push(format!(
                    "transition {} expected output contract {} but produced classes {:?}",
                    transition.id, contract, created_output_classes
                ));
            }
        }
    }

    for input_object_id in &assignment.input_object_ids {
        if store.read_head(input_object_id)?.is_none() {
            failures.push(format!(
                "assignment references missing input object {}",
                input_object_id.as_str()
            ));
        }
    }

    let is_valid = failures.is_empty();
    let result = ChangeSetValidationResult {
        is_valid,
        failures,
        warnings,
        info,
    };

    Ok((result, all_standing_requests))
}

pub fn validate_standing_rules(
    target_object_id: &ObjectId,
    standing: &Standing,
    class: &str,
    rules: &earmark_core::ClassStandingRules,
    failures: &mut Vec<String>,
) -> Vec<earmark_core::StandingTransitionRequest> {
    let mut requests = Vec::new();

    if !rules.allowed_epistemic.is_empty() && !rules.allowed_epistemic.contains(&standing.epistemic)
    {
        let actual = standing.epistemic.as_str().to_string();
        failures.push(format!(
            "created object class {} uses disallowed epistemic standing {}",
            class, actual
        ));
        if let Some(first) = rules.allowed_epistemic.first() {
            requests.push(earmark_core::StandingTransitionRequest {
                target_object_id: target_object_id.clone(),
                dimension: "epistemic".to_string(),
                from_value: actual,
                to_value: first.as_str().to_string(),
                rationale: Some("standing rule violation".to_string()),
                status: earmark_core::StandingRequestStatus::Proposed,
            });
        }
    }

    if !rules.allowed_review.is_empty() && !rules.allowed_review.contains(&standing.review) {
        let actual = standing.review.as_str().to_string();
        failures.push(format!(
            "created object class {} uses disallowed review standing {}",
            class, actual
        ));
        if let Some(first) = rules.allowed_review.first() {
            requests.push(earmark_core::StandingTransitionRequest {
                target_object_id: target_object_id.clone(),
                dimension: "review".to_string(),
                from_value: actual,
                to_value: first.as_str().to_string(),
                rationale: Some("standing rule violation".to_string()),
                status: earmark_core::StandingRequestStatus::Proposed,
            });
        }
    }

    if !rules.allowed_process.is_empty() && !rules.allowed_process.contains(&standing.process) {
        let actual = standing.process.as_str().to_string();
        failures.push(format!(
            "created object class {} uses disallowed process standing {}",
            class, actual
        ));
        if let Some(first) = rules.allowed_process.first() {
            requests.push(earmark_core::StandingTransitionRequest {
                target_object_id: target_object_id.clone(),
                dimension: "process".to_string(),
                from_value: actual,
                to_value: first.as_str().to_string(),
                rationale: Some("standing rule violation".to_string()),
                status: earmark_core::StandingRequestStatus::Proposed,
            });
        }
    }

    requests
}

pub(crate) fn validate_relation_object<S: CanonicalStore>(
    store: &S,
    object_id: &ObjectId,
    stored: &StoredObject,
    declared_classes: &HashMap<String, ClassDefinition>,
    failures: &mut Vec<String>,
) -> Result<(), ExecError> {
    let relation: earmark_core::RelationPayload = serde_json::from_slice(&stored.payload.bytes)?;
    if relation.relation_type.trim().is_empty() {
        failures.push(format!(
            "created relation {} has empty relation_type",
            object_id.as_str()
        ));
    }
    if store.read_head(&relation.source.id)?.is_none() {
        failures.push(format!(
            "created relation {} references missing source {}",
            object_id.as_str(), relation.source.id.as_str()
        ));
    }
    if store.read_head(&relation.target.id)?.is_none() {
        failures.push(format!(
            "created relation {} references missing target {}",
            object_id.as_str(), relation.target.id.as_str()
        ));
    }
    if let Some(source_class) = &relation.source.class {
        if let Some(definition) = declared_classes.get(source_class) {
            let relation_allowed = relation.relation_type == "used_instruction"
                || relation.relation_type == "used_compiled_context"
                || definition.relation_rules.iter().any(|rule| {
                    rule.relation_type == relation.relation_type
                        && (rule.target_classes.is_empty()
                            || relation
                                .target
                                .class
                                .as_ref()
                                .map(|target_class| rule.target_classes.contains(target_class))
                                .unwrap_or(false))
                });
            if !relation_allowed && !definition.relation_rules.is_empty() {
                failures.push(format!(
                    "relation {} is not allowed from class {}",
                    relation.relation_type, source_class
                ));
            }
        }
    }
    Ok(())
}
