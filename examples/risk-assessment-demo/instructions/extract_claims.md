---
name: extract_claims
version: 1.0.0
purpose: extract_claims
input_classes: ["source_note"]
output_classes: ["claim"]
execution_policy: "stateless"
trace_policy: "full"
register: "auto"
---
# Extract Claims

Extract all factual claims from the provided source notes.
