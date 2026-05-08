# Knowledge Briefing Pipeline: What This Demonstrates

## The Problem

Organisations accumulate knowledge faster than they can structure it. Workshop notes, internal assessments, stakeholder feedback, and progress reports pile up in shared drives. When someone needs to prepare a briefing, they start from scratch: re-reading raw material, re-extracting the key points, re-assessing what is reliable and what is uncertain. The work has already been done — but it was never captured in a reusable form.

AI can help, but the standard approach — "give the AI all your documents and ask it to summarise" — has a structural flaw. The AI sees everything at once. There is no way to verify which source informed which conclusion. There is no record of what was excluded or flagged as uncertain. If the summary is wrong, you cannot trace the error back to its source without re-reading everything yourself.

## What This Demo Shows

This demo implements a different approach: **staged extraction with bounded context**.

The pipeline works in two steps:

**Step 1: Finding extraction.** Each source note is processed individually. The system extracts discrete findings — atomic claims, each traceable to its source. If a source note is ambiguous or outdated, the finding is flagged rather than silently included.

**Step 2: Briefing synthesis.** The system produces a structured briefing card from the extracted findings only. It does not see the original source notes. This is deliberate: it forces the briefing to be grounded in the validated findings rather than in unstructured raw material.

The result is a briefing card with five sections:
- **Executive Summary** — key themes across all findings
- **Supported Findings** — claims backed by source material
- **Operational Implications** — what the findings mean for decision-making
- **Open Questions** — what remains unclear or contested
- **Blocked Material** — findings that cannot be used and why

## Why This Matters

**Traceability.** Every claim in the briefing traces back to a specific finding, and every finding traces back to a specific source note. If something in the briefing is wrong, you can follow the chain backward to identify where the error entered.

**Quality control.** Ambiguous or undated source material is flagged during extraction, not silently absorbed into the summary. Uncertain material appears in the "Open Questions" section rather than being presented as fact.

**Bounded context.** Each stage of the pipeline sees only what it is declared to see. The briefing synthesiser never accesses the raw source notes — it works from the extracted findings only. This prevents the AI from taking shortcuts (e.g., pulling details from raw notes that weren't validated as findings).

**Reusability.** The extracted findings are durable objects. They can be used in multiple briefings, queried later, or audited independently. They don't disappear after the summary is generated.

## The Seed Corpus

The demo uses five synthetic but realistic source notes:

1. **Workshop notes** — observations from a regional planning workshop
2. **Progress report excerpt** — quantitative data from a federal assessment
3. **Internal project note** — results from a data integration pilot
4. **Stakeholder feedback** — concerns raised by a municipal administrator
5. **Ambiguous legacy note** — an undated note with unclear provenance

This mix is deliberate. Notes 1–4 contain usable material. Note 5 is included to test the system's ability to flag unreliable sources rather than treating all input as equally trustworthy.

## How This Relates to Knowledge Management

This demo is a capability demonstration, not a finished product. It shows a specific pattern: **governed extraction and synthesis with inspectable lineage**.

In a production context, the same pattern applies to:
- Turning workshop documentation into reusable institutional knowledge
- Producing evidence-backed briefings for leadership from dispersed source material
- Maintaining a structured knowledge base where claims have provenance and expiry
- Auditing how AI-generated outputs relate to their source material

The underlying principle is that AI-assisted knowledge work should be inspectable: every output should trace back to its inputs, every excluded input should be documented, and every uncertain claim should be visible.
