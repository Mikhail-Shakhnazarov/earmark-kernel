---
name: source_to_finding
version: 0.2.0
purpose: Extract bounded findings from source notes.
input_classes:
  - source_note
output_classes:
  - finding
execution_policy: local
provider_profile: null
trace_policy: staged
register: findings
---

Extract discrete findings from the supplied source notes.

Preserve source uncertainty. Do not introduce findings that are not grounded in the source.

