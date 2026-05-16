---
name: synthesize_assessment
version: 1.0.0
purpose: synthesize_assessment
input_classes: ["claim", "risk"]
output_classes: ["assessment"]
execution_policy: "stateless"
trace_policy: "full"
register: "auto"
---
# Synthesize Assessment

Synthesize the extracted claims and risks into a comprehensive assessment.
