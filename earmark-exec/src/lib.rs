pub(crate) mod ir;
pub(crate) mod error;
pub(crate) mod provider;
pub(crate) mod engine;
pub(crate) mod validation;
pub(crate) mod helpers;
pub(crate) mod persistence;
pub(crate) mod resolution;
pub(crate) mod handoff;
pub(crate) mod state;
pub(crate) mod transition;
pub mod async_prep;

// Intended public surface
pub use engine::ExecutionEngine;
pub use error::{ExecError, ProviderFailure, ProviderFailureKind};
pub use ir::{WorkflowRunOutcome, WorkflowRunRequest};
pub use provider::{
    default_provider_registry, provide_with_registry, provider_record_from_failure, provider_record_from_response,
    AsyncProviderAdapter, AsyncProviderService, MockAdapter, ProviderAdapter, ProviderExecutionOutcome, ProviderMode,
    ProviderRegistry, ProviderService, RetrySleeper, ThreadSleepSleeper, resolve_provider_profile,
};

// Specialized adapters
pub mod gemini;
pub use gemini::GeminiAdapter;

#[cfg(test)]
mod tests;
