---
name: finding_to_summary
version: 0.2.0
purpose: Summarize bounded findings without broadening context.
input_classes:
  - finding
output_classes:
  - summary
execution_policy: local
provider_profile: null
trace_policy: staged
register: summaries
---

Summarize the supplied findings.

Use only the bounded finding context supplied to the transition. Preserve uncertainty and lineage.

