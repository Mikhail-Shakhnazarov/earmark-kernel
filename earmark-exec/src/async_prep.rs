/// Async preparation seam for provider dispatch and execution.
///
/// This module documents the known blocking boundaries in the current synchronous
/// execution path and provides a recommended migration sequence toward async
/// provider dispatch. It is a forward-looking integration seam: the types and
/// functions here are intended to guide incremental async adoption without
/// requiring a full runtime migration now.
///
/// At present, all provider dispatch and workflow execution is synchronous. These
/// boundaries are catalogued so that future work can isolate each blocking call
/// site and introduce async dispatch incrementally, starting with the provider
/// adapter boundary (`ProviderService` with internal async bridge seam).
///
/// This module is public but production code does not call it. It exists for
/// documentation, planning, and seam testing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockingBoundaryKind {
    ExternalIo,
    StoreIndexIo,
    OrchestrationDependency,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockingBoundary {
    pub id: &'static str,
    pub location: &'static str,
    pub kind: BlockingBoundaryKind,
    pub future_async_candidate: bool,
    pub rationale: &'static str,
}

pub fn blocking_boundaries() -> Vec<BlockingBoundary> {
    vec![
        BlockingBoundary {
            id: "provider_dispatch",
            location: "earmark-exec/src/transition.rs",
            kind: BlockingBoundaryKind::ExternalIo,
            future_async_candidate: true,
            rationale: "Provider execution includes network-bound adapter calls and retry backoff sleeps.",
        },
        BlockingBoundary {
            id: "provider_http_client",
            location: "earmark-exec/src/http_generation.rs",
            kind: BlockingBoundaryKind::ExternalIo,
            future_async_candidate: true,
            rationale: "HTTP generation adapter uses reqwest blocking client and synchronous response decode.",
        },
        BlockingBoundary {
            id: "store_index_access",
            location: "earmark-exec/src/engine.rs + transition.rs",
            kind: BlockingBoundaryKind::StoreIndexIo,
            future_async_candidate: false,
            rationale: "Canonical store/index operations are pervasive and should remain sync until provider seam migration stabilizes.",
        },
        BlockingBoundary {
            id: "runtime_tool_workflow_wrapper",
            location: "earmark-runtime-tools/src/modules/workflow.rs",
            kind: BlockingBoundaryKind::OrchestrationDependency,
            future_async_candidate: false,
            rationale: "Runtime surface is a synchronous orchestration wrapper and should follow engine migration, not lead it.",
        },
        BlockingBoundary {
            id: "cli_entrypoint",
            location: "earmark-cli/src/app.rs",
            kind: BlockingBoundaryKind::OrchestrationDependency,
            future_async_candidate: false,
            rationale: "CLI remains sync by design in this phase; async runtime introduction is explicitly deferred.",
        },
    ]
}

pub fn recommended_async_migration_sequence() -> Vec<&'static str> {
    vec![
        "provider_service boundary (ProviderService + adapters)",
        "execution transition provider call path",
        "engine orchestration call path",
        "runtime-tools workflow wrappers",
        "CLI entrypoints only if justified",
    ]
}

pub fn explicitly_deferred_in_slice_05() -> Vec<&'static str> {
    vec![
        "async ExecutionEngine conversion",
        "async CLI conversion",
        "executor/runtime dependency introduction",
        "parallel transition scheduling",
    ]
}
