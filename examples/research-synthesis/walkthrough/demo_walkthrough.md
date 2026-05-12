# Research Synthesis Demo Walkthrough

This walkthrough runs the research synthesis demo from a clean external workspace. The workflow turns source notes into findings, then turns those findings into a summary. The summary stage receives findings only, not the original notes.

## 0. Prepare the CLI and Workspace

Run these commands from the repository root:

```bash
cargo build -p earmark-cli
alias em="$(pwd)/target/debug/earmark-cli"
export REPO_ROOT="$(pwd)"
export WORKSPACE=/tmp/earmark-research-synthesis-demo
rm -rf "$WORKSPACE"
```

## 1. Initialize and Activate the Demo System

```bash
em --root "$WORKSPACE" init
em --root "$WORKSPACE" declare validate --kind system "$REPO_ROOT/examples/research-synthesis/declarations/systems/system.yaml"
em --root "$WORKSPACE" system register "$REPO_ROOT/examples/research-synthesis/declarations/systems/system.yaml"
em --root "$WORKSPACE" system activate sys_research_synthesis
```

Expected registration output includes a new declaration object and version:

```json
{
  "ok": true,
  "action": "register",
  "kind": "system",
  "object_id": "obj_...",
  "version_id": "ver_..."
}
```

Object and version IDs differ on each run.

## 2. Deposit Seed Notes

```bash
em --root "$WORKSPACE" deposit \
  --class source_note \
  --title "Federated Graphs: Agility and Ownership" \
  --payload-file "$REPO_ROOT/examples/research-synthesis/data/seed_notes/note_1_benefits.md"

em --root "$WORKSPACE" deposit \
  --class source_note \
  --title "The Cost of Heterogeneity" \
  --payload-file "$REPO_ROOT/examples/research-synthesis/data/seed_notes/note_2_challenges.md"
```

Each deposit returns an object ID and version ID:

```json
{
  "ok": true,
  "class": "source_note",
  "kind": "object",
  "object_id": "obj_...",
  "version_id": "ver_...",
  "title": "Federated Graphs: Agility and Ownership"
}
```

## 3. Run the Workflow

List the deposited notes:

```bash
em --root "$WORKSPACE" query --class source_note
```

Use the returned object IDs as workflow inputs:

```bash
em --root "$WORKSPACE" workflow run research_synthesis \
  --system-id sys_research_synthesis \
  --with <ID_1> \
  --with <ID_2>
```

Expected output:

```json
{
  "ok": true,
  "run_id": "run_...",
  "summary": "workflow run completed",
  "status": "completed",
  "event_count": 5,
  "packet_count": 4,
  "output_count": 2,
  "governance_event_count": 2,
  "created_assignments": ["obj_..."],
  "created_change_sets": ["obj_..."],
  "created_handoffs": ["obj_..."],
  "created_failures": []
}
```

Counts may differ as the workflow evolves. The important signal is `status: "completed"` and an empty `created_failures` list.

## 4. Inspect the Produced Objects

```bash
em --root "$WORKSPACE" query --class finding
em --root "$WORKSPACE" query --class summary
```

The workflow creates findings first. The summary is produced from those findings, not from the raw source notes.

## 5. Inspect the Run

```bash
em --root "$WORKSPACE" run explain latest
em --root "$WORKSPACE" run timeline latest
em --root "$WORKSPACE" run artifacts latest
em --root "$WORKSPACE" run graph latest
```

These commands show the assignment lifecycle, change sets, handoffs, failures if any occurred, and the relationship graph connecting source notes, findings, and summary.

## 6. Inspect a Handoff

List run artifacts, then choose one handoff ID:

```bash
em --root "$WORKSPACE" run artifacts latest
em --root "$WORKSPACE" handoff explain <HANDOFF_ID>
```

The extraction-stage handoff carries findings forward. It does not carry the original source notes into the summary stage.

## 7. Generate a Report

```bash
em --root "$WORKSPACE" report run latest --output "$WORKSPACE/reports/synthesis_run.html"
```

Open the generated HTML file in a browser to inspect the run as a static report.

## 8. If a Run Fails

Failed work is preserved as workspace state. If `created_failures` is not empty, inspect the failure instead of rerunning immediately:

```bash
em --root "$WORKSPACE" failure list --run-id latest
em --root "$WORKSPACE" failure explain <FAILURE_ID>
em --root "$WORKSPACE" run timeline latest
em --root "$WORKSPACE" report run latest --output "$WORKSPACE/reports/failed_run.html"
```

A failure record links the failed assignment, the failed change set if one was produced, the error type, and the input objects active at the time of failure. That record remains available for audit after later recovery work.
