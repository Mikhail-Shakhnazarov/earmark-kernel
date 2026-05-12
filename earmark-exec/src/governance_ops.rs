use crate::error::ExecError;
use crate::persistence_helpers::write_object_and_index;
use crate::resolution::load_standing_policy;
use chrono::Utc;
use earmark_core::projection::project;
use earmark_core::{
    DimensionId, HeaderValue, Kind, ObjectId, Provenance, Standing, StandingRegistry,
    StandingRequestStatus, StandingTransitionRequest, TokenId, VersionRef,
};
use earmark_governance::{check_immutability, validate_standing_transition, ReviewPayload};
use earmark_index::DerivedIndex;
use earmark_store::{CanonicalStore, StoredObject, StoredPayload};
use std::collections::BTreeMap;

pub fn approve_standing_request<S: CanonicalStore>(
    store: &S,
    index: &DerivedIndex,
    request_ref: &VersionRef,
    reason: Option<String>,
) -> Result<VersionRef, ExecError> {
    let mut request = load_standing_request(store, request_ref)?;

    if request.status != StandingRequestStatus::Proposed {
        return Err(ExecError::GovernanceOperation(format!(
            "cannot approve request with status {:?}",
            request.status
        )));
    }

    request.status = StandingRequestStatus::Approved;
    if let Some(r) = reason {
        request.rationale = Some(r);
    }

    persist_request_update(store, index, request_ref, &request)
}

pub fn reject_standing_request<S: CanonicalStore>(
    store: &S,
    index: &DerivedIndex,
    request_ref: &VersionRef,
    reason: Option<String>,
) -> Result<VersionRef, ExecError> {
    let mut request = load_standing_request(store, request_ref)?;

    if request.status != StandingRequestStatus::Proposed
        && request.status != StandingRequestStatus::Approved
    {
        return Err(ExecError::GovernanceOperation(format!(
            "cannot reject request with status {:?}",
            request.status
        )));
    }

    request.status = StandingRequestStatus::Rejected;
    if let Some(r) = reason {
        request.rationale = Some(r);
    }

    persist_request_update(store, index, request_ref, &request)
}

