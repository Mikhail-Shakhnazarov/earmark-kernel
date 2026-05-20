# Surface Inventory

© 2026 Mikhail Shakhnazarov

## Status

Initial project-owner classification for semantic hardening and streamlining. This document records intended architectural standing, not a guarantee that every listed surface is already fully hardened.

## Classification labels

| Label | Meaning |
|---|---|
| `core` | Required for the canonical spine. |
| `supporting` | Not central, but needed to operate or inspect the core. |
| `experimental` | Useful but not product-stable; must be labelled as such in docs and help. |
| `compatibility` | Retained to bridge old or external usage. |
| `internal-tooling` | Development tooling, not product surface. |
| `test-fixture` | Required by tests only. |
| `delete-candidate` | Stale, duplicate, misleading, or superseded. |

## Crates

| Surface | Classification | Rationale | Action |
|---|---|---|---|
| `earmark-core` | core | Shared domain model: IDs, objects, declarations, run records, standing, provider records, and serialization primitives. | Retain and harden invariants. |
| `earmark-store` | core | Git-backed canonical durable storage for object versions, payloads, heads, workspace layout, and write locking. | Retain and harden failure/rollback behaviour. |
| `earmark-index` | core | Rebuildable derived projection for query, relations, standing, active systems, and undo visibility. | Retain; make rebuild atomic/replacement-safe. |
| `earmark-declarations` | core | Declaration parsing, loading, and validation. | Retain; align validation with runtime capability. |
| `earmark-connected-context` | core | Bounded work-surface compilation from canonical objects and declared context templates. | Retain; harden path/error handling and expansion boundaries. |
| `earmark-exec` | core | Staged execution runtime: workflow execution, assignments, change sets, handoffs, failures, provider records, and relation persistence. | Retain; enforce canonical relation authorization and partial-run semantics. |
| `earmark-governance` | core | Review, standing, export, and governance event logic. | Retain; keep tied to execution and standing policy semantics. |
| `earmark-cli` | core | Primary operator interface over the canonical spine. | Retain; modularize later and make stability metadata executable. |
| `earmark-runtime-tools` | supporting | Runtime helper utilities; supports execution and integration but is not itself the spine. | Retain if actively used; re-evaluate after hardening. |
| `earmark` | compatibility | CLI-backed Rust facade. It shells out to `earmark-cli`, so it is not yet the primary in-process API. | Quarantine as compatibility; later replace with in-process facade or rename wrapper. |

## CLI command families

| Command family | Classification | Rationale | Action |
|---|---|---|---|
| `init` | core | Explicit workspace creation is part of the canonical substrate lifecycle. | Retain; ensure no other command silently initializes. |
| `status` | core | Minimal workspace inspection. | Retain. |
| `doctor` | supporting | Repair/diagnostic surface for canonical and derived state. | Retain; expose index rebuild diagnostics. |
| `system` | core | Registers and activates systems. | Retain; keep validation strict. |
| `declare` | core | Declaration authoring, validation, registration, and explanation. | Retain; align validation with runtime subset. |
| `deposit` | core | Canonical object creation by operator input. | Retain; ensure explicit workspace requirement. |
| `query` | core | Derived-index inspection of canonical state. | Retain. |
| `relation` | core | Relation inspection and eventually relation creation/authorization visibility. | Retain/harden. |
| `review` | core | Operator review over canonical objects. | Retain. |
| `standing-request` | core | Standing lifecycle management. | Retain if compensating standing model remains central. |
| `workflow` | core | Starts staged execution. | Retain; prevent runtime-impossible workflows. |
| `run` | core | Inspects run records and execution history. | Retain; surface partial execution. |
| `assignment` | core | Inspects assignment artifacts. | Retain. |
| `change-set` | core | Inspects transition-created durable changes. | Retain. |
| `handoff` | core | Inspects bounded continuation surfaces. | Retain. |
| `failure` | core | Inspects durable failure records. | Retain. |
| `audit` | supporting | Aggregated diagnostic/audit views over core artifacts. | Retain, but keep subordinate to canonical artifacts. |
| `report` | supporting | Human-readable rendering of run/handoff/system state. | Retain. |
| `context` | core | Explicit context compilation surface. | Retain; harden boundary behaviour. |
| `provider` | supporting | Provider capabilities and integration diagnostics. | Retain with stricter provider boundary semantics. |
| `undo` | core | Compensating visibility model for run-created artifacts. | Retain if undo remains compensating and non-destructive. |
| `orchestration` | experimental | Dogfooding/local execution ledger for project development. Useful, but not the main product spine. | Quarantine in experimental/internal docs; do not expand until core semantics are hardened. |
| `completions` | supporting | Shell integration convenience. | Retain. |

## Scripts

