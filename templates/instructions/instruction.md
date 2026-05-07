---
name: your_instruction_name
version: 0.2.0
description: Describe what this instruction does.
purpose: Extract or transform bounded inputs into declared outputs.
input_classes:
  - source_note
output_classes:
  - finding
execution_policy:
  mode: single
  max_output_objects: 1
trace_policy:
  include_inputs: true
  include_prompt: false
register:
  model_family: generic
  prose_template: null
provider_profile: null
---

# Instruction

Write concise, grounded output using only bounded inputs.
