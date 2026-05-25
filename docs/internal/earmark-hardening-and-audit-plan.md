# Earmark Hardening and Audit Implementation Plan

## Document Purpose

This document is a self-contained implementation plan to improve Earmark's correctness, reliability, maintainability, and security posture. It includes:

1. A prioritized issue register based on a workspace audit.
2. A concrete remediation roadmap with sequencing and ownership guidance.
3. Acceptance criteria and verification steps for each workstream.

This plan is intentionally independent of any external planning system or prior chat context.

## Scope and Goals

### Primary goals

- Eliminate panic-prone API paths introduced by recent typed-ID refactors.
- Preserve backward compatibility for run and transition identifiers where feasible.
- Complete mutability migration safely (`&self` -> `&mut self`) across all affected APIs.
- Reduce high-cost full-store scans in runtime and CLI hot paths.
- Remove report-generation trust/safety gaps (HTML injection, remote third-party runtime dependencies).
- Improve maintainability in high-churn/high-complexity files.

### Non-goals

- Full architectural rewrite of canonical store or execution engine.
- Immediate elimination of all technical debt.
- Replacing the current object model or index subsystem.

## Audit Method Summary

Audit lenses used:

- Rust API safety and panic surfaces.
- Data model and serialization compatibility.
- Performance and scalability hotspots.
- Security and trust-boundary review.
- Code health and maintainability diagnostics.
- Git/project-health signals (churn, bug-cluster indicators).

Static evidence was collected from source and metadata files in this workspace.

## Prioritized Issue Register

### P0-1: Blanket `Default` on `TypedId<T>` can panic at runtime

- Severity: P0
- Why this matters: Code that appears safe (`Default::default()`) can abort execution for ID types that intentionally cannot auto-generate.
- Evidence:
  - `earmark-core/src/ids.rs`: blanket `Default` for `TypedId<T>` uses `generate()`.
  - `SymbolicNameSpec`, `TransitionIdSpec`, `DimensionIdSpec`, `TokenIdSpec`, `KernelProtocolIdSpec` define `generate_body()` with `panic!`.
- Risk:
  - Hidden runtime crashes.
  - Hard-to-debug failures in generic or derived code paths.
- Fix direction:
  - Remove blanket `Default` for `TypedId<T>`.
  - Implement `Default` only for truly generatable IDs (`ObjectId`, `VersionId`, `RunId`, `ChangeSetId`, etc.).
  - Replace panic-based generation blocks with compile-time non-availability of `Default`.

### P0-2: Generic `as_object_id()` on `TypedId<T>` is panic-prone and type-unsafe

- Severity: P0
- Why this matters: Method is available for all typed IDs but only valid for object-ID-compatible types.
- Evidence:
  - `earmark-core/src/ids.rs`: `TypedId<T>::as_object_id()` uses `expect(...)` after parse.
- Risk:
  - Accidental runtime panic from invalid conversion.
  - API misleads callers into unsafe conversion patterns.
- Fix direction:
  - Remove generic `as_object_id()`.
  - Reintroduce conversion via explicit trait (`IntoObjectId`) implemented only for compatible IDs.
  - Update call sites accordingly.

### P0-3: Run/transition ID canonicalization introduces behavior drift and compatibility risk

- Severity: P0
- Why this matters: `RunId::parse` and `TransitionId::parse` currently prefix plain names (`run_`, `tr_`) via `extra_parse`, which changes serialized forms and user-visible values.
- Evidence:
  - `earmark-core/src/ids.rs`: `RunIdSpec::extra_parse`, `TransitionIdSpec::extra_parse` auto-prefix behavior.
  - Multiple tests still compare against unprefixed values (for example `"op_transform"` in `earmark-exec/tests/engine/error_handling.rs`), indicating migration incompleteness.
- Risk:
  - Inconsistent behavior across old and new records.
  - User-facing ID drift and unexpected filtering misses.
- Fix direction:
  - Decide and document canonical policy:
    1. Keep original token as stored (compat-first), or
    2. Store canonical prefixed forms but normalize all boundaries consistently.
  - Implement one compatibility layer only (centralized normalization function), not distributed ad-hoc parsing.
  - Add migration/compat tests for legacy payloads.

### P0-4: Inconsistent run-ID normalization in CLI/reporting path

