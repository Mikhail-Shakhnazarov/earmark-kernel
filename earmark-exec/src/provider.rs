use crate::error::{ProviderFailure, ProviderFailureKind};
use crate::helpers::{estimate_tokens_approx, uuid_like};
use chrono::Utc;
use earmark_core::{
    InstructionPayload, ProviderProfile, ProviderRecord, ProviderRequest, ProviderResponse,
    ProviderResponseContract, ProviderResponseFormat, ProviderResponseStatus, ScalarValue,
    VersionRef,
};
use std::collections::{BTreeMap, HashMap};
use std::env;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderCapabilityStatus {
    Available,
    CompileDisabled,
    MissingConfiguration,
    RuntimeUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ProviderCapability {
    pub provider: String,
    pub status: ProviderCapabilityStatus,
    pub feature: Option<String>,
    pub required_env: Vec<String>,
    pub missing_env: Vec<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderMode {
    LocalExecution,
    Delegated(VersionRef),
}

pub fn resolve_provider_profile(
    operation_provider_profile: Option<&VersionRef>,
    instruction: Option<&InstructionPayload>,
    system_definition_default: Option<&VersionRef>,
) -> ProviderMode {
    if let Some(reference) = operation_provider_profile {
        return ProviderMode::Delegated(reference.clone());
    }
    if let Some(reference) = instruction.and_then(|i| i.provider_profile.as_ref()) {
        return ProviderMode::Delegated(reference.clone());
    }
    if let Some(reference) = system_definition_default {
        return ProviderMode::Delegated(reference.clone());
    }
    ProviderMode::LocalExecution
}

pub trait ProviderAdapter: Send + Sync {
    fn provider_key(&self) -> &'static str;
    fn provide(
        &self,
        request: ProviderRequest,
        profile: &ProviderProfile,
        transition_operation: &str,
    ) -> Result<ProviderResponse, ProviderFailure>;

    fn capability(&self) -> ProviderCapability {
        ProviderCapability {
            provider: self.provider_key().to_string(),
            status: ProviderCapabilityStatus::Available,
            feature: None,
            required_env: vec![],
            missing_env: vec![],
            message: None,
        }
    }
}

pub trait ProviderService: Send + Sync {
    fn provide(
        &self,
        profile: &ProviderProfile,
        request: ProviderRequest,
        transition_operation: &str,
    ) -> Result<ProviderExecutionOutcome, ProviderFailure>;
}

#[derive(Default)]
pub struct ProviderRegistry {
    pub adapters: HashMap<String, Arc<dyn ProviderAdapter>>,
}

impl ProviderService for ProviderRegistry {
    fn provide(
        &self,
        profile: &ProviderProfile,
        request: ProviderRequest,
        transition_operation: &str,
    ) -> Result<ProviderExecutionOutcome, ProviderFailure> {
        provide_with_registry(self, profile, request, transition_operation)
    }
}

pub trait RetrySleeper: Send + Sync {
    fn sleep(&self, duration: Duration);
}

#[derive(Default)]
pub struct ThreadSleepSleeper;

impl RetrySleeper for ThreadSleepSleeper {
    fn sleep(&self, duration: Duration) {
        std::thread::sleep(duration);
    }
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self {
            adapters: HashMap::new(),
        }
    }

    pub fn with_defaults() -> Self {
        let mut registry = Self::new();
        registry.register_default_adapters();
        registry
    }

    pub fn register_default_adapters(&mut self) {
        self.register(Arc::new(MockAdapter));

        #[cfg(feature = "http-provider")]
        self.register(Arc::new(crate::HttpGenerationAdapter));
    }

    pub fn capabilities(&self) -> Vec<ProviderCapability> {
        let mut capabilities = self
            .adapters
            .values()
            .map(|adapter| adapter.capability())
            .collect::<Vec<_>>();
        capabilities.sort_by(|a, b| a.provider.cmp(&b.provider));
        capabilities
    }

    pub fn register(&mut self, adapter: Arc<dyn ProviderAdapter>) {
        self.adapters
            .insert(adapter.provider_key().to_string(), adapter);
    }

    pub fn get(&self, provider: &str) -> Option<Arc<dyn ProviderAdapter>> {
        self.adapters.get(provider).cloned()
    }
}

pub fn default_provider_registry() -> ProviderRegistry {
    ProviderRegistry::with_defaults()
}

pub fn compiled_provider_capabilities() -> Vec<ProviderCapability> {
    let mut capabilities = ProviderRegistry::with_defaults().capabilities();

    #[cfg(not(feature = "http-provider"))]
    capabilities.push(ProviderCapability {
        provider: "http_generation".to_string(),
        status: ProviderCapabilityStatus::CompileDisabled,
        feature: Some("http-provider".to_string()),
        required_env: vec![],
        missing_env: vec![],
        message: Some("provider requires the http-provider cargo feature".to_string()),
    });

    capabilities.sort_by(|a, b| {
        // Foreground http_generation
        if a.provider == "http_generation" {
            return std::cmp::Ordering::Less;
        }
        if b.provider == "http_generation" {
            return std::cmp::Ordering::Greater;
        }
        a.provider.cmp(&b.provider)
    });
    capabilities
}

pub struct MockAdapter;

pub fn provider_response_is_synthetic(response: &ProviderResponse) -> bool {
    match response.metadata.get("synthetic") {
        Some(ScalarValue::Bool(v)) => *v,
        _ => response.provider == "mock",
    }
}

pub fn provider_metadata_synthetic_source(
    metadata: &BTreeMap<String, ScalarValue>,
) -> Option<String> {
    match metadata.get("synthetic_source") {
        Some(ScalarValue::String(v)) => Some(v.clone()),
        _ => None,
    }
}

impl ProviderAdapter for MockAdapter {
    fn provider_key(&self) -> &'static str {
        "mock"
    }

    fn provide(
        &self,
        request: ProviderRequest,
        profile: &ProviderProfile,
        _transition_operation: &str,
    ) -> Result<ProviderResponse, ProviderFailure> {
        if profile.model == "fail" {
            return Err(ProviderFailure::new(
                ProviderFailureKind::ProviderUnavailable,
                "Intentional failure for demo purposes.",
            ));
        }
        let mut metadata = BTreeMap::new();
        metadata.insert("synthetic".to_string(), ScalarValue::Bool(true));
        metadata.insert(
            "synthetic_source".to_string(),
            ScalarValue::String("mock_provider".to_string()),
        );
        metadata.insert(
            "synthetic_kind".to_string(),
            ScalarValue::String("fixture_response".to_string()),
        );
        metadata.insert("production_eligible".to_string(), ScalarValue::Bool(false));
        metadata.insert(
            "warning".to_string(),
            ScalarValue::String(
                "Synthetic mock provider output; do not treat as model-derived evidence."
                    .to_string(),
            ),
        );
        Ok(ProviderResponse {
            request_id: request.request_id,
            provider: "mock".to_string(),
            model: "echo".to_string(),
            status: ProviderResponseStatus::Completed,
            candidate_payload: "[SYNTHETIC MOCK OUTPUT] Fixture response for extraction/synthesis tests. Federated graphs provide agile ownership but introduce heterogeneity costs.".to_string(),
            metadata,
            advisory_warnings: vec![],
            usage: None,
            received_at: Utc::now(),
        })
    }
}

