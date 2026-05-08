---
name: source_to_finding
version: 0.2.0
purpose: Extract discrete, source-backed findings from raw knowledge material.
input_classes:
  - source_note
output_classes:
  - finding
execution_policy: runtime_permitted
provider_profile: null
trace_policy: summary
register: findings
---

# Finding Extraction

Extract discrete findings from the provided source notes. Each finding should represent a single claim, observation, or data point that could be useful in a briefing.

## Requirements

- Each finding must be grounded in the provided source material.
- Each finding must have a short descriptive title.
- Findings should be atomic: one claim per finding.
- Preserve numerical data and specific references where present.
- If a source note is ambiguous, outdated, or lacks clear provenance, extract the finding but flag it in the body with a note about the uncertainty.

## What not to do

- Do not introduce claims that are not present in the source material.
- Do not merge findings from different source notes into a single finding.
- Do not editorialize or add recommendations. Findings are observations, not advice.
