# Knowledge Briefing Demo

This demo shows how a small body of expert material can be converted into a source-backed briefing artifact through staged, inspectable AI work.

## The Problem

Knowledge management breaks down long before document storage does. Teams rarely lose access to files, but they routinely lose the discipline of turning notes into reusable claims. Instead, context travels as ambient memory: forwarded threads, chat fragments, and informal summaries. This demo adds an explicit extraction and handoff step so claims become reusable objects instead of being re-derived each time.

## What This Demo Does

The pipeline runs in two stages: `source_note -> finding -> briefing_card`. First, raw notes are converted into atomic findings with provenance and uncertainty flags where needed. Second, only those findings are admitted into a bounded synthesis stage that produces a structured briefing card. The runtime never receives the whole corpus. Each stage receives only its declared work surface.

## The Seed Corpus

The seed corpus contains five short notes in mixed quality and register: workshop observations, a quantitative report excerpt, an internal pilot note, a stakeholder concern, and one ambiguous/outdated note. This mix is deliberate so the extraction step has to preserve signal while surfacing uncertainty.

## Prerequisites

This demo assumes you have built the Earmark CLI and aliased it as `em`. From the repository root:

```bash
cargo build -p earmark-cli
alias em="$(pwd)/target/debug/earmark-cli"
export REPO_ROOT="$(pwd)"
```

## Running the Demo

This example is designed to run against an external workspace, not from inside the example directory itself. The workspace below lives outside the repository so Earmark can manage its own store state cleanly.

```bash
export WORKSPACE=/tmp/earmark-knowledge-briefing-demo
rm -rf "$WORKSPACE"

# Initialize the workspace
em --root "$WORKSPACE" init

# Validate declarations
em --root "$WORKSPACE" declare validate --kind system "$REPO_ROOT/examples/knowledge-briefing/declarations/systems/system.yaml"

# Register declarations for this demo
em --root "$WORKSPACE" system register "$REPO_ROOT/examples/knowledge-briefing/declarations/systems/system.yaml"

# Activate the system
em --root "$WORKSPACE" system activate sys_knowledge_briefing

# Deposit source notes
em --root "$WORKSPACE" deposit --class source_note --title "Workshop Notes" --payload-file "$REPO_ROOT/examples/knowledge-briefing/seed/note_1_workshop.md"
em --root "$WORKSPACE" deposit --class source_note --title "Federal Progress Excerpt" --payload-file "$REPO_ROOT/examples/knowledge-briefing/seed/note_2_report_excerpt.md"
em --root "$WORKSPACE" deposit --class source_note --title "Pilot Integration Note" --payload-file "$REPO_ROOT/examples/knowledge-briefing/seed/note_3_project_note.md"
em --root "$WORKSPACE" deposit --class source_note --title "Stakeholder Concern" --payload-file "$REPO_ROOT/examples/knowledge-briefing/seed/note_4_stakeholder.md"
em --root "$WORKSPACE" deposit --class source_note --title "Ambiguous District Heating Note" --payload-file "$REPO_ROOT/examples/knowledge-briefing/seed/note_5_ambiguous.md"

# Review the deposited source notes and choose the IDs you want to process
em --root "$WORKSPACE" query --class source_note

# Run the governed pipeline with explicit inputs
em --root "$WORKSPACE" workflow run knowledge_briefing --system-id sys_knowledge_briefing --with <ID_1> --with <ID_2> --with <ID_3> --with <ID_4> --with <ID_5>

# Inspect resulting objects and lineage
em --root "$WORKSPACE" query --class finding
em --root "$WORKSPACE" query --class briefing_card
em --root "$WORKSPACE" run explain latest
```

## Inspecting the Output

Check the generated `briefing_card` object and compare it with `expected-output/briefing_card.md`. Also inspect workflow steps and lineage from briefing card back to findings. The key observation is that synthesis references findings, not raw source notes.

## What This Demonstrates

- Bounded context: each stage sees only what it is declared to see
- Durable findings: extracted claims persist as objects in the store
- Governed handoff: the briefing stage continues from findings only
- Failure visibility: uncertain material is flagged, not silently included
- Inspectable lineage: every finding traces back to its source note

## About Earmark

Earmark is a declaration-first runtime for governed AI execution and durable knowledge objects. This demo illustrates how Earmark turns ambient note-taking into inspectable, staged knowledge work.