pub struct ProviderPolicyDecision {
    pub advisory_warnings: Vec<String>,
}

pub(crate) fn validate_provider_invocation(
    profile: &ProviderProfile,
    transition_operation: &str,
    request: &ProviderRequest,
) -> Result<ProviderPolicyDecision, ProviderFailure> {
    let mut warnings = Vec::new();

    // 1. Allowed operations
    if profile.allowed_operations.is_empty() {
        return Err(ProviderFailure::new(
            ProviderFailureKind::ForbiddenOperation,
            format!(
                "Provider profile '{}' does not allow any operations (allowed_operations is empty).",
                profile.name
            ),
        ));
    }
    if !profile
        .allowed_operations
        .contains(&transition_operation.to_string())
    {
        return Err(ProviderFailure::new(
            ProviderFailureKind::ForbiddenOperation,
            format!(
                "Provider profile '{}' does not allow operation '{}'. Allowed: {:?}",
                profile.name, transition_operation, profile.allowed_operations
            ),
        ));
    }

    // 2. Exposure policies
    if profile.exposure.allow_work_surface_only && request.work_surface_manifest.is_none() {
        return Err(ProviderFailure::new(
            ProviderFailureKind::ForbiddenOperation,
            format!(
                "Provider profile '{}' requires work_surface_only, but no work surface manifest was provided.",
                profile.name
            ),
        ));
    }

    if !profile.exposure.allow_prose_objects {
        warnings.push("Advisory: allow_prose_objects is false, but prose payload filtering is not yet enforced in this path.".to_string());
    }
    if !profile.exposure.allow_structured_declarations {
        warnings.push("Advisory: allow_structured_declarations is false, but declaration filtering is not yet enforced in this path.".to_string());
    }
    if !profile.exposure.allow_export_requests && transition_operation == "export" {
        return Err(ProviderFailure::new(
            ProviderFailureKind::ForbiddenOperation,
            format!(
                "Provider profile '{}' disallows export requests.",
                profile.name
            ),
        ));
    }

    // 3. Budget/timeout posture
    if let Some(max_latency_ms) = profile.budget.max_latency_ms {
        warnings.push(format!(
            "Advisory: max_latency_ms is configured to {} ms; timeout is adapter-level only and does not provide runtime cancellation guarantees.",
            max_latency_ms
        ));
    }

    // 4. Response Contract (Advisory)
    if profile.response_contract.must_include_lineage {
        warnings.push("Advisory: must_include_lineage is true, but lineage capture is not yet enforced by all adapters.".to_string());
    }
    if !profile.response_contract.must_return_candidate_only {
        warnings.push("Advisory: must_return_candidate_only is false, but full message capture is not yet supported in this path.".to_string());
    }

    Ok(ProviderPolicyDecision {
        advisory_warnings: warnings,
    })
}

