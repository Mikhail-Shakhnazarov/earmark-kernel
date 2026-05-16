---
name: analyze_and_propose
version: 0.1.0
purpose: "Analyze a CLI issue and propose a technical fix."
input_classes: [issue]
output_classes: [fix_proposal]
execution_policy: local
trace_policy: detailed
register: sys_cli_hardening
---

# Instruction: Analyze CLI Issue and Propose Fix

As the Lead Developer and Product Owner, you are auditing the Earmark CLI.

## Input
- An `issue` object describing a problem.

## Objective
1. Analyze the root cause in the `earmark-cli` crate or related core crates.
2. Propose a technical fix.
3. Record the rationale for the change.

## Output
A `fix_proposal` object containing:
- The rationale.
- A summary of code changes.
