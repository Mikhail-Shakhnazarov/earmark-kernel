# Surface Inventory

© 2026 Mikhail Shakhnazarov

## Status

Current as of May 2026, after the native self-hosting migration (Engram removal, orchestration promotion, and semantic hardening).

## Classification labels

| Label | Meaning |
|---|---|
| `core` | Required for the canonical spine. |
| `supporting` | Not central, but needed to operate or inspect the core. |
| `stable` | Promoted to package-level stability. |
| `beta` | Functional but not yet fully hardened for package guarantees. |
| `internal-tooling` | Development tooling, not product surface. |
| `test-fixture` | Required by tests only. |

## Crates

| Surface | Classification | Rationale | Status |
|---|---|---|---|
| `earmark-core` | core | Shared domain model: IDs, objects, declarations, run records, standing, provider records, and serialization primitives. | Hardened |
| `earmark-store` | core | Git-backed canonical durable storage for object versions, payloads, heads, workspace layout, and write locking. | Hardened |
| `earmark-index` | core | Rebuildable derived projection for query, relations, standing, active systems, and undo visibility. Transactional rebuild with dirty-marker lifecycle. | Hardened |
| `earmark-declarations` | core | Declaration parsing, loading, and validation. Rejects invalid schemas at registration time. | Hardened |
| `earmark-connected-context` | core | Bounded work-surface compilation from canonical objects and declared context templates. | Hardened |
| `earmark-exec` | core | Staged execution runtime: workflow execution, assignments, change sets, handoffs, failures, provider records, and relation persistence. Canonical relation authorization enforced. | Hardened |
| `earmark-governance` | core | Review, standing, export, and governance event logic. | Hardened |
| `earmark-cli` | core | Primary operator interface over the canonical spine. Stability metadata exposed via `em commands --json`. | Hardened |
| `earmark-runtime-tools` | supporting | Runtime helper utilities; supports execution and integration. | Hardened |
| `earmark` | supporting | In-process Rust workspace API (`EarmarkWorkspace`). CLI-backed compatibility wrapper (`CliBackedWorkspace`) retained. | Hardened |

## CLI command families

| Command family | Classification | Stability | Notes |
|---|---|---|---|
| `init` | core | stable | Explicit workspace creation. |
| `deposit` | core | stable | Object creation through canonical write path. |
| `query` | core | stable | Index-backed object/relation lookup. |
| `declare` | core | stable | Declaration registration and scaffolding. |
| `system` | core | stable | System definition lifecycle. |
| `workflow` | core | stable | Staged execution. |
| `run` | core | stable | Run inspection and lifecycle. |
| `handoff` | core | stable | Handoff inspection. |
| `doctor` | core | stable | Workspace diagnostics. |
| `failure` | core | stable | Failure record inspection. |
| `report` | core | stable | Human-readable report generation. |
| `context` | core | stable | Bounded context compilation. |
| `relation` | core | stable | Relation management and authorization. |
| `standing-request` | core | stable | Standing lifecycle management. |
| `orchestration` | core | experimental | Native orchestration ledger for task, git snapshot, gate, and lifecycle management. |
| `commands` | core | stable | Command catalog with stability metadata. |
| `status` | core | stable | Workspace status. |
| `provider` | supporting | beta | Provider capabilities and integration. |
| `completions` | supporting | beta | Shell integration convenience. |
| `undo` | supporting | beta | Compensating visibility model. |
| `audit` | supporting | beta | Aggregated diagnostic views. |

## Scripts

| Surface | Classification | Notes |
|---|---|---|
| `scripts/dispatch-opencode.sh` | internal-tooling | Native OpenCode executor bridge. Primary dispatch mechanism. |
| `scripts/dispatch-native.sh` | internal-tooling | Local orchestration helper for dogfooding. |
| `scripts/smoke-opencode-big-pickle.sh` | internal-tooling | Local smoke test for executor configuration. |

**Note**: `scripts/dispatch-opencode-engram.sh` has been deleted. No scripts depend on Engram.

## Examples

| Surface | Classification | Notes |
|---|---|---|
| `examples/research-synthesis` | core | Primary tutorial/demo for the declared workflow model. |
| `examples/knowledge-briefing` | supporting | Additional domain example for staged knowledge work. |
| `examples/kommunale-waermeplanung` | supporting | Domain-specific example demonstrating non-English use. |
| `examples/earmark-dev-orchestration` | core | Native self-hosting orchestration system with hardened class declarations. |
| `examples/earmark_dev_trials` | internal-tooling | Development trial material. |

## Documentation

| Surface | Classification | Notes |
|---|---|---|
| `README.md` | core | Product entry point. Reflects native-only architecture. |
| `docs/tutorials/quickstart.md` | core | First operator path. |
| `docs/tutorials/practical-guide.md` | core | Product explanation. |
| `docs/concepts/staged-execution.md` | core | Execution spine. |
| `docs/concepts/context-compilation.md` | core | Bounded context. |
| `docs/concepts/standing.md` | core | Standing lifecycle. |
| `docs/concepts/relation-authorization.md` | core | Declared relation authorization. |
| `docs/concepts/native-orchestration.md` | core | Native orchestration ledger (stability is experimental). |
| `docs/reference/cli.md` | core | Command reference. |
| `docs/reference/runtime-integration-guide.md` | supporting | External integration guide. |
| `docs/reference/provider-extension.md` | supporting | Provider extension surface. |
| `docs/declarations/README.md` | core | Declaration authoring reference. |
| `docs/internal/INTERNAL_ARCHITECTURE.md` | internal-tooling | Core architecture internals. |
| `docs/internal/authorization-gates.md` | internal-tooling | Actor-trust authorization design. |
| `docs/internal/orchestration/` | internal-tooling | Orchestrator runtime instructions. |

## Completed hardening

The following hardening stages from the original streamlining plan have been fully implemented and verified:

1. ✅ Canonical relation authorization (fail-closed on class-definition errors)
2. ✅ Transactional derived-index rebuild (dirty-marker lifecycle)
3. ✅ Explicit partial workflow status (`partial`)
4. ✅ Declaration/runtime contract alignment (multi-output transform rejection)
5. ✅ Explicit workspace initialization semantics
6. ✅ HTTP provider boundary hardening (URL/domain safety)
7. ✅ Edge-case and property test expansion
8. ✅ In-process facade replacement (`EarmarkWorkspace`)
9. ✅ CLI stability catalog (`em commands --json`)
10. ✅ Native orchestration ledger implemented for dogfooding; public stability remains experimental
11. ✅ Engram dependency removal (adapter, scripts, environment, documentation)

Historical records for these stages are preserved in `docs/internal/archive/`.
