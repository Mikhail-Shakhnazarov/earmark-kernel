# Research Synthesis Demo

This tutorial walks through a complete two-stage workflow: extracting findings from source notes, then synthesizing a summary from only those findings.

The key demonstration: the summarization stage never sees the original source notes. It works from a bounded handoff containing only the extracted findings.

## The Workflow

```mermaid
flowchart LR
    SN[Source Notes] -->|Stage 1: Extraction| F[Findings]
    F -->|Handoff| S[Summary]
```

## Setup

If you've already completed the [Quickstart](quickstart.md), you have a workspace with `sys_research_synthesis` registered and activated.

Otherwise, from the repository root:

```bash
cargo build -p earmark-cli
alias em="$(pwd)/target/debug/earmark-cli"
export REPO_ROOT="$(pwd)"
export WORKSPACE=/tmp/earmark-research-synthesis-tutorial
rm -rf "$WORKSPACE"

em --root "$WORKSPACE" init
em --root "$WORKSPACE" system register "$REPO_ROOT/examples/research-synthesis/declarations/systems/system.yaml"
em --root "$WORKSPACE" system activate sys_research_synthesis
```

Deposit the seed notes:

```bash
em --root "$WORKSPACE" deposit --class source_note \
  --title "Federated Graphs: Agility and Ownership" \
  --payload-file "$REPO_ROOT/examples/research-synthesis/data/seed_notes/note_1_benefits.md"

em --root "$WORKSPACE" deposit --class source_note \
  --title "The Cost of Heterogeneity" \
  --payload-file "$REPO_ROOT/examples/research-synthesis/data/seed_notes/note_2_challenges.md"
```

## Stage 1: Finding Extraction

Run the workflow with your deposited source notes:

```bash
em --root "$WORKSPACE" query --class source_note
em --root "$WORKSPACE" workflow run research_synthesis --system-id sys_research_synthesis --with <source_note_id>
```

What happens:

1. The engine compiles a work packet containing only the `source_note` objects (as declared by the `source_notes_for_extraction` compiled context template).
2. The runtime processes the notes and produces `finding` objects.
3. An **assignment**, a **change set**, and a **handoff** are created and persisted.

The output will include a `handoff_id`. This is the bridge to Stage 2.

Inspect it:

```bash
em --root "$WORKSPACE" handoff explain <handoff_id>
```

You'll see which objects the handoff carries forward (findings) and which it excludes (the original source notes).

## Stage 2: Summarization

Continue work using the handoff from Stage 1:

```bash
em --root "$WORKSPACE" workflow run research_synthesis --system-id sys_research_synthesis --handoff <handoff_id>
```

> **This is the core idea.** The summarization step receives findings, not source notes. It cannot access the original raw material. If a finding is wrong, the fix is to re-run Stage 1, not to give Stage 2 more context.

## Inspecting the Full Run

After both stages complete:

```bash
# Summary of what happened and suggested next steps
em --root "$WORKSPACE" run explain <run_id>

# Visual timeline of every event
em --root "$WORKSPACE" run timeline <run_id>

# Relationship graph showing lineage from source to finding to summary
em --root "$WORKSPACE" run graph <run_id>

# HTML report you can share
em --root "$WORKSPACE" report run <run_id> --output research_report.html
```

## What This Demonstrates

**Bounded context**: Each stage sees only what its compiled context template declares. The summarizer works from findings, not from the full corpus.

**Durable findings**: Extracted claims persist as objects in the store. They can be queried, inspected, and reused independently of the run that created them.

**Governed handoff**: The handoff explicitly defines what the next stage may see. There is no implicit context leaking between stages.

**Inspectable lineage**: Every finding has a `derived_from` relation linking it to the source note it came from. The summary links back to findings. The full chain is traversable.

## Next Steps

- [Build a Domain Definition](build-a-domain-definition.md) — define your own classes and workflows
- [Context Compilation](../concepts/context-compilation.md) — how compiled context templates control what each stage sees
