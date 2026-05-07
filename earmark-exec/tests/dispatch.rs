use std::{collections::BTreeMap, sync::Arc};

use earmark_core::{
    ProviderBudget, ProviderExposure, ProviderProfile, ProviderRequest, ProviderResponse,
    ProviderResponseContract, ProviderUsage, Kind, ObjectId, ObjectRef, VersionId, VersionRef,
};
use earmark_exec::{
    provider_record_from_failure, provide_with_registry, resolve_provider_profile,
    ProviderAdapter, ProviderFailure, ProviderFailureKind, ProviderMode, ProviderRegistry,
};

struct GoodAdapter;
impl ProviderAdapter for GoodAdapter {
    fn provider_key(&self) -> &'static str {
        "google"
    }

    fn provide(
        &self,
        request: ProviderRequest,
        _profile: &ProviderProfile,
    ) -> Result<ProviderResponse, ProviderFailure> {
        Ok(ProviderResponse {
            request_id: request.request_id,
            provider: "google".to_string(),
            model: "gemma".to_string(),
            status: "ok".to_string(),
            candidate_payload: r#"{"candidate":"ok"}"#.to_string(),
            metadata: BTreeMap::new(),
            usage: Some(ProviderUsage {
                input_tokens: Some(10),
                output_tokens: Some(5),
                estimated_cost_usd: Some(0.0),
                latency_ms: Some(123),
            }),
            received_at: chrono::Utc::now(),
        })
    }
}

struct MalformedAdapter;
impl ProviderAdapter for MalformedAdapter {
    fn provider_key(&self) -> &'static str {
        "local_http"
    }

    fn provide(
        &self,
        request: ProviderRequest,
        _profile: &ProviderProfile,
    ) -> Result<ProviderResponse, ProviderFailure> {
        Ok(ProviderResponse {
            request_id: request.request_id,
            provider: "local_http".to_string(),
            model: "broken".to_string(),
            status: "ok".to_string(),
            candidate_payload: "not-json".to_string(),
            metadata: BTreeMap::new(),
            usage: None,
            received_at: chrono::Utc::now(),
        })
    }
}

fn profile(provider: &str, model: &str) -> ProviderProfile {
    ProviderProfile {
        name: format!("{}-profile", provider),
        version: "1".to_string(),
        description: None,
        provider: provider.to_string(),
        model: model.to_string(),
        endpoint_env: None,
        auth_env: None,
        budget: ProviderBudget {
            max_input_tokens: None,
            max_output_tokens: None,
            max_cost_usd: None,
            max_latency_ms: None,
        },
        allowed_operations: vec!["transform".to_string()],
        exposure: ProviderExposure {
            allow_prose_objects: true,
            allow_structured_declarations: true,
            allow_work_surface_only: true,
            allow_export_requests: false,
        },
        response_contract: ProviderResponseContract {
            format: "json".to_string(),
            must_return_candidate_only: true,
            must_include_lineage: true,
        },
    }
}

fn request() -> ProviderRequest {
    ProviderRequest {
        request_id: "req_1".to_string(),
        run_id: "run_1".to_string(),
        work_packet: ObjectRef::new(ObjectId::new(), VersionId::new(), Kind::WorkPacket, None),
        provider_profile: VersionRef::new(ObjectId::new(), VersionId::new()),
        instruction_text: "Do the thing".to_string(),
        work_surface_manifest: None,
        inputs: vec![],
        response_contract: ProviderResponseContract {
            format: "json".to_string(),
            must_return_candidate_only: true,
            must_include_lineage: true,
        },
        issued_at: chrono::Utc::now(),
    }
}

#[test]
fn dispatch_resolution_precedence() {
    let operation = VersionRef::new(ObjectId::new(), VersionId::new());
    let instruction = earmark_core::InstructionPayload {
        name: "instr".to_string(),
        version: "1".to_string(),
        purpose: "test".to_string(),
        input_classes: vec![],
        output_classes: vec![],
        execution_policy: "runtime_dispatch_permitted".to_string(),
        provider_profile: Some(VersionRef::new(ObjectId::new(), VersionId::new())),
        trace_policy: "summary".to_string(),
        register: "machined".to_string(),
        body: earmark_core::MarkdownBody("body".to_string()),
    };
    let system = VersionRef::new(ObjectId::new(), VersionId::new());

    assert_eq!(
        resolve_provider_profile(Some(&operation), Some(&instruction), Some(&system)),
        ProviderMode::Delegated(operation)
    );
}

#[test]
fn adapter_not_registered_failure() {
    let registry = ProviderRegistry::default();
    let result = provide_with_registry(&registry, &profile("google", "gemma"), request());
    let err = result.err().unwrap();
    assert_eq!(err.kind, ProviderFailureKind::AdapterNotRegistered);
}

#[test]
fn malformed_response_handling() {
    let mut registry = ProviderRegistry::default();
    registry.register(Arc::new(MalformedAdapter));
    let result = provide_with_registry(&registry, &profile("local_http", "broken"), request());
    let err = result.err().unwrap();
    assert_eq!(err.kind, ProviderFailureKind::MalformedResponse);
}

#[test]
fn dispatch_record_creation() {
    let mut registry = ProviderRegistry::default();
    registry.register(Arc::new(GoodAdapter));
    let req = request();
    let prof = profile("google", "gemma");
    let outcome = provide_with_registry(&registry, &prof, req.clone()).unwrap();
    assert_eq!(outcome.record.provider, "google");
    assert_eq!(outcome.record.model, "gemma");

    let failure = ProviderFailure::new(ProviderFailureKind::Timeout, "timed out");
    let record = provider_record_from_failure(&req, &prof, &failure);
    assert!(record.message.unwrap().contains("timed out"));
}