#[derive(Debug, Clone)]
pub struct ProviderExecutionOutcome {
    pub response: Option<ProviderResponse>,
    pub record: ProviderRecord,
}

#[derive(Default, Clone, Copy)]
pub struct CircuitState {
    pub consecutive_failures: u32,
    pub open_until_epoch_ms: i64,
}

pub(crate) fn provider_circuit_registry() -> &'static Mutex<HashMap<String, CircuitState>> {
    static REGISTRY: OnceLock<Mutex<HashMap<String, CircuitState>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

#[cfg(test)]
pub(crate) fn reset_provider_circuit_registry_for_tests() {
    if let Ok(mut lock) = provider_circuit_registry().lock() {
        lock.clear();
    }
}

pub(crate) fn resolved_endpoint_identity(profile: &ProviderProfile) -> String {
    match profile.endpoint_env.as_deref() {
        Some(env_name) => match env::var(env_name) {
            Ok(value) if !value.trim().is_empty() => value,
            Ok(_) => format!("<empty:{}>", env_name),
            Err(_) => format!("<unset:{}>", env_name),
        },
        None => "<default>".to_string(),
    }
}

pub(crate) fn provider_circuit_key(request: &ProviderRequest, profile: &ProviderProfile) -> String {
    format!(
        "{}|{}|{}|{}|{}",
        request.provider_profile.id.as_str(),
        request.provider_profile.version_id.as_str(),
        profile.provider,
        profile.model,
        resolved_endpoint_identity(profile)
    )
}

pub(crate) fn provider_now_ms() -> i64 {
    Utc::now().timestamp_millis()
}

pub(crate) fn is_retryable_failure(failure: &ProviderFailure) -> bool {
    matches!(
        failure.kind,
        ProviderFailureKind::ProviderUnavailable
            | ProviderFailureKind::RateLimited
            | ProviderFailureKind::Timeout
    )
}

pub fn provide_with_registry(
    registry: &ProviderRegistry,
    profile: &ProviderProfile,
    request: ProviderRequest,
    transition_operation: &str,
) -> Result<ProviderExecutionOutcome, ProviderFailure> {
    provide_with_registry_and_sleeper(
        registry,
        profile,
        request,
        transition_operation,
        &ThreadSleepSleeper,
    )
}

