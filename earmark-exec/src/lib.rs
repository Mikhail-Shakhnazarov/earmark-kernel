pub mod async_prep;
pub mod engine;
pub mod error;
pub mod handoff;
pub mod helpers;
pub mod ir;
pub mod governance_ops;
pub(crate) mod persistence;
pub mod persistence_helpers;
pub mod provider;
pub mod relation;
pub mod relation_logic;
pub(crate) mod resolution;
pub mod state;
pub(crate) mod transition;
pub mod validation;

// Intended public surface
pub use engine::ExecutionEngine;
pub use error::{ExecError, ProviderFailure, ProviderFailureKind};
pub use ir::{ExecutionIr, ExecutionTransition, WorkflowRunOutcome, WorkflowRunRequest};
pub use provider::{
    compiled_provider_capabilities, default_provider_registry, provide_with_registry,
    provider_metadata_synthetic_source, provider_record_from_failure,
    provider_record_from_response, provider_response_is_synthetic, resolve_provider_profile,
    AsyncProviderAdapter, AsyncProviderService, MockAdapter, ProviderAdapter, ProviderCapability,
    ProviderCapabilityStatus, ProviderExecutionOutcome, ProviderMode, ProviderRegistry,
    ProviderService, RetrySleeper, ThreadSleepSleeper,
};
pub use relation::persist_relation_canonical;
pub use relation_logic::{
    RelationAuthorizationDecision, RelationAuthorizationReason, RelationAuthorizationResolver,
    RelationEndpointFacts,
};

// Specialized adapters
pub mod gemini;
pub use gemini::GeminiAdapter;

#[cfg(test)]
mod tests;