- Severity: P0
- Why this matters: Some functions normalize run IDs while others use raw input, causing false "not found" or partial reports.
- Evidence:
  - `earmark-cli/src/app/listing.rs`: `load_run_record_by_id` normalizes input using `RunId::parse`.
  - `earmark-cli/src/app/reports.rs`: `generate_run_report` loads ledger using normalized path but then calls `run_related_artifacts(store, run_id)` and `build_run_graph(store, run_id)` with raw input.
  - `earmark-cli/src/app/resolve.rs`: `resolve_run_id` returns raw input except `latest`.
- Risk:
  - Report artifacts, provider records, and graph can mismatch the loaded run ledger.
- Fix direction:
  - Introduce one canonical resolver returning `ResolvedRunId`.
  - Thread resolved ID through all report/listing/graph functions.
  - Add regression tests for legacy and prefixed input forms.

### P1-1: Mutability migration (`&self` -> `&mut self`) is broad and must be fully closed

- Severity: P1
- Why this matters: Changes are directionally correct but cross-crate API transitions can leave stale call sites and subtle borrow-flow issues.
- Evidence:
  - `DerivedIndex` mutation methods now require `&mut self`.
  - `ExecutionEngine` and `RuntimeToolSurface` methods now require mutable receivers.
  - `build_errors.txt` records prior mutability mismatch compile failures.
- Risk:
  - Hidden build breaks in less-frequent code paths.
  - Partial migration with inconsistent borrowing contracts.
- Fix direction:
  - Perform full workspace compile closure and update all stale call sites.
  - Add compile-focused CI gate for all workspace members.
  - Add minimal API contract tests for mutable index flows.

### P1-2: Full-store scans in hot paths are likely to become a scaling bottleneck

- Severity: P1
- Why this matters: Numerous runtime and CLI operations scan all canonical objects and then filter in memory.
- Evidence:
  - `scan_objects()` loops in runtime tools, exec helpers, CLI loaders/listing/dispatch handlers.
- Risk:
  - O(N) behavior for common operations (assignment lookup, listings, report generation).
  - Increasing latency as corpus grows.
- Fix direction:
  - Move lookup-heavy flows to `DerivedIndex` query APIs.
  - Keep canonical scan only for rebuild/recovery paths.
  - Add measured benchmarks for run/report/list operations by object count.

### P1-3: Potential panic points remain in production paths

- Severity: P1
- Why this matters: A few `unwrap`/`expect` sites exist in runtime/CLI non-test code.
- Evidence:
  - `earmark-cli/src/app/emitter.rs`: JSON pretty-print fallback uses `unwrap()`.
  - `earmark-cli/src/app/dispatch/handlers.rs`: `expect("index available for workspace command")`.
  - `earmark-connected-context/src/lib.rs`: `strip_prefix(...).unwrap()` for path rendering.
- Risk:
  - Hard crashes on edge conditions.
- Fix direction:
  - Replace with typed error propagation or defensive fallback.
  - Add edge-case tests for path-prefix mismatch and output serialization failure handling.

### P1-4: Report HTML generation allows unescaped content insertion

- Severity: P1
- Why this matters: Report fields are interpolated directly into HTML; stored content can include markup.
- Evidence:
  - `earmark-cli/src/app/reports.rs` uses direct `format!` insertion of IDs/messages/provider fields.
- Risk:
  - Script/style injection in rendered reports.
  - Unsafe local report viewing behavior.
- Fix direction:
  - Centralize HTML escaping for all dynamic content.
  - Treat report rendering as untrusted-content transformation.
  - Add security-focused tests with adversarial payload strings.

### P1-5: Report generation relies on remote JS/CSS assets

- Severity: P1
- Why this matters: Runtime fetches third-party resources (CDN JS, Google fonts).
- Evidence:
  - `earmark-cli/src/app/reports.rs`: external Mermaid and font URLs.
- Risk:
  - Nondeterministic/offline failures.
  - Supply-chain and privacy concerns.
- Fix direction:
  - Vendor required assets or provide offline-safe fallback.
  - Add a `--offline-safe` report mode (default recommended).

### P2-1: Maintainability hotspots (very large files, broad responsibilities)

- Severity: P2
- Why this matters: Large files reduce review quality, slow onboarding, and increase regression probability.
- Evidence (line-count hotspots):
  - `earmark-cli/src/app/commands/orchestration.rs` (~2240 LOC)
  - `earmark-connected-context/src/lib.rs` (~1862 LOC)
  - `earmark-index/src/lib.rs` (~1090 LOC)
  - `earmark-exec/src/transition.rs` (~956 LOC)