pub fn provide_with_registry_and_sleeper(
    registry: &ProviderRegistry,
    profile: &ProviderProfile,
    request: ProviderRequest,
    transition_operation: &str,
    sleeper: &dyn RetrySleeper,
) -> Result<ProviderExecutionOutcome, ProviderFailure> {
    let budget_meter_text = format!(
        "{}\n{}\n{}",
        request.instruction_text,
        request.context_text.clone().unwrap_or_default(),
        request.input_text
    );
    if let Some(max_input_tokens) = profile.budget.max_input_tokens {
        let estimated_input_tokens = estimate_tokens_approx(&budget_meter_text);
        if estimated_input_tokens > max_input_tokens {
            return Err(ProviderFailure::new(
                ProviderFailureKind::BudgetExceeded,
                format!(
                    "Estimated input tokens {} exceed max_input_tokens {}",
                    estimated_input_tokens, max_input_tokens
                ),
            ));
        }
    }

    let circuit_key = provider_circuit_key(&request, profile);
    {
        let lock = provider_circuit_registry().lock().map_err(|_| {
            ProviderFailure::new(
                ProviderFailureKind::ProviderUnavailable,
                "provider circuit state lock poisoned",
            )
        })?;
        if let Some(state) = lock.get(&circuit_key) {
            if state.open_until_epoch_ms > provider_now_ms() {
                return Err(ProviderFailure::new(
                    ProviderFailureKind::ProviderUnavailable,
                    "provider circuit open",
                ));
            }
        }
    }

    let adapter = registry.get(&profile.provider).ok_or_else(|| {
        ProviderFailure::new(
            ProviderFailureKind::AdapterNotRegistered,
            format!("no adapter registered for provider {}", profile.provider),
        )
    })?;

    // Policy Gate
    let policy_decision = validate_provider_invocation(profile, transition_operation, &request)?;

    let mut effective_profile = profile.clone();
    if effective_profile.budget.max_latency_ms.is_none() {
        effective_profile.budget.max_latency_ms = Some(30_000);
    }

    let backoff_schedule = [Duration::from_millis(250), Duration::from_secs(1)];
    let mut last_error: Option<ProviderFailure> = None;
    let mut response: Option<ProviderResponse> = None;
    for attempt in 0..3 {
        match adapter.provide(request.clone(), &effective_profile, transition_operation) {
            Ok(resp) => {
                match validate_provider_response(&resp, &effective_profile.response_contract) {
                    Ok(()) => {
                        if let Some(max_output_tokens) = effective_profile.budget.max_output_tokens
                        {
                            let estimated_output_tokens =
                                estimate_tokens_approx(&resp.candidate_payload);
                            if estimated_output_tokens > max_output_tokens {
                                last_error = Some(ProviderFailure::new(
                                    ProviderFailureKind::BudgetExceeded,
                                    format!(
                                        "Estimated output tokens {} exceed max_output_tokens {}",
                                        estimated_output_tokens, max_output_tokens
                                    ),
                                ));
                                break;
                            }
                        }

                        if let Some(max_cost_usd) = effective_profile.budget.max_cost_usd {
                            if let Some(usage) = &resp.usage {
                                if let Some(estimated_cost_usd) = usage.estimated_cost_usd {
                                    if estimated_cost_usd > max_cost_usd {
                                        last_error = Some(ProviderFailure::new(
                                            ProviderFailureKind::BudgetExceeded,
                                            format!(
                                                "Estimated cost ${:.4} exceeds max_cost_usd ${:.4}",
                                                estimated_cost_usd, max_cost_usd
                                            ),
                                        ));
                                        break;
                                    }
                                }
                            }
                        }

                        response = Some(resp);
                        break;
                    }
                    Err(err) => {
                        last_error = Some(err.clone());
                        if !is_retryable_failure(&err) || attempt >= 2 {
                            break;
                        }
                    }
                }
            }
            Err(err) => {
                last_error = Some(err.clone());
                if !is_retryable_failure(&err) || attempt >= 2 {
                    break;
                }
            }
        }
        if let Some(backoff) = backoff_schedule.get(attempt) {
            sleeper.sleep(*backoff);
        }
    }
    let response = match response {
        Some(r) => r,
        None => {
            let err = last_error.unwrap_or_else(|| {
                ProviderFailure::new(
                    ProviderFailureKind::ProviderUnavailable,
                    "unknown provider failure",
                )
            });
            let mut lock = provider_circuit_registry().lock().map_err(|_| {
                ProviderFailure::new(
                    ProviderFailureKind::ProviderUnavailable,
                    "provider circuit state lock poisoned",
                )
            })?;
            let state = lock.entry(circuit_key).or_default();
            state.consecutive_failures += 1;
            if state.consecutive_failures >= 5 {
                state.open_until_epoch_ms = provider_now_ms() + 60_000;
            }
            return Err(err);
        }
    };

    // Merge advisory warnings
    let mut final_warnings = policy_decision.advisory_warnings;
    final_warnings.extend(response.advisory_warnings.clone());
    if effective_profile.budget.max_latency_ms.is_some()
        && !response.metadata.contains_key("latency_ms")
    {
        final_warnings.push(
            "Advisory: provider response did not report measured latency_ms; timeout compliance cannot be confirmed from runtime metadata.".to_string(),
        );
    }
    if effective_profile.budget.max_cost_usd.is_some()
        && response
            .usage
            .as_ref()
            .and_then(|usage| usage.estimated_cost_usd)
            .is_none()
    {
        final_warnings.push(
            "Advisory: max_cost_usd is configured but provider response did not report usage.estimated_cost_usd; cost budget cannot be enforced for this invocation.".to_string(),
        );
    }

    {
        let mut lock = provider_circuit_registry().lock().map_err(|_| {
            ProviderFailure::new(
                ProviderFailureKind::ProviderUnavailable,
                "provider circuit state lock poisoned",
            )
        })?;
        lock.remove(&circuit_key);
    }
    let mut record = provider_record_from_response(&request, profile, &response, None);
    record.advisory_warnings = final_warnings;
    Ok(ProviderExecutionOutcome {
        response: Some(response),
        record,
    })
}

