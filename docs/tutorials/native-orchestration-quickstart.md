# Native Orchestration Quickstart

Run a complete orchestration lifecycle — from task to review — in one walkthrough.

## Problem

A single orchestrator should coordinate a long-running AI work item without losing the trail: what was asked, what context was given, what the worker produced, what code state applied, what gates passed, and whether the result was accepted.

## 1. Initialize

Build the CLI and create a workspace:

```bash
REPO_ROOT=$(pwd)
cargo build -p earmark-cli
alias em="$REPO_ROOT/target/debug/earmark-cli"

mkdir orch-demo && cd orch-demo
em init
```

Register the orchestration example domain:

```bash
em orchestration init-example
```

Expected output (IDs will differ):

```json
{
  "kind": "orchestration_example_init",
  "system_id": "sys_earmark_dev_orchestration",
  "namespace": "examples.earmark-dev",
  "class_count": 15,
  "workflow_count": 1,
  "activation_status": "active"
}
```

This registers 15 classes (`work_item`, `dispatch`, `context_packet`, `gate_result`, `git_snapshot`, `evidence`, `review`, `closure`, etc.) and one workflow.

## 2. Create a Work Item

Ingest an ad-hoc task:

```bash
em orchestration ingest-task pf-s1 \
  --title "Public Facelift" \
  --description "Update the landing page hero section" \
  --priority high \
  --status proposed
```

Expected output shape:

```json
{
  "kind": "work_item_ingest",
  "source": "native-json",
  "tasks": [
    {
      "object_id": "obj_...",
      "task_id": "pf-s1",
      "title": "Public Facelift",
      "status": "proposed",
      "priority": "high"
    }
  ]
}
```

You can also ingest from a JSON file:

```bash
cat > task.json << 'EOF'
{
  "task_id": "pf-s1",
  "title": "Public Facelift",
  "goal": "Update the landing page hero section",
  "priority": "high",
  "status": "proposed"
}
EOF

em orchestration ingest-task --source native-json task.json
```

Copy the `object_id` from the output — you will need it for subsequent commands.

## 3. Record Context

Attach a context packet — the instructions and data handed to the worker:

```bash
cat > context.json << 'EOF'
{
  "files": ["src/landing/hero.tsx", "src/landing/hero.css"],
  "design_ref": "figma://landing/hero-v2",
  "constraints": ["max 60fps animation", "accessibility AA compliance"]
}
EOF

em orchestration record-context --task-id pf-s1 context.json
```

Expected output shape:

```json
{
  "kind": "orchestration_context_packet",
  "task_id": "pf-s1",
  "object_id": "obj_...",
  "version_id": "ver_..."
}
```

The context packet is now linked to the work item via a `has_context` relation.

## 4. Create a Dispatch

A dispatch represents a specific worker attempt. Use `ingest-manifest` to parse a markdown manifest into a dispatch object:

```bash
cat > manifest-pf-s1.md << 'EOF'
---
task_uuid: pf-s1
attempt_number: 1
---

## Objective

Update the landing page hero section per the design spec.

## Local Gates

- `npm run lint` must pass
- `npm test` must pass
- Bundle size increase must be under 5KB

## Target Files

- src/landing/hero.tsx
- src/landing/hero.css
EOF

em orchestration ingest-manifest manifest-pf-s1.md --task-id pf-s1
```

Expected output shape:

```json
{
  "kind": "dispatch_ingest",
  "object_id": "obj_...",
  "version_id": "ver_...",
  "task_id": "pf-s1",
  "attempt": 1,
  "objective": "Update the landing page hero section per the design spec.",
  "local_gates": [
    "npm run lint must pass",
    "npm test must pass",
    "Bundle size increase must be under 5KB"
  ],
  "target_files": ["src/landing/hero.tsx", "src/landing/hero.css"]
}
```

The dispatch is linked to the work item via a `dispatched_as` relation.

## 5. Capture Git State

Record the code state before the worker began:

```bash
em orchestration capture-git \
  --task-id pf-s1 \
  --phase pre-dispatch \
  --commit demo-pre-dispatch
```

Expected output shape:

```json
{
  "kind": "orchestration_git_snapshot",
  "task_id": "pf-s1",
  "phase": "pre-dispatch",
  "commit": "demo-pre-dispatch",
  "dirty": false
}
```

This deposits a `git_snapshot` object and links it to the work item.

> [!TIP]
> In a real project repository, omit `--commit` to let Earmark read the current Git `HEAD`, or pass an explicit commit hash.

## 6. Record a Gate Result

After the worker runs, record a verification check:

```bash
cat > test-output.log << 'EOF'
Running lint...
PASS src/landing/hero.tsx
PASS src/landing/hero.css
All lint checks passed.

Running tests...
PASS tests/landing.test.tsx
All 12 tests passed.
EOF

em orchestration record-gate \
  --task-id pf-s1 \
  --command "npm run lint && npm test" \
  --status passed \
  --log test-output.log
```

Expected output shape:

```json
{
  "kind": "orchestration_gate_result",
  "task_id": "pf-s1",
  "command": "npm run lint && npm test",
  "status": "pass"
}
```

