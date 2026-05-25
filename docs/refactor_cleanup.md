# Dev Refactor Cleanup Workpackage

© 2026 Mikhail Shakhnazarov. All rights reserved.

## Purpose

Bring the current `dev` branch from successful refactor state to release-clean integration state. This workpackage addresses six concrete issues found after the substantial refactor:

1. Repository residue and `.gitignore` contradiction.
2. Incorrect stability labeling for native orchestration.
3. Vocabulary split between native orchestration object classes.
4. Ambient Git provenance capture.
5. New orchestration monolith formation.
6. Full-index rebuilds after ordinary orchestration writes.

The target outcome is a clean, testable, mechanically reviewable `dev` branch that can be considered for promotion into `main` without carrying local-runtime residue, misleading public contracts, or avoidable scaling debt.

## Operating assumptions

This work begins from the current `dev` branch of `Mikhail-Shakhnazarov/earmark-workspace`.

The native orchestration surface remains a dogfooding/runtime feature, but it must not be falsely labeled stable. The implementation should prefer explicit contract discipline over optimistic public claims.

No behavioral compatibility is required for transient local files such as root-level debug logs or generated task lists. Compatibility is required for public CLI JSON envelopes unless this workpackage explicitly changes them.

## Global gates

Run these gates before opening the final PR:

