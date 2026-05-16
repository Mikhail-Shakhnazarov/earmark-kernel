# Risk Assessment Demo

This example demonstrates a non-linear workflow topology in Earmark.

## Topology

```text
source_notes -> extract_claims -> synthesize_assessment -> review -> export
             -> extract_risks  ->
```

1.  **Branching:** `source_notes` are processed by both `extract_claims` and `extract_risks` operations.
2.  **Joining:** `synthesize_assessment` consumes outputs from both extraction stages.
3.  **Review:** A manual review stage is included before export.
4.  **Export:** Final artifact export governed by a standing policy.

## Usage

1.  Initialize workspace: `em init`
2.  Register system: `em declare register --kind system examples/risk-assessment-demo/systems/system.yaml`
3.  Deposit source notes:
    ```bash
    em deposit --class source_note --title "Note 1" --body "The product is fast but has a high failure rate."
    ```
4.  Run workflow:
    ```bash
    em workflow run risk_assessment_workflow
    ```

## Limitations

- Guards are declared but enforcement depends on engine implementation status.
- Conditional edges are defined but may fall back to default behavior.