Status normalization: `passed`, `success`, `ok` → `pass`; `failed`, `error` → `fail`; `skip` → `skipped`.

## 7. Ingest Evidence

Record the output the worker produced:

```bash
cat > report-pf-s1.md << 'EOF'
---
task_uuid: pf-s1
attempt_number: 1
---

## Files Changed

- src/landing/hero.tsx
- src/landing/hero.css

## Summary

Updated hero section with new animation and accessibility improvements.
EOF

em orchestration ingest-report report-pf-s1.md --task-id pf-s1 --manifest obj_<dispatch_id>
```

Replace `<dispatch_id>` with the `object_id` from the manifest ingest step. Expected output shape:

```json
{
  "kind": "evidence_ingest",
  "object_id": "obj_...",
  "version_id": "ver_...",
  "task_id": "pf-s1",
  "attempt": 1,
  "files_changed": ["src/landing/hero.tsx", "src/landing/hero.css"]
}
```

The evidence is linked to the dispatch via a `produced_evidence` relation.

## 8. Inspect

View the full task detail:

```bash
em orchestration show pf-s1
```

Expected output shape includes all linked objects:

```json
{
  "kind": "orchestration_work_item_show",
  "work_item_id": "obj_...",
  "title": "Public Facelift",
  "status": "proposed",
  "priority": "high",
  "context_packets": [...],
  "dispatches": [...],
  "evidence": [...],
  "git_snapshots": [{ "phase": "pre-dispatch", "commit": "abc123..." }],
  "gate_results": [{ "command": "npm run lint && npm test", "status": "pass" }],
  "reviews": [],
  "closures": []
}
```

View the chronological timeline:

```bash
em orchestration timeline pf-s1
```

```json
{
  "kind": "orchestration_timeline",
  "events": [
    { "class": "work_item", "title": "Public Facelift", ... },
    { "class": "context_packet", ... },
    { "class": "dispatch", ... },
    { "class": "git_snapshot", ... },
    { "class": "gate_result", ... },
    { "class": "evidence", ... }
  ]
}
```

Explain a specific dispatch's lifecycle:

```bash
em orchestration explain-dispatch obj_<dispatch_id>
```

## 9. Review

Close the loop with a review decision:

```bash
em orchestration review pf-s1 \
  --decision accepted \
  --comment "All gates pass, hero section looks correct."
```

Expected output shape:

```json
{
  "kind": "orchestration_review_decision",
  "task_id": "pf-s1",
  "decision": "accepted",
  "comment": "All gates pass, hero section looks correct.",
  "task_object_id": "obj_...",
  "task_new_version_id": "ver_..."
}
```

Review decisions:
- **`accepted`** — task standing becomes `closed` / `accepted`, status → `implemented`
- **`rejected`** — task standing becomes `closed` / `rejected`, status → `closed`
- **`needs_revision`** — task standing becomes `proposed` / `needs_revision`, status → `proposed`

The `review` and `closure` objects are deposited and linked. Verifying with `show` again:

```bash
em orchestration show pf-s1
```

The `reviews` and `closures` arrays will now be populated.

## Harmonized Status Language

The orchestration surface uses several status layers. This table maps public-facing states to the underlying orchestration records:

| Public state   | Orchestration record                              | Meaning                                       |
| -------------- | ------------------------------------------------- | --------------------------------------------- |
| Proposed       | `work_item.status = proposed`                     | Work exists but is not dispatched.            |
| Ready          | `work_item.status = ready`                        | Work has enough context to send to a worker.  |
| Dispatched     | `dispatch.status = queued/running`                | A worker attempt exists.                      |
| Under review   | `work_item.status = under_review`                 | Output needs judgment.                        |
| Accepted       | review decision `accepted`; closure exists        | Operator accepted the work.                   |
| Needs revision | review decision `needs_revision`                  | Work continues with follow-up.                |
| Rejected       | review decision `rejected`; closure exists        | Work is preserved but not accepted.           |
| Failed         | dispatch/gate/evidence indicates failure          | Execution failed or gates failed.             |

## What Just Happened?

The orchestration ledger recorded every step:

```text
work_item → context_packet → dispatch → git_snapshot → gate_result → evidence → review → closure
```

Each phase produced a durable object with relations to its neighbors. The operator review is visible as a distinct state transition — the task standing moved from `active`/`unreviewed` to `closed`/`accepted`.

## Summary of Commands

| Step | Command | Object Created |
|------|---------|----------------|
| Initialize | `em orchestration init-example` | System registration |
| Create work item | `em orchestration ingest-task` | `work_item` |
| Record context | `em orchestration record-context` | `context_packet` |
| Create dispatch | `em orchestration ingest-manifest` | `dispatch` |
| Capture git state | `em orchestration capture-git` | `git_snapshot` |
| Record gate | `em orchestration record-gate` | `gate_result` |
| Ingest evidence | `em orchestration ingest-report` | `evidence` |
| Inspect | `em orchestration show / timeline / explain-dispatch` | — |
| Review | `em orchestration review` | `review` + `closure` |
