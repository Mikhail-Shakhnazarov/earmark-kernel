# Semantic Streamlining Completion Report

© 2026 Mikhail Shakhnazarov

## Summary

- Streamlining and semantic hardening stages were executed on `hardening/canonical-relation-authorization`.
- Stages 1-7 were implemented in prior and current branch work.
- This pass completed Stage 8 facade replacement (primary in-process API), Stage 9 command-catalog contract, Stage 10 architecture docs fill-ins, and Stage 11 release-readiness gates.

## Removed or Quarantined

- Experimental orchestration remains quarantined and explicitly documented as internal/dogfooding.
- CLI shell-out compatibility API is retained as `CliBackedWorkspace` instead of being treated as the primary Rust API.

## Hardened Semantics

- Canonical relation writes enforce authorization and provenance mode checks.
- Derived-index rebuild behavior is transactional with dirty-marker lifecycle and skipped-entry diagnostics.
- Workflow partial completion is represented explicitly as `partial`.
- Runtime-impossible workflow declarations (multi-output transform) fail before execution.
- Workspace initialization is explicit (`em init`); write paths do not silently bootstrap layout.
- HTTP provider boundary enforces URL/domain safety with explicit local override.
- Edge/property test coverage expanded for ID boundaries, relation authorization edges, context depth/cycles, and redaction.
- Primary Rust workspace API is now in-process (`EarmarkWorkspace`), with shell-out compatibility preserved in `CliBackedWorkspace`.
- CLI command stability metadata is executable via `em commands --json`.

## Verification

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | pass |
| `cargo check --workspace` | pass |
| `cargo test --workspace` | pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | pass |
| `cargo test --workspace --features http-provider` | pass |
| `cargo clippy --workspace --all-targets --features http-provider -- -D warnings` | pass |
| CLI smoke | pass |
| JSON CLI smoke | pass |

## Remaining Risks

- In-process `report_run` currently emits a compact report summary and graph placeholder rather than the full CLI HTML renderer.
- Relation authorization trusted-provenance evaluation still depends on governance hard-coded trust rather than a single store/runtime-configured trust authority.

## Recommendation

Not ready for packaging milestone.