```bash
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Also run the orchestration-specific smoke path:

```bash
cargo test -p earmark-cli --test wp09_orchestration_native
cargo test -p earmark-cli --test wp10_orchestration_schemas
cargo test -p earmark-cli --test wp11_relation_wiring
cargo test -p earmark-cli --test wp12_orchestration_lifecycle
```

If any gate fails, fix the implementation rather than weakening tests. Only update tests when the product contract intentionally changes.

---

# Stage 1 — Repository hygiene and tracked-residue removal

## Goal

Remove accidental local/runtime artifacts from version control and make `.gitignore` accurately describe what is local state versus promoted fixture/tooling.

## Required changes

Delete the following tracked root-level residue files unless an implementation pass proves one has become a deliberate fixture. Treat these as accidental artifacts by default:

```text
build_errors.txt
final_list.json
oids_proposed.txt
oids_ready.txt
raw_active_list.txt
tasks_to_clean.json
ear mark-runtime-tools/check_out.txt
```

Use the exact path actually present for `earmark-runtime-tools/check_out.txt`; the spaced form above must not be introduced.

Inspect the tracked `.opencode/` and `.orchestration/` paths. Apply the following policy:

* `.orchestration/` is local runtime state. Remove tracked `.orchestration/.gitkeep` and `.orchestration/README.md` unless the README is moved into `docs/internal/` or `docs/tutorials/` as documentation.
* `.opencode/commands/execute-manifest.md` is promoted tooling, not local state. Move it to `docs/internal/opencode/execute-manifest.md` or `scripts/opencode/execute-manifest.md`. Do not keep tracked files under `.opencode/` while `.gitignore` ignores `.opencode/`.
* Keep `.gitignore` entries for `.orchestration/` and `.opencode/` as local-only directories.

## Acceptance checks

```bash
git ls-files build_errors.txt final_list.json oids_proposed.txt oids_ready.txt raw_active_list.txt tasks_to_clean.json
git ls-files .orchestration .opencode
```

Both commands must return no tracked local-runtime residue. If promoted OpenCode guidance was moved, verify its new path is tracked under `docs/` or `scripts/` and referenced from the native orchestration quickstart.

---

# Stage 2 — Correct stability/status contract for native orchestration

## Goal

Align public command status with implementation maturity. Native orchestration is experimental or beta until the object vocabulary, Git capture, module boundaries, and indexing behavior are hardened.

## Required changes

In `earmark-cli/src/cli/core.rs`:

* Change the `Commands::Orchestration` help text from `[STABLE] Manage orchestration tasks` to `[EXPERIMENTAL] Manage native orchestration tasks`.
* Change `Commands::stability()` so `Commands::Orchestration(_)` returns `CommandStability::Experimental`.
* Change `command_catalog()` entry for `orchestration` to `CommandStability::Experimental` and summary `Manage native orchestration tasks`.

In `docs/reference/stability.md`:

* Mark native orchestration as experimental.
* State that the surface is intended for local dogfooding and development coordination.
* State that command names and JSON fields may still change before public stabilization.

In `docs/tutorials/native-orchestration-quickstart.md`:

* Add a short status note near the top: native orchestration is experimental but usable for local dogfooding.

## Tests

Add or update a CLI contract test asserting that:

```bash
em --json commands
```

reports `orchestration` with stability `experimental`.

## Acceptance checks

```bash
cargo test -p earmark-cli --test cli
cargo test -p earmark-cli --test wp09_orchestration_native
```

Manual check:

```bash
target/debug/earmark-cli commands | grep -i orchestration
```

The displayed status must not say stable.

---

# Stage 3 — Canonicalize native orchestration vocabulary

## Goal

Remove the current split between `dispatch` versus `executor_manifest`, and `evidence` versus `executor_report`. Native orchestration should use one canonical object vocabulary throughout code, declarations, docs, tests, and JSON output.

## Decision

Use the native vocabulary as canonical:

| Canonical class  | Meaning                                                                                 |
| ---------------- | --------------------------------------------------------------------------------------- |
| `work_item`      | Durable unit of intended work.                                                          |
| `context_packet` | Captured task-specific context used for dispatch.                                       |
| `dispatch`       | Executor-facing manifest / instruction packet.                                          |
| `evidence`       | Worker report, changed-file summary, verification material, or implementation evidence. |
| `gate_result`    | Recorded result of a verification gate.                                                 |
| `git_snapshot`   | Captured repository state.                                                              |
| `review`         | Human or automated review decision.                                                     |
| `closure`        | Durable closing event caused by review.                                                 |
| `followup_task`  | Task produced from rejected or incomplete work.                                         |
| `trace_event`    | Optional execution trace event.                                                         |

Do not keep `executor_manifest` or `executor_report` as active native orchestration class names. If legacy compatibility is needed, support them only as deprecated aliases at read boundaries, not as write targets.

## Required code changes

In `earmark-cli/src/app/commands/orchestration.rs`:

* Rename comments and emitted `kind` strings away from `executor_manifest_ingest` and `executor_report_ingest` unless those names are deliberately preserved as JSON compatibility aliases. Preferred new emitted kinds:

  * `orchestration_dispatch_ingest`
  * `orchestration_evidence_ingest`
* Change `resolve_manifest_for_report()` to search for class `dispatch`, not `executor_manifest`.
* Change the fallback show path to query `dispatch` and `evidence`, not `executor_manifest` and `executor_report`.
* Change `find_orchestration_task()` and list/show/timeline class sets so only canonical classes are primary.
* Add legacy alias support only in one helper if existing tests require it:

  * `canonical_orchestration_class(class: &str) -> &str`
  * Map `executor_manifest -> dispatch` and `executor_report -> evidence` for reads only.

In declaration examples under `examples/earmark-dev-orchestration/declarations/classes/`:

* Remove or deprecate class files named `executor_manifest.yaml` and `executor_report.yaml`.
* Ensure `dispatch.yaml` and `evidence.yaml` carry the fields currently required by manifest/report ingestion.
* Update `system.yaml` to register only the canonical classes unless deprecated aliases are intentionally retained in a separate compatibility section.

In documentation:

* Use `dispatch` and `evidence` consistently.
* Define `dispatch` as the durable executor-facing work packet.
* Define `evidence` as the durable report or verification material returned from execution.

## Tests

Update existing orchestration tests to assert canonical names:

* `ingest-manifest` creates class `dispatch`.
* `ingest-report` creates class `evidence`.
* `show` and `timeline` surface dispatch/evidence entries using canonical names.
* `explain-dispatch latest` resolves newest `dispatch` object.

Add a regression test for `resolve_manifest_for_report()` behavior by ingesting a task, a dispatch, then evidence without `--manifest`; the evidence must link to the correct dispatch.

## Acceptance checks

```bash
rg "executor_manifest|executor_report" earmark-cli examples docs tests
```

Allowed results only:

* a short migration note, if added;
* a compatibility alias helper, if retained;
* tests explicitly proving alias handling.

No ordinary native orchestration write path may create `executor_manifest` or `executor_report`.

---

# Stage 4 — Make Git capture explicit and non-ambient

## Goal

Ensure `em orchestration capture-git` records provenance for the intended repository, not the process current working directory by accident.

## Required CLI change

Add an optional argument:

```bash
em orchestration capture-git --repo <path>
```

Behavior:

* If `--repo` is supplied, run all Git commands with `current_dir(repo)`.
* If `--repo` is omitted, use the workspace root as the repository path only if it contains `.git` or is inside a Git worktree.
* If neither condition holds, fail with a clear error instructing the operator to pass `--repo <path>`.
* Store the resolved repository path in the `git_snapshot` payload as `repo_path`.
* Store the Git top-level path, if available, as `git_toplevel`.

## Required implementation changes

Replace `run_git_cmd(args: &[&str])` with:

```rust
fn run_git_cmd(repo: &Path, args: &[&str]) -> Result<String, CliError>
```

Implement:

```rust
fn resolve_git_repo(store_root: &Path, explicit_repo: Option<&Path>) -> Result<PathBuf, CliError>
```

Use:

```bash
git rev-parse --show-toplevel
git rev-parse HEAD
git branch --show-current
git status --porcelain
git status --short
git diff --stat
```

through the resolved repo path.

## Tests

Add tests proving:

1. `capture-git --repo <repo>` records the commit from that repo even when the CLI process current directory is elsewhere.
2. `capture-git` fails clearly when neither workspace root nor explicit repo is a Git repository.
3. `git_snapshot` payload contains `repo_path` and `git_toplevel`.

Use a temporary Git repository in the test. Configure local Git identity inside the temp repo before committing:

```bash
git config user.email test@example.invalid
git config user.name Test Operator
```

## Acceptance checks

```bash
cargo test -p earmark-cli --test wp12_orchestration_lifecycle capture_git
```

If test filtering does not match names exactly, run the whole orchestration lifecycle test file.

---

# Stage 5 — Split the orchestration command monolith

## Goal

Prevent the new orchestration surface from becoming another monolithic command file. Separate parsing, persistence, graph traversal, Git capture, review mutation, and command handlers into bounded modules.

## Target module structure

Create:

```text
earmark-cli/src/app/commands/orchestration/
  mod.rs
  adapters/
    mod.rs
    native_json.rs
  capture_git.rs
  graph.rs
  ingest.rs
  list.rs
  parse.rs
  persist.rs
  review.rs
  status.rs
  types.rs