- Risk:
  - High cognitive load and fragile edits.
- Fix direction:
  - Extract per-domain modules with narrow interfaces.
  - Add internal ADR for module boundaries.

### P2-2: Code health warnings and cleanup debt

- Severity: P2
- Why this matters: Persistent warnings reduce signal for new regressions.
- Evidence:
  - `build_errors.txt` shows warning accumulation (unused doc comments, unused vars, dead-code warning artifacts).
- Risk:
  - Warning fatigue and hidden real issues.
- Fix direction:
  - Warning budget policy per crate.
  - Resolve existing warning set before adding new APIs in affected files.

### P3-1: Local generated artifact clutter in repo root

- Severity: P3
- Why this matters: Untracked generated/debug files create accidental commit risk and noisy workspace state.
- Evidence:
  - Files like `build_errors.txt`, `final_list.json`, `raw_active_list.txt`, `tasks_to_clean.json`, etc.
- Risk:
  - Accidental source-control pollution.
- Fix direction:
  - Decide whether to persist as fixtures or ignore as transient outputs.
  - Add precise `.gitignore` entries for local analysis artifacts.

## Implementation Roadmap

## Phase 0: Alignment and Contract Freeze (1-2 days)

### Deliverables

- ADR: typed-ID behavior and compatibility policy.
- ADR: canonical run/transition ID normalization contract.
- Issue tracker mapping from this document IDs (`P0-1`, `P0-2`, etc.).

### Tasks

1. Finalize canonical ID policy (preserve legacy string vs canonical prefix storage).
2. Define conversion boundary rules (CLI input, API payloads, persistence).
3. Freeze report-security acceptance criteria.

### Exit criteria

- Approved ADRs.
- No implementation starts without policy sign-off.

## Phase 1: Correctness and Panic Elimination (P0, 3-5 days)

### Workstream A: Typed-ID API hardening

1. Remove blanket `Default` on `TypedId<T>`.
2. Add explicit `Default` impls only for safe/generatable ID aliases.
3. Remove generic `as_object_id`; replace with constrained conversions.
4. Update all call sites.

### Workstream B: ID normalization consistency

1. Introduce `normalize_run_id_input(...)` and `normalize_transition_id_input(...)` helper APIs in one module.
2. Use normalized IDs consistently in:
   - listing
   - report generation
   - graph generation
   - filtering helpers
3. Add compatibility tests for:
   - legacy unprefixed IDs
   - prefixed IDs
   - round-trip serialization.

### Exit criteria

- No panic-prone typed-ID API usage in production paths.
- ID compatibility tests passing for all supported input forms.

## Phase 2: Mutability Migration Closure and Build Integrity (P1, 1-2 days)

### Tasks

1. Compile all workspace members and fix mutability signature mismatches.
2. Ensure all `write_object_and_index` and mutable index APIs are consistently passed `&mut` references.
3. Add CI guard to compile all crates with warnings surfaced.

### Exit criteria

- Full workspace compile closure.
- No stale immutable call sites for mutable index APIs.

## Phase 3: Performance and Query Path Refactor (P1, 4-7 days)

### Tasks

1. Inventory high-frequency `scan_objects()` usages.
2. For each runtime/CLI hot path, introduce index-backed query helpers.
3. Keep canonical scans only for rebuild/recovery and explicit diagnostics.
4. Add benchmark harness or timing assertions for representative corpus sizes.

### Suggested initial targets

- assignment lookup/release flows
- report artifact aggregation
- run-related list commands

### Exit criteria

- Material reduction of full-store scans in common commands.
- Measured latency improvement for listing/report workflows.

## Phase 4: Security Hardening for Reports (P1, 2-4 days)

### Tasks

1. Add HTML escaping utility and apply to all dynamic insertions.
2. Introduce offline-safe asset mode (local/vendored assets, no remote dependency by default).
3. Add adversarial test fixtures for report payload rendering.

### Exit criteria

- No unescaped dynamic HTML insertion paths.
- Reports render without internet access.

## Phase 5: Maintainability Refactors (P2, incremental)

### Tasks

1. Split orchestration command file into focused modules:
   - ingest
   - status/list/show
   - review/gates
   - explain/timeline
