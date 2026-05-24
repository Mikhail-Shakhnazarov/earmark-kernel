# Declaration Reference

This document provides the technical schemas, validation rules, and standing model details for Earmark declarations.

## Standing Model

Standing is stored as a map from standing dimension IDs to token IDs.

```yaml
standing:
  kernel:epistemic: working
  kernel:review: unreviewed
  kernel:process: active
  research:status: draft
```

Dimensions and tokens are declared by the active system definition. Kernel behavior is derived by projecting standing tokens through protocol bindings. The kernel enforces protocols, not token names.

### Dimension IDs
Dimension IDs use the `kernel:*` prefix for built-in kernel dimensions. Legacy v0.2 objects using bare `epistemic` / `review` / `process` fields are not supported.

### Custom Dimension Example

```yaml
standing_dimensions:
  - id: research:status
    default: draft
    tokens:
      - id: draft
      - id: verified
        implements:
          - protocol: kernel:review
            state: accepted
          - protocol: kernel:visibility
            properties:
              include_in_standard_context: true
              expose_to_provider: true
```

## Declaration Kinds

Supported kinds:
- `class`: Object types and relation authorization.
- `instruction`: Task purpose and LLM prompts.
- `standing-policy`: Lifecycle transitions and escalation rules.
- `workflow`: Staged execution graphs.
- `compiled-context`: Input set templates.
- `provider-profile`: Model and API credentials.
- `system`: Deployment manifest.

## Validation Coverage

| Kind | Validation Rules |
|---|---|
| `class` | Class name, version, standing-rules, relation types, counterparty classes, and relation directions. |
| `instruction` | Purpose, body, input/output class tokens. |
| `standing-policy` | Transition-rules, dimension/token validity, escalation messages. |
| `workflow` | Operation uniqueness, kindle matching, instruction references, input/output contracts. |
| `compiled-context` | Class selection, render modes, relation visibility. |
| `provider-profile` | Provider/model presence, format, budget, environment variables. |
| `system` | Dependency resolution, object existence, namespace verification. |

## JSON Schemas

Authoritative schemas are published in:
```text
docs/declarations/schema/
```
Note: CLI and Rust validation are authoritative. JSON Schemas are authoring aids.
