# Artifact Types

Earmark produces durable artifacts during execution. These are persistent objects in the store — not log entries, not ephemeral state. They can be queried, inspected, and used for audit and continuation.

## TransitionAssignment

Tracks ownership and status of a single piece of work.

- **What it records**: which transition was claimed, by whom, over which bounded inputs, and what happened.
- **Statuses**: `Assigned`, `Completed`, `Blocked`, `Released`, `Expired`, `Superseded`.
- **Example**: "Agent gemini-flash is extracting findings from source_note obj_abc123. Status: Completed."

```bash
em assignment explain <assignment_id>
```

## ChangeSet

The atomic record of what a transition produced.

- **What it records**: created objects, created relations, and updated standings.
- **Validity**: `Valid` or `Invalid`. Invalid change sets are preserved for audit — they show exactly what the model tried to do when it failed.
- **Example**: "This transition created 3 finding objects and 3 derived_from relations. Validation: passed."

```bash
em change-set explain <change_set_id>
```

## HandoffManifest

Defines the bounded input for the next stage of work.

- **What it records**: root objects, inherited inputs, newly created objects, allowed classes, allowed relations, standing constraints, and required checks.
- **Example**: "Successor work may see finding objects and traverse derived_from relations. Source notes are excluded."

```bash
em handoff explain <handoff_id>
```

## TransformationFailure

A durable error record produced when a transition fails.

- **What it records**: the error message, the failed assignment, and the failed change set (if any).
- **Example**: "Validation failure: output missing required 'title' header. Assignment blocked. Change set persisted as invalid."

```bash
em failure explain <failure_id>
```

## WorkflowRunRecord

The top-level record of a single workflow execution.

- **What it records**: start time, end time, system ID, workflow ID, status, and links to all generated artifacts (assignments, change sets, handoffs, failures).

```bash
em run explain <run_id>
```

## Object

The fundamental unit of data in the corpus.

- **Versioning**: objects are immutable. Changes create a new version linked to the previous one.
- **Class**: every object has a declared class (e.g., `source_note`, `finding`, `briefing_card`).
- **Headers**: typed metadata fields (e.g., `title`).
- **Payload**: the actual content.

## Relation

A typed link between two objects.

- **Examples**: `derived_from`, `supports`, `reviews`.
- **Declared constraints**: relation rules in class declarations define which classes can participate in which relation types.
- **Used by**: context compilation (relation traversal) and lineage inspection.

## Summary

| Artifact | Purpose | Inspect With |
|---|---|---|
| Assignment | Work tracking | `em assignment explain` |
| ChangeSet | Data delta | `em change-set explain` |
| Handoff | Bounded continuation | `em handoff explain` |
| Failure | Error audit | `em failure explain` |
| Run | Execution history | `em run explain` |

## See Also

- [Staged Execution](../concepts/staged-execution.md) — the lifecycle that produces these artifacts
- [Failures](../concepts/failures.md) — how failed work is preserved
