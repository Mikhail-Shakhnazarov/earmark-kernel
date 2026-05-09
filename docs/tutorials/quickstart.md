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
  "ok": true,
  "summary": "workspace initialized",
  "root": ".",
  "paths": {
    "canonical_dir": ".\\.earmark\\canonical",
    "declarations_dir": ".\\.earmark\\declarations",
    "work_surfaces_dir": ".\\.earmark\\work_surfaces",
    "index_path": ".\\.earmark\\derived\\index.sqlite"
  },
  "next_commands": [
    "em doctor",
    "em status",
    "em declare validate --kind class docs/declarations/examples/classes/finding.yaml"
  ]
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
  "object_id": "obj_c5ef57bdcbcf4be5a67ec0b467b75012",
  "version_id": "ver_c7ba974ad96047ecb39b914207600ac4"
}
```

This registers the research synthesis domain — three object classes (`source_note`, `finding`, `summary`), two instructions, and one two-stage workflow.

## Deposit some data

Put a few source notes into the corpus:

```bash
em deposit --class source_note --title "Context Boundaries" --body "AI context should be bounded, not ambient."
em deposit --class source_note --title "Lineage" --body "Every derived object should trace back to its source."
```

Expected output for each deposit:

```json
{
  "ok": true,
  "class": "source_note",
  "object_id": "obj_92aef3ed1ade41fea3a47019cc734181",
  "version_id": "ver_a04062dba5394bb38ea533469ec7df8b",
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
    "object_id": "obj_92aef3ed1ade41fea3a47019cc734181",
    "class": "source_note",
    "title": "Context Boundaries",
    "standing_process": "active",
    "standing_review": "unreviewed"
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
  "run_id": "run_1778353534174187900",
  "summary": "workflow run completed",
  "status": "completed",
  "created_assignments": ["obj_...", "obj_..."],
  "created_change_sets": ["obj_...", "obj_..."],
  "created_handoffs": ["obj_...", "obj_..."]
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
--- RUN Explanation: run_1778353534174187900 ---

Summary: run run_1778353534174187900 is completed
Status: completed
Started At: 2026-05-09T19:05:34.178213100Z
Ended At: 2026-05-09T19:05:40.728925100Z

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