2. Split connected-context and index monoliths into domain submodules.
3. Reduce `clippy::too_many_arguments` hotspots through typed parameter structs.

### Exit criteria

- Reduced file-size hotspots.
- Fewer broad lint suppressions.
- Clearer module boundaries documented.

## Phase 6: Workspace Hygiene and Policy (P3, 0.5-1 day)

### Tasks

1. Classify local generated artifacts as:
   - keep as fixture
   - move to tooling output directory
   - ignore via `.gitignore`
2. Add lightweight housekeeping conventions in contributing docs.

### Exit criteria

- Clean repo root baseline.
- Low accidental-commit risk for local artifacts.

## Detailed Task Backlog

### Epic E1: Typed-ID Safety

- E1-T1: Remove blanket `Default` for `TypedId<T>`.
- E1-T2: Add selective `Default` impls for safe aliases.
- E1-T3: Replace generic `as_object_id` with constrained conversion trait.
- E1-T4: Add compile-fail/behavior tests for non-generatable IDs.

### Epic E2: ID Compatibility and Normalization

- E2-T1: Centralize normalization helpers.
- E2-T2: Normalize all run-filtering/reporting call chains.
- E2-T3: Add golden tests for legacy payload compatibility.
- E2-T4: Update external docs describing ID contract.

### Epic E3: Mutability and Build Closure

- E3-T1: Audit all mutable index call sites.
- E3-T2: Fix remaining borrow/mutability signatures.
- E3-T3: Add workspace compile gate in CI.

### Epic E4: Performance

- E4-T1: Build query map for current `scan_objects` call sites.
- E4-T2: Implement index-backed assignment and run-artifact queries.
- E4-T3: Add benchmarks and regression thresholds.

### Epic E5: Report Security

- E5-T1: Implement HTML escaping and sanitize all dynamic fields.
- E5-T2: Remove default remote asset dependencies.
- E5-T3: Add injection regression tests.

### Epic E6: Maintainability and Hygiene

- E6-T1: Modularize orchestration command file.
- E6-T2: Modularize connected-context/index internals.
- E6-T3: Warning cleanup pass and policy.
- E6-T4: Local artifact hygiene policy.

## Verification Strategy

## Unit and integration verification

1. Typed-ID behavior tests:
   - non-generatable IDs do not implement dangerous default behavior.
   - conversions are explicit and constrained.
2. Compatibility tests:
   - legacy run/transition IDs still resolve as intended.
3. CLI/report tests:
   - normalized IDs produce consistent ledger/artifact/graph output.
4. Security tests:
   - HTML-escaping test vectors are rendered inert.
5. Performance tests:
   - command latency remains bounded as object count scales.

## Release safety checklist

- No new panics in production paths from ID operations.
- All crate members compile in CI.
- Report generation passes offline and escaped-content tests.
- Key user-facing commands validated with mixed legacy and current IDs.

## Suggested Sequencing and Capacity

- Sprint A: Phase 0 + Phase 1 (correctness first).
- Sprint B: Phase 2 + start Phase 3 (build closure + top performance wins).
- Sprint C: Phase 4 + targeted Phase 5 modules.
- Continuous: Phase 6 hygiene and warning debt reduction.

## Risk Register

- Risk: Backward compatibility ambiguity during ID policy decision.
  - Mitigation: Freeze ADR before code changes and add golden compatibility fixtures.
- Risk: Refactor breadth causes merge conflict churn.
  - Mitigation: slice by epic and keep PRs small with narrow acceptance tests.
- Risk: Performance refactor introduces behavior drift.
  - Mitigation: golden-output tests before query-path replacement.

## Definition of Done

This plan is complete when:

1. P0 issues are closed and verified.
2. Workspace compile and core command flows are stable under the new mutability and ID contracts.
3. Report path is escaped and offline-safe by default.
4. Major scan-based hotspots are replaced or isolated with explicit rationale.
5. High-churn modules have a documented decomposition roadmap underway.

---

## Appendix A: Key Audit Evidence (File Pointers)

- Typed-ID default/conversion surfaces: `earmark-core/src/ids.rs`
- Run/list/report normalization paths:
  - `earmark-cli/src/app/listing.rs`
  - `earmark-cli/src/app/resolve.rs`
  - `earmark-cli/src/app/reports.rs`
