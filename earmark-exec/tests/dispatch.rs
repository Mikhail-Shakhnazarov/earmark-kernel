use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use earmark_core::{
    Kind, ObjectId, ObjectRef, ProviderBudget, ProviderExposure, ProviderProfile, ProviderRequest,
    ProviderResponse, ProviderResponseContract, ProviderUsage, VersionId, VersionRef,
};
use earmark_exec::{
    default_provider_registry, provide_with_registry, provider_record_from_failure,
    resolve_provider_profile, ProviderAdapter, ProviderFailure, ProviderFailureKind, ProviderMode,
    ProviderRegistry,
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
        _transition_operation: &str,
    ) -> Result<ProviderResponse, ProviderFailure> {
        Ok(ProviderResponse {
            request_id: request.request_id,
            provider: "google".to_string(),
            model: "gemma".to_string(),
            status: "ok".to_string(),
            candidate_payload: r#"{"candidate":"ok"}"#.to_string(),
            metadata: BTreeMap::new(),
            advisory_warnings: vec![],
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

struct CustomEchoAdapter;
impl ProviderAdapter for CustomEchoAdapter {
    fn provider_key(&self) -> &'static str {
        "custom_echo"
    }

    fn provide(
        &self,
        request: ProviderRequest,
        _profile: &ProviderProfile,
        _transition_operation: &str,
    ) -> Result<ProviderResponse, ProviderFailure> {
        Ok(ProviderResponse {
            request_id: request.request_id,
            provider: "custom_echo".to_string(),
            model: "custom".to_string(),
            status: "ok".to_string(),
            candidate_payload: r#"{"candidate":"custom"}"#.to_string(),
            metadata: BTreeMap::new(),
            advisory_warnings: vec![],
            usage: None,
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
        _transition_operation: &str,
    ) -> Result<ProviderResponse, ProviderFailure> {
        Ok(ProviderResponse {
            request_id: request.request_id,
            provider: "local_http".to_string(),
            model: "broken".to_string(),
            status: "ok".to_string(),
            candidate_payload: "not-json".to_string(),
            metadata: BTreeMap::new(),
            advisory_warnings: vec![],
            usage: None,
            received_at: chrono::Utc::now(),
        })
    }
}

struct FlakyAdapter {
    fail_count: Arc<AtomicUsize>,
}
impl ProviderAdapter for FlakyAdapter {
    fn provider_key(&self) -> &'static str {
        "flaky"
    }
    fn provide(
        &self,
        request: ProviderRequest,
        _profile: &ProviderProfile,
        _transition_operation: &str,
    ) -> Result<ProviderResponse, ProviderFailure> {
        let n = self.fail_count.fetch_add(1, Ordering::SeqCst);
        if n < 2 {
            return Err(ProviderFailure::new(
                ProviderFailureKind::ProviderUnavailable,
                "transient",
            ));
        }
        Ok(ProviderResponse {
            request_id: request.request_id,
            provider: "flaky".to_string(),
            model: "ok".to_string(),
            status: "ok".to_string(),
            candidate_payload: r#"{"candidate":"ok"}"#.to_string(),
            metadata: BTreeMap::new(),
            advisory_warnings: vec![],
            usage: None,
            received_at: chrono::Utc::now(),
        })
    }
}

struct AlwaysFailAdapter;
impl ProviderAdapter for AlwaysFailAdapter {
    fn provider_key(&self) -> &'static str {
        "always_fail"
    }
    fn provide(
        &self,
        _request: ProviderRequest,
        _profile: &ProviderProfile,
        _transition_operation: &str,
    ) -> Result<ProviderResponse, ProviderFailure> {
        Err(ProviderFailure::new(
            ProviderFailureKind::ProviderUnavailable,
            "down",
        ))
    }
}

struct BudgetFailAdapter {
    calls: Arc<AtomicUsize>,
}

struct ThrottleThenSucceedAdapter {
    calls: Arc<AtomicUsize>,
}
impl ProviderAdapter for ThrottleThenSucceedAdapter {
    fn provider_key(&self) -> &'static str {
        "throttle_then_succeed"
    }
    fn provide(
        &self,
        request: ProviderRequest,
        _profile: &ProviderProfile,
        _transition_operation: &str,
    ) -> Result<ProviderResponse, ProviderFailure> {
        let n = self.calls.fetch_add(1, Ordering::SeqCst);
        if n < 2 {
            return Err(ProviderFailure::new(
                ProviderFailureKind::RateLimited,
                "rate limited",
            ));
        }
        Ok(ProviderResponse {
            request_id: request.request_id,
            provider: "throttle_then_succeed".to_string(),
            model: "ok".to_string(),
            status: "ok".to_string(),
            candidate_payload: r#"{"candidate":"ok"}"#.to_string(),
            metadata: BTreeMap::new(),
            advisory_warnings: vec![],
            usage: None,
            received_at: chrono::Utc::now(),
        })
    }
}
impl ProviderAdapter for BudgetFailAdapter {
    fn provider_key(&self) -> &'static str {
        "budget_fail"
    }
    fn provide(
        &self,
        _request: ProviderRequest,
        _profile: &ProviderProfile,
        _transition_operation: &str,
    ) -> Result<ProviderResponse, ProviderFailure> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Err(ProviderFailure::new(
            ProviderFailureKind::BudgetExceeded,
            "budget exceeded",
        ))
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
            allow_work_surface_only: false,
            allow_export_requests: false,
        },
        response_contract: ProviderResponseContract {
            format: "json".to_string(),
            must_return_candidate_only: true,
            must_include_lineage: true,
        },
        http: None,
    }
}

