# Knowledge Briefing Demo

This demo shows how a small body of expert material can be converted into a source-backed briefing artifact through staged, inspectable AI work.

## The Problem

Knowledge management breaks down long before document storage does. Teams rarely lose access to files, but they routinely lose the discipline of turning notes into reusable claims. Instead, context travels as ambient memory: forwarded threads, chat fragments, and informal summaries. This demo adds an explicit extraction and handoff step so claims become reusable objects instead of being re-derived each time.

## What This Demo Does

The pipeline runs in two stages: `source_note -> finding -> briefing_card`. First, raw notes are converted into atomic findings with provenance and uncertainty flags where needed. Second, only those findings are admitted into a bounded synthesis stage that produces a structured briefing card. The runtime never receives the whole corpus. Each stage receives only its declared work surface.

## The Seed Corpus

The seed corpus contains five short notes in mixed quality and register: workshop observations, a quantitative report excerpt, an internal pilot note, a stakeholder concern, and one ambiguous/outdated note. This mix is deliberate so the extraction step has to preserve signal while surfacing uncertainty.

## Running the Demo

From the repo root:

```bash
cd examples/knowledge-briefing

# Register declarations for this demo
em system register declarations/systems/system.yaml

# Validate declarations
em declare validate declarations/systems/system.yaml

# Activate the system
em system activate sys_knowledge_briefing

# Deposit source notes
em deposit seed/note_1_workshop.md --class source_note --title "Workshop Notes"
em deposit seed/note_2_report_excerpt.md --class source_note --title "Federal Progress Excerpt"
em deposit seed/note_3_project_note.md --class source_note --title "Pilot Integration Note"
em deposit seed/note_4_stakeholder.md --class source_note --title "Stakeholder Concern"
em deposit seed/note_5_ambiguous.md --class source_note --title "Ambiguous District Heating Note"

# Run the governed pipeline
em workflow run knowledge_briefing --provider local_mock

# Inspect resulting objects and lineage
em list --class finding
em list --class briefing_card
em show --class briefing_card --latest
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
