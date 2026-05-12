# v0.3 Protocol-Based Standing Migration Note

v0.3 replaces the legacy three-field Standing object with declared standing dimensions.

## Legacy Standing Format (v0.2)

```yaml
standing:
  epistemic: working
  review: unreviewed
  process: active
```

## Current Standing Format (v0.3)

```yaml
standing:
  kernel:epistemic: working
  kernel:review: unreviewed
  kernel:process: active
  research:status: draft
```

Standing is now a map from dimension IDs to token IDs. Built-in kernel dimensions are prefixed with `kernel:`.

## Class Declaration Standing Rules

Legacy class declaration constraints:

```yaml
standing_rules:
  allowed_epistemic: [working]
  allowed_review: [unreviewed]
  allowed_process: [active]
```

Current class declaration constraints:

```yaml
standing_rules:
  allowed_standing:
    kernel:epistemic: [working]
    kernel:review: [unreviewed]
    kernel:process: [active]
```

## Standing Policy Dimensions

Legacy policy dimension references:

```yaml
transition_rules:
  - dimension: review
    from: [unreviewed]
    to: [accepted]
    requires_review: true
```

Current policy dimension references:

```yaml
transition_rules:
  - dimension: kernel:review
    from: [unreviewed]
    to: [accepted]
    requires_review: true
```

## Compatibility

Legacy objects using `epistemic`, `review`, and `process` fields remain readable and normalize to `kernel:epistemic`, `kernel:review`, and `kernel:process` during deserialization. No destructive migration is required for existing objects.

## Required Action After Upgrade

Rebuild the derived index so the `object_standing` table is populated with normalized standing rows:

- **CLI**: `em system register` triggers a full rebuild.
- **Programmatic**: `index.rebuild_from_store(&store)`.
