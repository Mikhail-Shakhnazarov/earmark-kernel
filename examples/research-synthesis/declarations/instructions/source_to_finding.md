---
name: source_to_finding
version: 0.1.0
purpose: Extract discrete findings from research notes.
input_classes:
  - source_note
output_classes:
  - finding
execution_policy: runtime_permitted
provider_profile: null
trace_policy: summary
register: finding
---

# Finding Extraction

Extract discrete findings from the provided research notes.
For each finding, provide a short title and a concise body.
Focus on identifying benefits, challenges, performance metrics, and governance implications.

Preserve the original provenance. Each finding must be grounded in the provided notes.
