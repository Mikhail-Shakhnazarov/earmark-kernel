# Quickstart

Get a working Earmark run in under 5 minutes using the built-in research synthesis demo.

## Build the CLI

```bash
REPO_ROOT=$(pwd)
cargo build -p earmark-cli
alias em="$REPO_ROOT/target/debug/earmark-cli"
```

## Initialize a workspace

```bash
mkdir my-workspace && cd my-workspace
em init
```

You should see:

```json
{
  "next_commands": [
    "em doctor",
    "em status",
    "em declare list-examples"
  ],
  "ok": true,
  "paths": {
    "canonical_dir": "/path/to/my-workspace/.earmark/canonical",
    "declarations_dir": "/path/to/my-workspace/.earmark/declarations",
    "index_path": "/path/to/my-workspace/.earmark/derived/index.sqlite",
    "work_surfaces_dir": "/path/to/my-workspace/.earmark/work_surfaces"
  },
  "root": "/path/to/my-workspace",
  "summary": "workspace initialized"
}
```

## Register a system

Use the example system manifest from the repository:

```bash
em system register "$REPO_ROOT/examples/research-synthesis/declarations/systems/system.yaml"
em system activate sys_research_synthesis
```

Expected output for registration:

```json
{
  "ok": true,
  "kind": "system_registration",
  "object_id": "obj_...",
  "version_id": "ver_..."
}
```

Object and version IDs will differ on each run. This registers the research synthesis domain — three object classes (`source_note`, `finding`, `summary`), two instructions, and one two-stage workflow.

## Deposit some data

Put a few source notes into the corpus:

```bash
em deposit --class source_note --title "Context Boundaries" --body "AI context should be bounded, not ambient."
em deposit --class source_note --title "Lineage" --body "Every derived object should trace back to its source."
```

> [!NOTE]
> Because you activated `sys_research_synthesis`, these deposits are validated against the system's admitted class list. If you tried to deposit a class not in the system definition, the command would fail.

Expected output for each deposit:

```json
{
  "ok": true,
  "class": "source_note",
  "kind": "object",
  "object_id": "obj_...",
  "version_id": "ver_...",
  "title": "Context Boundaries"
}
```

## Run the workflow

Find your deposited objects:

```bash
em query --class source_note
```

Expected output (snippet):

```json
[
  {
    "object_id": "obj_...",
    "class": "source_note",
    "kind": "object",
    "title": "Context Boundaries",
    "summary": "AI context should be bounded, not ambient.",
    "standing_epistemic": "working",
    "standing_process": "active",
    "standing_review": "unreviewed",
    "version_id": "ver_..."
  }
]
```

Pick an object ID from the output and run the workflow:

```bash
em workflow run research_synthesis --system-id sys_research_synthesis --with <object_id>
```

Expected output:

```json
{
  "ok": true,
  "run_id": "run_...",
  "summary": "workflow run completed",
  "status": "completed",
  "created_assignments": ["obj_...", "obj_...", "obj_...", "obj_..."],
  "created_change_sets": ["obj_...", "obj_...", "obj_...", "obj_..."],
  "created_failures": [],
  "created_handoffs": ["obj_...", "obj_...", "obj_...", "obj_..."],
  "event_count": 5,
  "output_count": 2,
  "packet_count": 4
}
```

## Inspect the results

```bash
# What happened in the run
em run explain latest

# Visual timeline of events
em run timeline latest

# Generate an HTML report you can open in a browser
em report run latest --output report.html
```

`em run explain latest` will show:

```text
--- RUN Explanation: run_... ---

Summary: run run_... is completed

Purpose: A run records the execution of a workflow system.
Status: completed
Started At: 2026-05-10T...
Ended At: 2026-05-10T...

Related Artifacts:
  Assignments: 4
  Change Sets: 4
  Handoffs: 4
  Failures: 0
```

## What just happened?

You ran a two-stage workflow:

1. **Extraction**: Earmark compiled a bounded work surface containing your source notes, then extracted findings. Each finding was linked to its source through a `derived_from` relation.

2. **Synthesis**: Earmark emitted a handoff from Stage 1 containing only the findings — not the original source notes. Stage 2 produced a summary from that bounded input.

The key thing: Stage 2 never saw the raw source notes. It worked from the handoff. That's bounded continuation.

## Next steps

- [Research Synthesis Demo](research-synthesis-demo.md) — deeper walkthrough of staged execution
- [Context Compilation](../concepts/context-compilation.md) — how Earmark decides what a runtime sees
- [Build a Domain Definition](build-a-domain-definition.md) — define your own classes and workflows