- Panic-prone paths:
  - `earmark-cli/src/app/emitter.rs`
  - `earmark-cli/src/app/dispatch/handlers.rs`
  - `earmark-connected-context/src/lib.rs`
- Scan-heavy paths:
  - `earmark-cli/src/app/loaders.rs`
  - `earmark-cli/src/app/listing.rs`
  - `earmark-runtime-tools/src/modules/workflow.rs`
  - `earmark-exec/src/helpers.rs`
- Report rendering trust boundary: `earmark-cli/src/app/reports.rs`
- Mutability migration context: `earmark-index/src/lib.rs`, `earmark-exec/src/engine.rs`, `earmark-runtime-tools/src/modules/workflow.rs`


## Appendix B: Audit Snapshot Metrics

### Git Health Snapshot

- Contributors (all-time and last 6 months):
  - Mikhail Shakhnazarov: 152 commits
  - mShak: 32 commits
- Firefighting marker count in last year (`revert|hotfix|emergency|rollback|outage|incident`): 4 commits.
- Churn hotspots (last year, top sample):
  - `earmark-core/src/lib.rs`
  - `earmark-cli/tests/cli.rs`
  - `earmark-cli/src/app.rs`
  - `earmark-index/src/lib.rs`
  - `earmark-cli/src/app/commands/orchestration.rs`

### Bug-Fix Cluster Indicators (commit-message heuristic)

Top files appearing in fix/bug/regression commit messages (last year, top sample):

- `earmark-cli/tests/cli.rs`
- `earmark-core/src/lib.rs`
- `docs/reference/cli.md`
- `README.md`
- `earmark-index/src/lib.rs`
- `earmark-cli/src/app/commands/orchestration.rs`

Interpretation:

- Runtime/index/CLI layers recur in both churn and bug-fix patterns, supporting prioritization of those subsystems.

### Codebase Size and Complexity Hotspots

Top Rust LOC hotspots:

- `earmark-cli/src/app/commands/orchestration.rs` (~2240 LOC)
- `earmark-connected-context/src/lib.rs` (~1862 LOC)
- `earmark-core/src/projection.rs` (~1443 LOC)
- `earmark-exec/src/http_generation.rs` (~1191 LOC)
- `earmark-index/src/lib.rs` (~1090 LOC)
- `earmark-exec/src/transition.rs` (~956 LOC)
- `earmark-cli/src/app/dispatch/handlers.rs` (~953 LOC)

Interpretation:

- Large, multi-responsibility files correlate with high-change areas and should be decomposed first in maintainability work.

# Earmark Hardening and Audit Execution Runbook

## 1. Execution Contract

This document is an implementation-facing runbook intended for direct handoff to an executing runtime.

- It is self-contained.
- It does not rely on chat context.
- It does not rely on external planning systems.
- Tasks are ordered, dependency-aware, and acceptance-gated.

Execution policy:

1. Execute tasks in dependency order.
2. Do not skip validation commands.
3. If a task fails validation, stop and fix before continuing.
4. Commit in small atomic changes per task.

## 2. Runtime Inputs and Environment

- Repository root: `/home/mikhails/from_ubuntu/GITHUB/earmark-workspace`
- Primary language/tooling: Rust workspace (`cargo`)
- Shell: POSIX-compatible (`bash`)

Required tools:

- `cargo`
- `rg`
- `git`

Optional tools:

- `cargo clippy`

## 3. Task DAG (Machine-Readable)

```yaml
plan_id: earmark-hardening-2026-05
entrypoint_tasks:
  - T00

tasks:
  - id: T00
    title: Baseline and branch prep
    depends_on: []
  - id: T10
    title: Remove panic-prone blanket Default on TypedId
    depends_on: [T00]
  - id: T11
    title: Remove generic panic-prone as_object_id API
    depends_on: [T10]
  - id: T12
    title: Add explicit safe conversion traits for obj-compatible IDs
    depends_on: [T11]
  - id: T20
    title: Define and implement canonical run/transition normalization policy
    depends_on: [T00]
  - id: T21
    title: Thread normalized run IDs through listing/report/graph flows
    depends_on: [T20]
  - id: T22
    title: Add compatibility regression tests for legacy and prefixed IDs
    depends_on: [T21]
  - id: T30
    title: Close mutability migration and compile all workspace members
    depends_on: [T12, T22]
  - id: T40
    title: Replace panic sites in production paths
    depends_on: [T30]
  - id: T50
    title: Report HTML escaping hardening
    depends_on: [T30]
  - id: T51
    title: Remove default remote report dependencies (offline-safe default)
    depends_on: [T50]
  - id: T60
    title: Reduce full-store scans in hot paths via index queries
    depends_on: [T30]
  - id: T70
    title: Warning debt and hygiene cleanup
    depends_on: [T40, T51, T60]
  - id: T80
    title: Final verification and release checklist
    depends_on: [T70]
```