pub fn apply_standing_request<S: CanonicalStore>(
    store: &S,
    index: &DerivedIndex,
    request_ref: &VersionRef,
    policy_id: Option<&str>,
    reason: Option<String>,
    registry: &StandingRegistry,
) -> Result<(VersionRef, VersionRef), ExecError> {
    let mut request = load_standing_request(store, request_ref)?;

    if request.status != StandingRequestStatus::Approved {
        return Err(ExecError::GovernanceOperation(format!(
            "cannot apply request with status {:?}",
            request.status
        )));
    }

    // 1. Load target object
    let target_id = &request.target_object_id;
    let target_head_ref = index.get_head(target_id)?.ok_or_else(|| {
        ExecError::GovernanceOperation(format!("target object {} not found", target_id.as_str()))
    })?;
    let target_head = store.read_version(&target_head_ref)?;
    let current_standing = &target_head.envelope.standing;

    // 1b. Enforce immutability: reject if sealed
    check_immutability(registry, current_standing)
        .map_err(|e| ExecError::GovernanceOperation(e.to_string()))?;

    // 1c. Drift Check: verify current standing matches request.from_value
    let dim_id = DimensionId::parse(&request.dimension)
        .map_err(|e| ExecError::GovernanceOperation(format!("invalid dimension: {}", e)))?;
    let current_value = current_standing
        .get(&dim_id)
        .map(TokenId::as_str)
        .unwrap_or("unknown")
        .to_string();

    if current_value != request.from_value.to_lowercase() {
        return Err(ExecError::GovernanceOperation(format!(
            "drift detected: target object {} standing for {} is {}, but request expected {}",
            target_id.as_str(),
            request.dimension,
            current_value,
            request.from_value
        )));
    }

    // 2. Load policy
    let policy_ref = if let Some(pid) = policy_id {
        index
            .get_head(
                &ObjectId::parse(pid).map_err(|e| ExecError::GovernanceOperation(e.to_string()))?,
            )?
            .ok_or_else(|| ExecError::GovernanceOperation(format!("policy {} not found", pid)))?
    } else {
        return Err(ExecError::GovernanceOperation(
            "policy required for application".to_string(),
        ));
    };
    let policy = load_standing_policy(store, index, &policy_ref)?;

    // 3. Construct requested standing
    let mut next_standing = target_head.envelope.standing.clone();
    next_standing
        .values
        .insert(dim_id.clone(), TokenId::new(&request.to_value));

    // 3b. No-op Protection: skip version creation if already at next_standing
    if next_standing == *current_standing {
        request.status = StandingRequestStatus::Applied;
        if let Some(r) = reason {
            request.rationale = Some(r);
        }
        let next_request_ref = persist_request_update(store, index, request_ref, &request)?;
        return Ok((target_head_ref, next_request_ref));
    }

    // 4. Validate transition
    let transition_res = validate_standing_transition(
        &policy,
        registry,
        &target_head.envelope.standing,
        &next_standing,
    )?;

    // 5. Enforce review if required by policy rule
    if transition_res.requires_review && !has_accepted_review(store, index, &target_head_ref)? {
        return Err(ExecError::GovernanceOperation(
            "transition requires accepted review evidence for the current version".to_string(),
        ));
    }

    // 5b. Enforce existing version-matched accepted review evidence for transitions
    //     into accepted review projection.  Uses a global index scan (not same-change-set).
    //     See validate_transition_change_set in validation.rs for the same-change-set path.
    let requested_projection = project(&next_standing, registry)
        .map_err(|e| ExecError::GovernanceOperation(format!("projection error: {}", e)))?;
    if requested_projection.review == Some(earmark_core::projection::ReviewProjection::Accepted) {
        let actor = target_head.envelope.provenance.actor.as_str();
        if !earmark_governance::is_trusted_actor(actor)
            && !has_accepted_review(store, index, &target_head_ref)?
        {
            return Err(ExecError::GovernanceOperation(
                "transition into accepted review projection requires existing accepted review \
                 evidence targeting the current object version"
                    .to_string(),
            ));
        }
    }

    // 6. Create new target version
    let mut next_target = target_head.clone();
    next_target.envelope.standing = next_standing;
    next_target.envelope.parents = vec![target_head_ref];
    next_target.envelope.version_id = earmark_core::VersionId::new();
    next_target.envelope.updated_at = Utc::now();

    let next_target_ref = write_object_and_index(store, index, &next_target)?;

    // 7. Update request status to Applied
    request.status = StandingRequestStatus::Applied;
    if let Some(r) = reason {
        request.rationale = Some(r);
    }
    let next_request_ref = persist_request_update(store, index, request_ref, &request)?;

    Ok((next_target_ref, next_request_ref))
}

fn load_standing_request<S: CanonicalStore>(
    store: &S,
    request_ref: &VersionRef,
) -> Result<StandingTransitionRequest, ExecError> {
    let obj = store.read_version(request_ref)?;
    let request: StandingTransitionRequest = serde_json::from_slice(&obj.payload.bytes)?;
    Ok(request)
}

fn persist_request_update<S: CanonicalStore>(
    store: &S,
    index: &DerivedIndex,
    parent_ref: &VersionRef,
    request: &StandingTransitionRequest,
) -> Result<VersionRef, ExecError> {
    let stored = StoredObject::new_with_id(
        parent_ref.id.clone(),
        Kind::Object,
        Some("standing_transition_request".to_string()),
        Standing::default(),
        Provenance::direct_input("governance"),
        BTreeMap::from([(
            "title".to_string(),
            HeaderValue::String(format!(
                "Standing Request Update for {}",
                request.target_object_id.as_str()
            )),
        )]),
        StoredPayload::from_json_bytes(serde_json::to_vec_pretty(request)?),
        vec![parent_ref.clone()],
    );

    write_object_and_index(store, index, &stored)
}

/// Scans all indexed Review objects for an accepted review targeting the exact
/// object version.  This is a *global* scan (not restricted to a change set)
/// and implements *existing version-matched review evidence*, not same-change-set
/// authorization.  See `validate_transition_change_set` in validation.rs for the
/// same-change-set path used during change-set validation.
fn has_accepted_review<S: CanonicalStore>(
    store: &S,
    index: &DerivedIndex,
    target_ref: &VersionRef,
) -> Result<bool, ExecError> {
    let reviews = index.get_objects_by_kind(Kind::Review)?;
    for review_ref in reviews {
        let obj = store.read_version(&review_ref)?;
        let payload: ReviewPayload = serde_json::from_slice(&obj.payload.bytes)?;
        if payload.target.id == target_ref.id
            && payload.target.version_id == target_ref.version_id
            && payload.status == "accepted"
        {
            return Ok(true);
        }
    }
    Ok(false)
}