fn request() -> ProviderRequest {
    request_with_provider_profile(VersionRef::new(ObjectId::new(), VersionId::new()))
}

fn request_with_provider_profile(provider_profile: VersionRef) -> ProviderRequest {
    ProviderRequest {
        request_id: "req_1".to_string(),
        run_id: "run_1".to_string(),
        work_packet: ObjectRef::new(ObjectId::new(), VersionId::new(), Kind::WorkPacket, None),
        provider_profile,
        instruction_text: "Do the thing".to_string(),
        context_text: None,
        input_text: "Do the thing".to_string(),
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
        body: earmark_core::MarkdownBody::new("body".to_string()),
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
    let result = provide_with_registry(
        &registry,
        &profile("google", "gemma"),
        request(),
        "transform",
    );
    let err = result.err().unwrap();
    assert_eq!(err.kind, ProviderFailureKind::AdapterNotRegistered);
}

#[test]
fn default_registry_can_be_extended_with_custom_provider() {
    let mut registry = default_provider_registry();
    assert!(registry.get("mock").is_some());

    #[cfg(feature = "gemini")]
    assert!(registry.get("google_gemini").is_some());

    registry.register(Arc::new(CustomEchoAdapter));
    let outcome = provide_with_registry(
        &registry,
        &profile("custom_echo", "custom"),
        request(),
        "transform",
    );
    assert!(outcome.is_ok());
}

#[test]
fn malformed_response_handling() {
    let mut registry = ProviderRegistry::default();
    registry.register(Arc::new(MalformedAdapter));
    let result = provide_with_registry(
        &registry,
        &profile("local_http", "broken"),
        request(),
        "transform",
    );
    let err = result.err().unwrap();
    assert_eq!(err.kind, ProviderFailureKind::MalformedResponse);
}

#[test]
fn dispatch_record_creation() {
    let mut registry = ProviderRegistry::default();
    registry.register(Arc::new(GoodAdapter));
    let req = request();
    let prof = profile("google", "gemma");
    let outcome = provide_with_registry(&registry, &prof, req.clone(), "transform").unwrap();
    assert_eq!(outcome.record.provider, "google");
    assert_eq!(outcome.record.model, "gemma");

    let failure = ProviderFailure::new(ProviderFailureKind::Timeout, "timed out");
    let record = provider_record_from_failure(&req, &prof, &failure);
    assert!(record.message.unwrap().contains("timed out"));
}

#[test]
fn retry_succeeds_after_transient_failures() {
    let mut registry = ProviderRegistry::default();
    let counter = Arc::new(AtomicUsize::new(0));
    registry.register(Arc::new(FlakyAdapter {
        fail_count: counter.clone(),
    }));
    let outcome = provide_with_registry(&registry, &profile("flaky", "ok"), request(), "transform");
    assert!(outcome.is_ok());
    assert!(counter.load(Ordering::SeqCst) >= 3);
}

#[test]
fn circuit_opens_after_repeated_failures() {
    let mut registry = ProviderRegistry::default();
    registry.register(Arc::new(AlwaysFailAdapter));
    let prof = profile("always_fail", "down");
    let provider_profile = VersionRef::new(ObjectId::new(), VersionId::new());
    for _ in 0..5 {
        let _ = provide_with_registry(
            &registry,
            &prof,
            request_with_provider_profile(provider_profile.clone()),
            "transform",
        );
    }
    let err = provide_with_registry(
        &registry,
        &prof,
        request_with_provider_profile(provider_profile),
        "transform",
    )
    .unwrap_err();
    assert!(err.message.contains("circuit open"));
}

#[test]
fn retry_succeeds_after_rate_limit_failures() {
    let mut registry = ProviderRegistry::default();
    let calls = Arc::new(AtomicUsize::new(0));
    registry.register(Arc::new(ThrottleThenSucceedAdapter {
        calls: calls.clone(),
    }));
    let outcome = provide_with_registry(
        &registry,
        &profile("throttle_then_succeed", "ok"),
        request(),
        "transform",
    );
    assert!(outcome.is_ok());
    assert!(calls.load(Ordering::SeqCst) >= 3);
}

#[test]
fn budget_exceeded_is_not_retried() {
    let mut registry = ProviderRegistry::default();
    let calls = Arc::new(AtomicUsize::new(0));
    registry.register(Arc::new(BudgetFailAdapter {
        calls: calls.clone(),
    }));
    let prof = profile("budget_fail", "budget");
    let err = provide_with_registry(&registry, &prof, request(), "transform").unwrap_err();
    assert_eq!(err.kind, ProviderFailureKind::BudgetExceeded);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn circuit_isolated_by_provider_profile_identity() {
    let mut registry = ProviderRegistry::default();
    registry.register(Arc::new(AlwaysFailAdapter));
    let prof = profile("always_fail", "down");

    let first_profile = VersionRef::new(ObjectId::new(), VersionId::new());
    for _ in 0..5 {
        let _ = provide_with_registry(
            &registry,
            &prof,
            request_with_provider_profile(first_profile.clone()),
            "transform",
        );
    }
    let first_err = provide_with_registry(
        &registry,
        &prof,
        request_with_provider_profile(first_profile),
        "transform",
    )
    .unwrap_err();
    assert!(first_err.message.contains("circuit open"));

    let second_profile = VersionRef::new(ObjectId::new(), VersionId::new());
    let second_err = provide_with_registry(
        &registry,
        &prof,
        request_with_provider_profile(second_profile),
        "transform",
    )
    .unwrap_err();
    assert!(!second_err.message.contains("circuit open"));
}
