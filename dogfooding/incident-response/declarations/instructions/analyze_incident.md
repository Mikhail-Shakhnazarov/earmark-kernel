---
name: analyze_incident
version: 0.1.0
purpose: Analyze an incident to extract key observations and recommend remediation actions.
input_classes:
  - incident
output_classes:
  - observation
  - action
execution_policy: single
trace_policy: full
register: ""
provider_profile: null
---

# Instruction

Write concise, grounded output using only bounded inputs.