pub(crate) fn validate_provider_response(
    response: &ProviderResponse,
    contract: &ProviderResponseContract,
) -> Result<(), ProviderFailure> {
    if response.candidate_payload.trim().is_empty() {
        return Err(ProviderFailure::new(
            ProviderFailureKind::MalformedResponse,
            "candidate payload was empty",
        ));
    }

    if contract.format == ProviderResponseFormat::Json {
        serde_json::from_str::<serde_json::Value>(&response.candidate_payload).map_err(
            |error| {
                ProviderFailure::new(
                    ProviderFailureKind::MalformedResponse,
                    format!("candidate payload was not valid json: {}", error),
                )
            },
        )?;
    }

    Ok(())
}

pub fn provider_record_from_response(
    request: &ProviderRequest,
    profile: &ProviderProfile,
    response: &ProviderResponse,
    message: Option<String>,
) -> ProviderRecord {
    let mut metadata = response.metadata.clone();
    if profile.provider == "mock" {
        metadata.insert("synthetic".to_string(), ScalarValue::Bool(true));
        metadata.insert(
            "synthetic_source".to_string(),
            ScalarValue::String("mock_provider".to_string()),
        );
        metadata.insert("production_eligible".to_string(), ScalarValue::Bool(false));
    }

    ProviderRecord {
        record_id: format!("prec_{}", uuid_like()),
        request_id: request.request_id.clone(),
        run_id: request.run_id.clone(),
        work_packet: request.work_packet.clone(),
        provider_profile: request.provider_profile.clone(),
        provider: profile.provider.clone(),
        model: profile.model.clone(),
        status: response.status.clone(),
        metadata,
        advisory_warnings: response.advisory_warnings.clone(),
        usage: response.usage.clone(),
        message,
        recorded_at: Utc::now(),
    }
}

pub fn provider_record_from_failure(
    request: &ProviderRequest,
    profile: &ProviderProfile,
    failure: &ProviderFailure,
) -> ProviderRecord {
    let mut metadata = BTreeMap::new();
    if profile.provider == "mock" {
        metadata.insert("synthetic".to_string(), ScalarValue::Bool(true));
        metadata.insert(
            "synthetic_source".to_string(),
            ScalarValue::String("mock_provider".to_string()),
        );
        metadata.insert("production_eligible".to_string(), ScalarValue::Bool(false));
    }
    ProviderRecord {
        record_id: format!("prec_{}", uuid_like()),
        request_id: request.request_id.clone(),
        run_id: request.run_id.clone(),
        work_packet: request.work_packet.clone(),
        provider_profile: request.provider_profile.clone(),
        provider: profile.provider.clone(),
        model: profile.model.clone(),
        status: ProviderResponseStatus::Failed,
        metadata,
        advisory_warnings: vec![],
        usage: None,
        message: Some(failure.message.clone()),
        recorded_at: Utc::now(),
    }
}
