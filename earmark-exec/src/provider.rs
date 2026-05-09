use crate::error::{ProviderFailure, ProviderFailureKind};
use crate::helpers::uuid_like;
use chrono::Utc;
use earmark_core::{
    InstructionPayload, ProviderProfile, ProviderRecord, ProviderRequest, ProviderResponse,
    ProviderResponseContract, VersionRef,
};
use std::collections::{BTreeMap, HashMap};
use std::env;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

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
    ) -> Result<ProviderResponse, ProviderFailure>;
}

pub trait ProviderService: Send + Sync {
    fn provide(
        &self,
        profile: &ProviderProfile,
        request: ProviderRequest,
    ) -> Result<ProviderExecutionOutcome, ProviderFailure>;
}

pub trait AsyncProviderAdapter: Send + Sync {
    fn provider_key(&self) -> &'static str;
    fn provide_blocking_bridge(
        &self,
        request: ProviderRequest,
        profile: &ProviderProfile,
    ) -> Result<ProviderResponse, ProviderFailure>;
}

pub trait AsyncProviderService: Send + Sync {
    fn provide_blocking_bridge(
        &self,
        profile: &ProviderProfile,
        request: ProviderRequest,
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
    ) -> Result<ProviderExecutionOutcome, ProviderFailure> {
        provide_with_registry(self, profile, request)
    }
}

impl AsyncProviderService for ProviderRegistry {
    fn provide_blocking_bridge(
        &self,
        profile: &ProviderProfile,
        request: ProviderRequest,
    ) -> Result<ProviderExecutionOutcome, ProviderFailure> {
        provide_with_registry(self, profile, request)
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
        self.register(Arc::new(crate::GeminiAdapter::new(
            "gemini-1.5-pro".to_string(),
            "GOOGLE_API_KEY".to_string(),
        )));
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

pub struct MockAdapter;

impl ProviderAdapter for MockAdapter {
    fn provider_key(&self) -> &'static str {
        "mock"
    }

    fn provide(
        &self,
        request: ProviderRequest,
        profile: &ProviderProfile,
    ) -> Result<ProviderResponse, ProviderFailure> {
        if profile.model == "fail" {
            return Err(ProviderFailure::new(
                ProviderFailureKind::ProviderUnavailable,
                "Intentional failure for demo purposes.",
            ));
        }
        Ok(ProviderResponse {
            request_id: request.request_id,
            provider: "mock".to_string(),
            model: "echo".to_string(),
            status: "completed".to_string(),
            candidate_payload: "Mock response for extraction/synthesis. Federated graphs provide agile ownership but introduce heterogeneity costs.".to_string(),
            metadata: BTreeMap::new(),
            usage: None,
            received_at: Utc::now(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct ProviderExecutionOutcome {
    pub response: Option<ProviderResponse>,
    pub record: ProviderRecord,
}

#[derive(Default, Clone, Copy)]
pub(crate) struct CircuitState {
    pub consecutive_failures: u32,
    pub open_until_epoch_ms: i64,
}

pub(crate) fn provider_circuit_registry() -> &'static Mutex<HashMap<String, CircuitState>> {
    static REGISTRY: OnceLock<Mutex<HashMap<String, CircuitState>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
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
) -> Result<ProviderExecutionOutcome, ProviderFailure> {
    provide_with_registry_and_sleeper(registry, profile, request, &ThreadSleepSleeper)
}

pub fn provide_with_registry_and_sleeper(
    registry: &ProviderRegistry,
    profile: &ProviderProfile,
    request: ProviderRequest,
    sleeper: &dyn RetrySleeper,
) -> Result<ProviderExecutionOutcome, ProviderFailure> {
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

    let mut effective_profile = profile.clone();
    if effective_profile.budget.max_latency_ms.is_none() {
        effective_profile.budget.max_latency_ms = Some(30_000);
    }

    let backoff_schedule = [Duration::from_millis(250), Duration::from_secs(1)];
    let mut last_error: Option<ProviderFailure> = None;
    let mut response: Option<ProviderResponse> = None;
    for attempt in 0..3 {
        match adapter.provide(request.clone(), &effective_profile) {
            Ok(resp) => {
                match validate_provider_response(&resp, &effective_profile.response_contract) {
                    Ok(()) => {
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
    {
        let mut lock = provider_circuit_registry().lock().map_err(|_| {
            ProviderFailure::new(
                ProviderFailureKind::ProviderUnavailable,
                "provider circuit state lock poisoned",
            )
        })?;
        lock.remove(&circuit_key);
    }
    let record = provider_record_from_response(&request, profile, &response, None);
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

    if contract.format == "json" {
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
    ProviderRecord {
        record_id: format!("prec_{}", uuid_like()),
        request_id: request.request_id.clone(),
        run_id: request.run_id.clone(),
        work_packet: request.work_packet.clone(),
        provider_profile: request.provider_profile.clone(),
        provider: profile.provider.clone(),
        model: profile.model.clone(),
        status: response.status.clone(),
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
    ProviderRecord {
        record_id: format!("prec_{}", uuid_like()),
        request_id: request.request_id.clone(),
        run_id: request.run_id.clone(),
        work_packet: request.work_packet.clone(),
        provider_profile: request.provider_profile.clone(),
        provider: profile.provider.clone(),
        model: profile.model.clone(),
        status: format!("{:?}", failure.kind).to_lowercase(),
        usage: None,
        message: Some(failure.message.clone()),
        recorded_at: Utc::now(),
    }
}