```

Move responsibilities as follows:

| Module           | Responsibility                                                            |
| ---------------- | ------------------------------------------------------------------------- |
| `mod.rs`         | CLI action dispatch only. No parsing or persistence helpers.              |
| `capture_git.rs` | Git repo resolution and snapshot capture.                                 |
| `graph.rs`       | `traverse_orchestration_graph`, graph summaries, timeline event shaping.  |
| `ingest.rs`      | task, dispatch, evidence, and context ingestion handlers.                 |
| `list.rs`        | list/show/explain-dispatch read surfaces.                                 |
| `parse.rs`       | markdown manifest/report parsing helpers.                                 |
| `persist.rs`     | object deposit, relation creation, write batching, index update helpers.  |
| `review.rs`      | review/closure/task-standing mutation.                                    |
| `status.rs`      | normalization helpers for work item, dispatch, gate, and review statuses. |
| `types.rs`       | small shared structs/enums used across orchestration modules.             |

## Constraints

* Preserve public command names unless explicitly changed in earlier stages.
* Preserve JSON envelope shape.
* Avoid broad `pub(crate)` exports. Export only functions used by `mod.rs` or sibling modules.
* Do not move unrelated command families into this module split.

## Tests

The existing orchestration tests are the safety net. Add one small unit test for `parse.rs` or `status.rs` if convenient, but do not overbuild internal tests at the expense of behavior tests.

## Acceptance checks

```bash
wc -l earmark-cli/src/app/commands/orchestration.rs
find earmark-cli/src/app/commands/orchestration -maxdepth 2 -type f -name '*.rs' -print
```

There should no longer be a large `orchestration.rs` implementation file. The orchestration command implementation should be distributed across the module directory above.

---

# Stage 6 — Replace rebuild-heavy orchestration writes with targeted indexing

## Goal

Stop rebuilding the entire derived index after each ordinary orchestration object or relation write. Preserve explicit repair and recovery behavior. Use full rebuild only for workspace repair, system registration/activation, migration, and test setup where simplicity is intentional.

## Design decision

Introduce a small orchestration write boundary that performs canonical writes and targeted index updates in one place. Do not let handlers call `index.rebuild_from_store(store)?` after each deposit/relation.

The write boundary should live in:

```text
earmark-cli/src/app/commands/orchestration/persist.rs
```

It should expose functions such as:

```rust
pub(crate) fn deposit_orchestration_object(...) -> Result<ObjectRef, CliError>
pub(crate) fn create_orchestration_relation(...) -> Result<(), CliError>
pub(crate) fn update_orchestration_object_head(...) -> Result<VersionRef, CliError>
```

These functions should use existing lower-level write-and-index helpers wherever possible. If `RuntimeToolSurface::deposit_object` and `RuntimeToolSurface::create_relation` already perform index updates, remove the redundant rebuilds. If they do not, add or call targeted index methods rather than rebuilding globally.

## Required investigation

Before editing, inspect:

```text
earmark-runtime-tools/src/modules/deposit.rs
earmark-runtime-tools/src/modules/relations.rs
earmark-exec/src/persistence_helpers.rs
earmark-index/src/lib.rs
earmark-store/src/authorization.rs
earmark-store/src/lib.rs
```

Determine whether these lower layers already index:

* new object versions;
* relation objects;
* relation adjacency;
* standing/head changes;
* undo-hidden objects and relations.

Document the conclusion in a short code comment in `persist.rs` near the write boundary.

## Required implementation changes

Remove ordinary `index.rebuild_from_store(store)?` calls from orchestration handlers after:

* task ingestion;
* dispatch ingestion;
* evidence ingestion;
* context packet recording;
* Git snapshot capture;
* gate result recording;
* review object creation;
* closure object creation;
* relation creation;
* task head update.

Keep full rebuilds only where semantically justified:

* `orchestration init-example`, after registering and activating the example system;
* explicit `em doctor --repair-index`;
* test setup if needed;
* any future migration path.

Add a debug-only or test-only counter if practical to prove ordinary orchestration commands do not call full rebuild. If that is invasive, add regression tests that exercise a full orchestration lifecycle after temporarily corrupting no state and verify all objects remain queryable without explicit rebuild.

## Tests

Add or update tests proving:

1. A task ingested through native orchestration is immediately queryable.
2. A dispatch linked to a task is immediately visible through `show` and `timeline`.
3. A gate result linked to a dispatch or task is immediately visible through `show` and `timeline`.
4. A review updates task standing/status and remains queryable.
5. `em doctor --repair-index` still rebuilds successfully after intentional index deletion/corruption.

## Acceptance checks

```bash
rg "rebuild_from_store" earmark-cli/src/app/commands/orchestration earmark-runtime-tools earmark-exec earmark-index
```

Allowed orchestration results only:

* `init-example` system activation path;
* explicit repair/migration code;
* tests that intentionally exercise repair.

No ordinary native orchestration write path may rebuild the full index.

---

# Final review checklist

Before PR:

```bash
git status --short
rg "executor_manifest|executor_report" earmmark-cli examples docs tests || true
rg "\[STABLE\].*orchestration|CommandStability::Stable.*Orchestration" earmark-cli || true
rg "rebuild_from_store" earmark-cli/src/app/commands/orchestration
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Correct the typo in the second command path if copied: use `earmark-cli`, not `earmmark-cli`.

## PR summary template

````markdown
## Summary

Cleans up the post-refactor `dev` branch so the native orchestration surface is explicit, bounded, and release-clean.

## Changes

- Removes tracked local-runtime residue and resolves `.gitignore` contradictions.
- Marks native orchestration experimental in the CLI catalog and documentation.
- Canonicalizes native orchestration vocabulary around work items, dispatches, evidence, gates, reviews, closures, snapshots, and context packets.
- Makes Git provenance capture explicit through a resolved repository path.
- Splits the orchestration command implementation into bounded modules.
- Replaces full-index rebuilds after ordinary orchestration writes with targeted write/index updates while preserving explicit repair.

## Verification

```bash
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
````

## Compatibility

Native orchestration remains experimental. Public command availability is preserved, but internal class vocabulary is normalized around native class names.

```
```
