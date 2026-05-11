use crate::relation_logic::{
    RelationAuthorizationDecision, RelationAuthorizationResolver, RelationEndpointFacts,
};
use earmark_core::{
    ChangeSetDraft, ChangeSetValidationResult, ClassDefinition, Kind, ObjectId, ObjectRef,
    Standing, SystemDefinition, WorkflowGuard,
};
use earmark_index::DerivedIndex;
use earmark_store::{CanonicalStore, StoredObject};
use std::collections::{BTreeSet, HashMap, VecDeque};

use crate::error::ExecError;
use crate::ir::{ExecutionEdge, ExecutionIr, ExecutionTransition, WorkflowRunRequest};
use crate::resolution::load_class_definition;

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
    let mut info = Vec::new();
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
                &mut info,
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
            &mut info,
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

fn check_dimension(
    target_object_id: &ObjectId,
    standing: &Standing,
    class: &str,
    dim_id: &earmark_core::DimensionId,
    allowed_tokens: &[earmark_core::TokenId],
    failures: &mut Vec<String>,
    requests: &mut Vec<earmark_core::StandingTransitionRequest>,
) {
    if allowed_tokens.is_empty() {
        return;
    }
    let actual = standing.get(dim_id);
    let actual_str = actual.map(|t| t.as_str()).unwrap_or("unknown");
    if allowed_tokens.iter().any(|t| t.as_str() == actual_str) {
        return;
    }
    failures.push(format!(
        "created object class {} uses disallowed {} standing {}",
        class,
        dim_id.as_str(),
        actual_str
    ));
    if let Some(first) = allowed_tokens.first() {
        requests.push(earmark_core::StandingTransitionRequest {
            target_object_id: target_object_id.clone(),
            dimension: dim_id.as_str().to_string(),
            from_value: actual_str.to_string(),
            to_value: first.as_str().to_string(),
            rationale: Some("standing rule violation".to_string()),
            status: earmark_core::StandingRequestStatus::Proposed,
        });
    }
}

pub fn validate_standing_rules(
    target_object_id: &ObjectId,
    standing: &Standing,
    class: &str,
    rules: &earmark_core::ClassStandingRules,
    failures: &mut Vec<String>,
) -> Vec<earmark_core::StandingTransitionRequest> {
    let mut requests = Vec::new();

    for (dim_id, allowed_tokens) in &rules.allowed_standing {
        check_dimension(
            target_object_id,
            standing,
            class,
            dim_id,
            allowed_tokens,
            failures,
            &mut requests,
        );
    }

    requests
}

fn is_trusted_actor(actor: &str) -> bool {
    actor == "runtime" || actor == "execution_engine" || actor == "system"
}

pub fn validate_relation_object<S: CanonicalStore>(
    store: &S,
    object_id: &ObjectId,
    stored: &StoredObject,
    declared_classes: &HashMap<String, ClassDefinition>,
    failures: &mut Vec<String>,
    info: &mut Vec<String>,
) -> Result<(), ExecError> {
    let relation: earmark_core::RelationPayload = serde_json::from_slice(&stored.payload.bytes)?;
    if relation.relation_type.trim().is_empty() {
        failures.push(format!(
            "created relation {} has empty relation_type",
            object_id.as_str()
        ));
    }

    // Step 2 & 3: Load canonical endpoint versions and verify identity
    let source_stored = load_relation_endpoint(
        store,
        &relation.source.id,
        &relation.source.version_id,
        "source",
        failures,
    )?;
    let target_stored = load_relation_endpoint(
        store,
        &relation.target.id,
        &relation.target.version_id,
        "target",
        failures,
    )?;

    if let Some(source_stored) = &source_stored {
        verify_endpoint_identity(
            &relation.source,
            &source_stored.envelope,
            "source",
            failures,
        );
    }
    if let Some(target_stored) = &target_stored {
        verify_endpoint_identity(
            &relation.target,
            &target_stored.envelope,
            "target",
            failures,
        );
    }

    // Stop if we couldn't load endpoints or identity failed
    if !failures.is_empty() {
        return Ok(());
    }

    let Some(source_stored) = source_stored else {
        failures.push(format!(
            "relation {} source endpoint missing after load attempt",
            object_id.as_str()
        ));
        return Ok(());
    };
    let Some(target_stored) = target_stored else {
        failures.push(format!(
            "relation {} target endpoint missing after load attempt",
            object_id.as_str()
        ));
        return Ok(());
    };

    // Step 4: Construct facts
    let source_facts = RelationEndpointFacts {
        id: source_stored.envelope.id.clone(),
        version_id: source_stored.envelope.version_id.clone(),
        kind: source_stored.envelope.kind.clone(),
        class: source_stored.envelope.class.clone(),
    };
    let target_facts = RelationEndpointFacts {
        id: target_stored.envelope.id.clone(),
        version_id: target_stored.envelope.version_id.clone(),
        kind: target_stored.envelope.kind.clone(),
        class: target_stored.envelope.class.clone(),
    };

    // Step 5: Evaluate authorization
    let creation_mode = stored
        .envelope
        .headers
        .get("relation_creation_mode")
        .and_then(|v| v.as_string());

    let resolver = RelationAuthorizationResolver {
        relation_type: &relation.relation_type,
        source: &source_facts,
        target: &target_facts,
        source_definition: source_facts
            .class
            .as_ref()
            .and_then(|c| declared_classes.get(c)),
        target_definition: target_facts
            .class
            .as_ref()
            .and_then(|c| declared_classes.get(c)),
        creation_mode: creation_mode.as_deref(),
        is_trusted_provenance: is_trusted_actor(&stored.envelope.provenance.actor),
    };

    match resolver.resolve() {
        RelationAuthorizationDecision::Allowed(reason) => {
            info.push(format!("relation {} authorized: {}", object_id, reason));
        }
        RelationAuthorizationDecision::Blocked(failure) => {
            failures.push(format!(
                "relation {} authorization failed: {}",
                object_id, failure
            ));
        }
    }

    Ok(())
}

fn load_relation_endpoint<S: CanonicalStore>(
    store: &S,
    id: &ObjectId,
    version_id: &earmark_core::VersionId,
    role: &str,
    failures: &mut Vec<String>,
) -> Result<Option<StoredObject>, ExecError> {
    let version_ref = earmark_core::VersionRef::new(id.clone(), version_id.clone());
    match store.read_version(&version_ref) {
        Ok(stored) => Ok(Some(stored)),
        Err(_) => {
            failures.push(format!(
                "relation references missing {} version {} for object {}",
                role,
                version_id.as_str(),
                id.as_str()
            ));
            Ok(None)
        }
    }
}

fn verify_endpoint_identity(
    expected: &earmark_core::ObjectRef,
    actual: &earmark_core::Envelope,
    role: &str,
    failures: &mut Vec<String>,
) {
    if expected.id != actual.id {
        failures.push(format!(
            "relation {} object ID mismatch: payload has {}, canonical has {}",
            role, expected.id, actual.id
        ));
    }
    if expected.version_id != actual.version_id {
        failures.push(format!(
            "relation {} version ID mismatch: payload has {}, canonical has {}",
            role, expected.version_id, actual.version_id
        ));
    }
    if expected.kind != actual.kind {
        failures.push(format!(
            "relation {} kind mismatch: payload has {:?}, canonical has {:?}",
            role, expected.kind, actual.kind
        ));
    }
    if expected.class != actual.class {
        failures.push(format!(
            "relation {} class mismatch: payload has {:?}, canonical has {:?}",
            role, expected.class, actual.class
        ));
    }
}