## 4. Task Packets

### T00 - Baseline and branch prep

Goal:

- Capture current state and ensure repeatable execution context.

Files modified:

- none

Commands:

```bash
cd /home/mikhails/from_ubuntu/GITHUB/earmark-workspace
git status --short
git rev-parse --abbrev-ref HEAD
git rev-parse HEAD
```

Acceptance:

- Repository status captured.
- Current branch and commit SHA logged.

---

### T10 - Remove panic-prone blanket `Default` on `TypedId`

Issue addressed:

- P0-1 (`TypedId<T>` blanket `Default` can panic for non-generatable specs).

Primary files:

- `earmark-core/src/ids.rs`

Required edits:

1. Remove generic `impl<T: IdSpec> Default for TypedId<T>`.
2. Add explicit `Default` impls only for IDs that are safely generatable:
   - `ObjectId`
   - `VersionId`
   - `RunId`
   - `TransitionAssignmentId`
   - `ChangeSetId`
   - `UndoRecordId`
   - `HandoffManifestId`
3. Confirm non-generatable IDs do not implement `Default`.

Validation commands:

```bash
cd /home/mikhails/from_ubuntu/GITHUB/earmark-workspace
cargo check -p earmark-core
rg -n "impl<T: IdSpec> Default for TypedId" earmark-core/src/ids.rs
```

Acceptance:

- `cargo check -p earmark-core` passes.
- No blanket `Default` impl remains.

---

### T11 - Remove generic panic-prone `as_object_id`

Issue addressed:

- P0-2 (generic conversion available for all typed IDs, panic-prone).

Primary files:

- `earmark-core/src/ids.rs`

Required edits:

1. Remove `TypedId<T>::as_object_id()` from generic impl block.
2. Replace with constrained conversion interfaces in T12.

Validation commands:

```bash
cd /home/mikhails/from_ubuntu/GITHUB/earmark-workspace
rg -n "fn as_object_id\(" earmark-core/src/ids.rs
cargo check -p earmark-core
```

Acceptance:

- Generic `as_object_id` removed.
- Build remains green.

---

### T12 - Add explicit safe conversion traits for obj-compatible IDs

Issue addressed:

- P0-2 remediation completion.

Primary files:

- `earmark-core/src/ids.rs`
- any compile-break call sites surfaced by `cargo check`

Required edits:

1. Introduce trait (for example `IntoObjectId`) with method returning `ObjectId`.
2. Implement only for object-compatible IDs:
   - `ObjectId`
   - `TransitionAssignmentId`
   - `ChangeSetId`
   - `UndoRecordId`
   - `HandoffManifestId`
3. Update call sites currently relying on removed generic API.

Validation commands:

```bash
cd /home/mikhails/from_ubuntu/GITHUB/earmark-workspace
cargo check -p earmark-core
cargo check -p earmark-exec
cargo check -p earmark-runtime-tools
```

Acceptance:

- Only constrained conversions exist.
- All impacted crates compile.

---

### T20 - Define and implement canonical run/transition normalization policy

Issues addressed:

- P0-3 (ID canonicalization behavior drift).
- P0-4 (inconsistent normalization paths).

Primary files:

- `earmark-core/src/ids.rs`
- `earmark-cli/src/app/resolve.rs`
- `earmark-cli/src/app/listing.rs`

Required edits:

1. Choose one canonical policy and encode it in code comments + docs in-file:
   - either compat-first preservation, or canonical prefix persistence.
2. Add central normalization helper(s) with single source of truth.
3. Remove ad-hoc normalization behavior spread across unrelated functions.

Validation commands:

```bash
cd /home/mikhails/from_ubuntu/GITHUB/earmark-workspace
cargo check -p earmark-core -p earmark-cli
rg -n "normalize.*run|normalize.*transition|RunId::parse\(|TransitionId::parse\(" earmark-cli/src/app
```