| Surface | Classification | Rationale | Action |
|---|---|---|---|
| `scripts/dispatch-native.sh` | internal-tooling | Local orchestration helper for dogfooding the repository. | Move or document under internal tooling; not product surface. |
| `scripts/dispatch-opencode.sh` | internal-tooling | Local OpenCode executor bridge. | Keep only as local helper; ensure environment overrides and clear non-product status. |
| `scripts/dispatch-opencode-engram.sh` | internal-tooling | Local Engram/OpenCode bridge. | Keep only as local helper; no product Rust code may depend on local paths. |
| `scripts/smoke-opencode-big-pickle.sh` | internal-tooling | Local smoke test for a particular executor configuration. | Keep internal or delete if no longer used. |
| Other `scripts/*` | supporting or internal-tooling | Classification depends on whether the script verifies core behaviour or supports local experimentation. | Inventory before pruning; keep one canonical script per purpose. |

## Examples

| Surface | Classification | Rationale | Action |
|---|---|---|---|
| `examples/research-synthesis` | core | Primary tutorial/demo for the declared workflow model. | Retain and keep green. |
| `examples/knowledge-briefing` | supporting | Additional domain example for staged knowledge work. | Retain if current declarations validate; otherwise repair or demote. |
| `examples/kommunale-waermeplanung` | supporting | Domain-specific example demonstrating non-English/public-planning use. | Retain if current declarations validate; otherwise repair or demote. |
| `examples/earmark-dev-orchestration` | experimental | Self-hosting/dogfooding orchestration system. | Quarantine as experimental/internal; do not present as core product flow. |
| `examples/earmark_dev_trials` | internal-tooling | Development trial/dogfooding material. | Keep only if actively used by tests or local workflows; otherwise mark delete-candidate. |
| Live-smoke/internal provider examples | internal-tooling | Useful for integration checks but not product tutorial surface. | Move under internal docs/examples if user-facing docs currently expose them. |

## Documentation

| Surface | Classification | Rationale | Action |
|---|---|---|---|
| `README.md` | core | Product entry point. | Keep focused on canonical spine; avoid foregrounding orchestration. |
| `docs/tutorials/quickstart.md` | core | First operator path. | Keep current and minimal. |
| `docs/tutorials/practical-guide.md` | core | Product explanation. | Keep aligned with canonical spine. |
| `docs/concepts/staged-execution.md` | core | Explains execution spine. | Retain. |
| `docs/concepts/context-compilation.md` | core | Explains bounded context. | Retain. |
| `docs/concepts/standing.md` | core | Explains standing lifecycle. | Retain. |
| `docs/concepts/relation-authorization.md` | core | Explains declared relation authorization. | Retain; must match implementation after hardening. |
| `docs/reference/cli.md` | core | Command reference. | Retain; later reflect stability metadata. |
| `docs/reference/runtime-integration-guide.md` | supporting | External integration guide. | Update to distinguish in-process API from CLI-backed compatibility facade. |
| `docs/reference/provider-extension.md` | supporting | Provider extension surface. | Retain, but keep provider risks explicit. |
| `docs/declarations/README.md` | core | Declaration authoring reference. | Retain. |
| `docs/concepts/native-orchestration.md` | experimental | Dogfooding orchestration ledger. | Move or label as experimental/internal; do not place in core navigation. |
| Baseline verification snapshots | delete-candidate | Historical local state reports become stale quickly. | Delete if not referenced; keep only current completion reports under `docs/internal/`. |
| Work-packet/process notes | internal-tooling | Useful for agentic implementation process but not product docs. | Keep under `docs/internal/` only. |

## Immediate pruning decisions

These decisions should guide the next PR after this inventory:

1. Keep orchestration code if already merged, but quarantine it as experimental dogfooding infrastructure.
2. Do not expand orchestration until relation authorization, index rebuild, and workflow completion semantics are hardened.
3. Treat the `earmark` crate as compatibility until it becomes an in-process facade.
4. Remove stale baseline reports that claim historical green states unless they are needed as audit history.
5. Keep one canonical example for first-user documentation: `examples/research-synthesis`.
6. Keep additional examples only if they validate against current declarations and serve distinct domain coverage.
7. Keep local executor scripts, if at all, under internal-tooling framing.

## Implementation implications

The next hardening work should target the core spine, in this order:

1. canonical relation authorization;
2. atomic/replacement-safe derived index rebuild;
3. explicit partial workflow status;
4. declaration/runtime contract alignment;
5. explicit workspace initialization semantics;
6. provider boundary hardening;
7. edge-case and property test expansion;
8. in-process facade replacement;
9. CLI modularisation and stability catalog.

The local runtime should not decide what the project is. It should implement the decisions encoded in `docs/architecture/canonical-spine.md` and this inventory.
