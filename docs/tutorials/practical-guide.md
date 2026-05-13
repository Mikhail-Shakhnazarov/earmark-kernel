# Earmark: A Practical Guide

For AI-assisted work that needs to be inspectable, resumable, and reviewable.

## The Problem

Most AI work happens in an ambient state. A person opens a chat, pastes some material, asks for a result, and keeps going. That is useful for quick tasks. It is weak for work that needs to survive beyond the moment.

The weakness appears when ordinary accountability questions arise:

* What exactly did the model see?
* Which source produced this claim?
* Was this output reviewed, or is it still a draft?
* What failed, and where?
* Can the next step continue without inheriting the whole previous conversation?
* Can another person inspect the work without replaying a chat transcript?

In normal AI workflows, the answer is often: maybe, if the chat history is still available and someone remembers what happened.

Earmark is built for a different answer.

## The Basic Idea

Earmark turns AI-assisted work into a sequence of visible steps. Each step receives a declared input, produces a durable output, records what happened, and passes the right material to the next step.

Each step is declared work: scoped inputs, recorded outputs, and explicit handoff to the next stage.

Each stage sees only the objects assigned to it. Each result is recorded as an artifact. Failures remain in the workspace as inspectable records. Later steps continue from handoffs rather than from ambient conversation history.

AI work becomes something that can be resumed, reviewed, explained, and governed.

## A Concrete Example: Research Notes to Summary

The main example in the repository is a small research synthesis workflow.

Imagine a folder of research notes. The goal is to turn those notes into a summary. A normal AI workflow might paste all the notes into a model and ask for a briefing. The result may be good, but the process is hard to inspect. Claims, sources, intermediate judgments, and failures are all mixed together.

Earmark breaks the work into stages:

```text
source notes → findings → summary
```

### Stage 1: Extract findings

The first stage receives the source notes. It extracts findings from them.

A finding becomes a stored object: queryable, linkable, reviewable, and reusable. It can link back to the note it came from. It can be accepted, revised, rejected, or used by later work.

### Stage 2: Write the summary

The second stage receives the extracted findings as its working set. The raw notes remain behind the extraction stage.

The summary step works from the handed-off findings. If a finding is wrong, the repair belongs in the extraction step: fix the finding or rerun the stage that produced it.

The summary becomes the visible result of staged work.

## What Earmark Adds

Earmark adds structure around work that would otherwise live in a chat window.

| Ordinary AI workflow                       | Earmark workflow                              |
| ------------------------------------------ | --------------------------------------------- |
| Paste context into a chat                  | Deposit objects into a workspace              |
| Ask for an output                          | Run a declared workflow                       |
| Hope the model used the right material     | Compile the allowed context for each stage    |
| Treat intermediate results as temporary    | Store intermediate results as durable objects |
| Let the next step inherit the conversation | Pass a bounded handoff to the next stage      |
| Lose failures in logs or memory            | Preserve failures as inspectable records      |
| Review socially, outside the system        | Record review state as part of the work       |

Legibility makes checking possible. Correctness still comes from good sources, good declarations, validation, and review.

## The Main Pieces

Earmark uses a small working vocabulary.

**Object** means a piece of work stored in the workspace: a note, a finding, a summary, a review decision, or another declared type.

**Class** means the kind of object: `source_note`, `finding`, `summary`, and so on.

**Relation** means a visible link between objects. In the demo, findings are linked back to source notes through a `derived_from` relation.

**Workflow** means the sequence of stages. In the demo, the workflow extracts findings and then summarizes them.

**Handoff** means the package passed from one stage to the next. It defines what the next stage is allowed to see.

**Run** means one execution of a workflow.

**Report** means an inspectable view of what happened in a run.

## What Can Be Inspected After a Run

A workflow run produces more than an answer. It produces a record of what happened.

Earmark records:

* which inputs were used;
* which stage claimed the work;
* which objects were created;
* which links connect outputs back to sources;
* which handoff was created for the next stage;
* whether validation passed;
* what failed, if anything failed;
* what can be resumed from the current state.

The process becomes visible enough to govern.

## Trying the Demo

Build the CLI from the repository root, then run the demo in a fresh workspace:

```bash
cargo build -p earmark-cli
alias em="$(pwd)/target/debug/earmark-cli"

mkdir my-workspace && cd my-workspace
em init

em system register ../examples/research-synthesis/declarations/systems/system.yaml
em system activate sys_research_synthesis

em deposit --class source_note --title "Context Boundaries" --body "AI context should be bounded, not ambient."
em deposit --class source_note --title "Lineage" --body "Every derived object should trace back to its source."

em query --class source_note
em workflow run research_synthesis --system-id sys_research_synthesis --with <object_id>
```

Then inspect the run:

```bash
em run explain latest
em run timeline latest
em report run latest --output report.html
```

The HTML report shows the generated answer together with the record of the work that produced it.

## Path Through the Repository

Start with the working flow before moving into declarations and code.

1. **README** — purpose, status, and the basic problem.
2. **Quickstart** — the shortest working path through the demo.
3. **Research Synthesis Demo** — the staged workflow in more detail.
4. **Staged Execution** — how each stage leaves durable evidence.
5. **Context Compilation** — how Earmark decides what a runtime is allowed to see.
6. **Standing** — how domain lifecycle state is declared and enforced.
7. **Declaration Authoring** — how classes, workflows, instructions, and systems are declared.

The center is the demo: notes become findings, findings become a summary, and each move leaves a trail.

## Adapting the Pattern

The research synthesis demo is a small instance of a more general pattern:

```text
raw material → intermediate objects → reviewed output
```

That pattern can apply wherever work should move through controlled stages:

* interview notes → findings → research brief;
* incident reports → facts → postmortem draft;
* policy documents → obligations → compliance checklist;
* meeting notes → decisions → implementation tasks;
* manuscript comments → revisions → editorial summary.

Raw material becomes intermediate objects. Intermediate objects become higher-level outputs. Every step keeps its lineage.

Start with two or three object types, one relation, and one two-stage workflow. A clear small workflow can grow. A vague workflow gains little from extra declarations.

## Where Earmark Fits

Earmark fits work where the process matters as much as the output.

It is useful when work needs provenance, review, staged continuation, repeatability, failure inspection, or handoff between people or runtimes. It is especially relevant when a result may be challenged later and the system needs to show how that result came to exist.

For casual prompting, one-off summaries, private scratch notes, or tasks where the process will never be inspected, a lighter tool is usually enough.

A simple test: if losing the chat transcript would make the work hard to trust, resume, or explain, Earmark may be relevant.

## Current Capabilities

The repository demonstrates the governed execution layer end to end.

It can initialize a workspace, register a declared system, deposit objects, run staged workflows, create handoffs, preserve failures, inspect runs, and generate reports. The research synthesis example shows these pieces working together in a small domain.

The public surface is currently CLI-first. The repo is best read as a working kernel and demonstration workspace: enough to inspect the model of work, run the example, and build new declared domains from the same pattern.

## The Short Version

Earmark is for AI-assisted work that needs a memory stronger than chat history.

It turns loose context into declared inputs, loose outputs into durable objects, loose continuation into handoffs, and loose trust into inspectable process.

The demo shows the whole idea in miniature: source notes become findings, findings become a summary, and the system keeps the trail.

© 2026 Mikhail Shakhnazarov. Licensed under AGPL-3.0-or-later OR Commercial, matching the repository license.