Acceptance:

- Normalization policy is explicit and centralized.
- CLI compile succeeds.

---

### T21 - Thread normalized run IDs through listing/report/graph flows

Issue addressed:

- P0-4 (partial report/list mismatches due to mixed raw/resolved IDs).

Primary files:

- `earmark-cli/src/app/reports.rs`
- `earmark-cli/src/app/listing.rs`
- `earmark-cli/src/app/resolve.rs`
- `earmark-cli/src/app/graph.rs`

Required edits:

1. Ensure `generate_run_report(...)` uses resolved/canonical run ID for all downstream calls.
2. Ensure `run_related_artifacts(...)`, graph building, provider record filtering use same resolved ID.
3. Ensure CLI command handlers route `latest` and legacy/prefixed run IDs through one resolver.

Validation commands:

```bash
cd /home/mikhails/from_ubuntu/GITHUB/earmark-workspace
cargo check -p earmark-cli
```

Acceptance:

- No mixed raw/resolved run ID usage in report flow.
- CLI compiles cleanly.

---

### T22 - Compatibility regression tests for legacy and prefixed IDs

Issues addressed:

- P0-3 and P0-4 verification.

Primary files:

- `earmark-cli/tests/*.rs` (new or existing)
- `earmark-core/src/ids.rs` tests module or dedicated crate tests
- `earmark-exec/tests/engine/error_handling.rs` (update assertions if needed per chosen policy)

Required edits:

1. Add tests proving both input forms resolve correctly (legacy + prefixed).
2. Add serialization/round-trip tests for stable persisted representation under chosen policy.
3. Update stale expectations that conflict with policy.

Validation commands:

```bash
cd /home/mikhails/from_ubuntu/GITHUB/earmark-workspace
cargo test -p earmark-core
cargo test -p earmark-cli
cargo test -p earmark-exec
```

Acceptance:

- Compatibility tests pass.
- No unresolved expectation drift.

---

### T30 - Close mutability migration and workspace compile closure

Issue addressed:

- P1-1 (mutability migration incompleteness risk).

Primary files:

- all compile-impacted files surfaced by `cargo check`

Required edits:

1. Fix all stale `&DerivedIndex` usages where mutable APIs are required.
2. Ensure all `write_object_and_index` call sites pass mutable index references where required.
3. Ensure `ExecutionEngine` and `RuntimeToolSurface` caller flows use mutable receivers correctly.

Validation commands:

```bash
cd /home/mikhails/from_ubuntu/GITHUB/earmark-workspace
cargo check --workspace
```

Acceptance:

- Entire workspace compiles.
- No mutability mismatch errors remain.

---

### T40 - Replace panic sites in production paths

Issue addressed:

- P1-3 (remaining `unwrap`/`expect` crash paths in non-test code).

Primary files (minimum):

- `earmark-cli/src/app/emitter.rs`
- `earmark-cli/src/app/dispatch/handlers.rs`
- `earmark-connected-context/src/lib.rs`

Required edits:

1. Replace `unwrap`/`expect` in non-test flows with:
   - typed error return, or
   - safe fallback behavior.
2. Preserve user-facing error quality.

Validation commands:

```bash
cd /home/mikhails/from_ubuntu/GITHUB/earmark-workspace
rg -n "unwrap\(|expect\(" earmark-cli/src earmark-connected-context/src --glob '!**/tests/**'
cargo check -p earmark-cli -p earmark-connected-context
```

Acceptance:

- Identified production panic sites removed.
- Affected crates compile.

---

### T50 - Report HTML escaping hardening

Issue addressed:

- P1-4 (HTML/script injection risk in generated reports).

Primary files:

- `earmark-cli/src/app/reports.rs`

Required edits:

1. Introduce centralized HTML escaping helper.
2. Escape every dynamic field before insertion into HTML templates.
3. Ensure graph labels and timeline/provider messages are escaped.

Validation commands:

```bash
cd /home/mikhails/from_ubuntu/GITHUB/earmark-workspace
cargo check -p earmark-cli
```

Acceptance:

- No direct insertion of unescaped dynamic values into HTML output.

---

### T51 - Offline-safe report mode as default

Issue addressed:

- P1-5 (remote JS/CSS dependencies for report rendering).

Primary files:

- `earmark-cli/src/app/reports.rs`
- optional assets under `earmark-cli` or `docs` depending on implementation

