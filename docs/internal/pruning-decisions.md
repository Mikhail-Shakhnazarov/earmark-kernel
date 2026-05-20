# Pruning and Quarantine Decisions

© 2026 Mikhail Shakhnazarov

## Status

Authoritative project-owner decision record for the first streamlining pass.

This document records what has been decided before local semantic hardening begins. It does not claim that all hardening work has been completed.

## Completed in this pass

### Canonical spine established

The canonical spine is documented in:

```text
docs/architecture/canonical-spine.md
```

The project should now be interpreted through this spine:

```text
declarations
  -> bounded context / work surface
  -> staged execution
  -> durable artifacts
  -> derived index
  -> query / audit / report
```

### Surface inventory established

The initial surface inventory is documented in:

```text
docs/internal/surface-inventory.md
```

It classifies crates, command families, scripts, examples, and documentation into:

```text
core
supporting
experimental
compatibility
internal-tooling
test-fixture
delete-candidate
```

### Native orchestration quarantined

Native orchestration remains in the repository, but it is classified as experimental/internal dogfooding infrastructure.

It must not be used as the main explanation of Earmark's product architecture.

It must not be expanded before the following core hardening work is complete:

1. canonical relation authorization;
2. atomic/replacement-safe derived index rebuild;
3. explicit partial workflow status;
4. declaration/runtime contract alignment;
5. explicit workspace initialization semantics.

### Shell-out facade quarantined

The `earmark` crate is classified as compatibility.

It currently shells out to `earmark-cli` and parses JSON output. It is not the primary in-process Rust API.

The crate-level documentation now states this directly. The interim error path now attempts to parse stdout JSON error envelopes on non-zero CLI exit before falling back to stderr.

## Decisions for the next local-verified pruning pass

### Do not delete code before green local verification

This pass deliberately does not remove code, move scripts, or delete examples. Those operations need local reference checks and test execution.

### Delete-candidates to verify locally

The next local runtime should search for and verify references before deletion:

- stale baseline verification snapshots that claim old green states;
- duplicate work-packet/process notes outside `docs/internal/`;
- live-smoke provider artifacts not referenced by tests or current docs;
- local executor scripts that duplicate another retained script;
- examples that no longer validate against the current declaration/runtime contract.

### Script consolidation rule

Local executor scripts may remain only as internal tooling. If retained, they should either:

1. move under `scripts/internal/`; or
2. stay in `scripts/` with a visible header stating they are local dogfooding helpers, not product interfaces.

Keep one canonical script per purpose.

### Documentation navigation rule

User-facing documentation should foreground:

- declarations;
- deposit/query;
- bounded context;
- staged workflow execution;
- audit/report;
- compensating undo if retained.

User-facing documentation should not foreground:

- orchestration;
- OpenCode scripts;
- Engram integration;
- shell-out facade internals;
- speculative async/provider seams.

## Implementation implication

The next implementation branch should start from current `dev` and treat Stage 1 through Stage 3.2 of the streamlining plan as already done.

The next code-bearing stage is canonical relation authorization.