Required edits:

1. Remove runtime dependency on remote CDN/Google Fonts in default output.
2. Vendor or inline required assets, or degrade gracefully without remote fetch.
3. Keep report generation deterministic in offline environments.

Validation commands:

```bash
cd /home/mikhails/from_ubuntu/GITHUB/earmark-workspace
cargo check -p earmark-cli
rg -n "https://cdn|fonts.googleapis.com|fonts.gstatic.com" earmark-cli/src/app/reports.rs
```

Acceptance:

- Default report path has no remote asset dependency.

---

### T60 - Reduce full-store scans in hot paths

Issue addressed:

- P1-2 (O(N) scan hotspots).

Primary files (starting set):

- `earmark-cli/src/app/loaders.rs`
- `earmark-cli/src/app/listing.rs`
- `earmark-runtime-tools/src/modules/workflow.rs`
- `earmark-exec/src/helpers.rs`
- `earmark-index/src/lib.rs` (new query helpers)

Required edits:

1. Introduce index-backed query helpers for high-frequency lookups.
2. Replace direct `scan_objects()` loops in hot paths with index calls.
3. Keep scan-based fallback only in rebuild/recovery diagnostics.

Validation commands:

```bash
cd /home/mikhails/from_ubuntu/GITHUB/earmark-workspace
rg -n "scan_objects\(\)\?" earmark-cli/src earmark-runtime-tools/src earmark-exec/src --glob '!**/tests/**'
cargo check --workspace
```

Acceptance:

- Hot-path scan usages reduced materially.
- Workspace compile remains green.

---

### T70 - Warning debt and workspace hygiene

Issues addressed:

- P2-2 (warning accumulation)
- P3-1 (artifact clutter)

Primary files:

- warning-producing source files (as surfaced by build)
- `.gitignore`

Required edits:

1. Resolve known warning classes in edited hotspots (unused vars/comments/dead code where practical).
2. Add precise ignore patterns for local generated artifacts that should not be committed.
3. Avoid broad ignore patterns that hide legitimate source files.

Validation commands:

```bash
cd /home/mikhails/from_ubuntu/GITHUB/earmark-workspace
cargo check --workspace
```

Acceptance:

- Warning count reduced in touched areas.
- Root-level local artifact policy encoded in `.gitignore`.

---

### T80 - Final verification and release checklist

Goal:

- Ensure all preceding tasks are complete and safe to merge.

Commands:

```bash
cd /home/mikhails/from_ubuntu/GITHUB/earmark-workspace
cargo check --workspace
cargo test --workspace
```

Release checklist:

1. P0 tasks T10-T22 are complete and validated.
2. Workspace compiles fully.
3. ID compatibility tests pass.
4. Report output is escaped and offline-safe.
5. Scan hotspots reduced in runtime/CLI paths.
6. No known panic-prone production paths remain from this audit scope.

Acceptance:

- All checklist items pass.

## 5. Concrete Issue-to-Task Mapping

- P0-1 -> T10
- P0-2 -> T11, T12
- P0-3 -> T20, T22
- P0-4 -> T21, T22
- P1-1 -> T30
- P1-2 -> T60
- P1-3 -> T40
- P1-4 -> T50
- P1-5 -> T51
- P2-2 -> T70
- P3-1 -> T70

## 6. Operational Notes for Executor

- Keep PRs/task diffs small:
  - PR1: T10-T12
  - PR2: T20-T22
  - PR3: T30-T40
  - PR4: T50-T51
  - PR5: T60-T70
  - PR6: T80 final verification
- Stop-the-line conditions:
  - failing workspace compile after any task
  - incompatible ID behavior not covered by tests
  - regression in run/report resolution behavior

## 7. Audit Baseline Snapshot (for tracking impact)

- Contributor concentration observed (2 primary identities in current history sample).
- Firefighting keyword count in last-year history sample: 4 commits.
- High churn/hotspot files include:
  - `earmark-core/src/lib.rs`
  - `earmark-index/src/lib.rs`
  - `earmark-cli/src/app/commands/orchestration.rs`
- Largest Rust hotspots include:
  - `earmark-cli/src/app/commands/orchestration.rs`
  - `earmark-connected-context/src/lib.rs`
  - `earmark-core/src/projection.rs`
  - `earmark-index/src/lib.rs`

This baseline should be compared after T80 for measurable improvement.
