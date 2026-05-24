# Artifact Types

Earmark produces durable artifacts during execution. These are persistent objects in the store — not ephemeral logs. They can be queried, inspected, and used for audit and continuation of work.

## Assignment

Tracks the ownership and status of a specific task within the work spine.

- **What it records**: which transition was claimed, by which model/runtime, and the specific inputs used.
- **Statuses**: `Assigned`, `Completed`, `Blocked`.

## Transition Result (Change Set)

The atomic record of exactly what a task produced.

- **What it records**: new objects, new relationships, and updated evaluation metadata.
- **Validity**: `Valid` or `Invalid`. Invalid results are preserved so you can audit exactly what the model tried to do when it failed.

## Handoff

Defines the task-specific context passed to the next stage of work.

- **What it records**: the specific findings and evidence that the next stage is allowed to see.
- **Example**: "Successor work may see `finding` objects. Source notes are excluded."

## Failure Record

A durable record produced when a task cannot move forward.

- **What it records**: the error message, the failed attempt, and the rejected result (if any).
- **Example**: "Validation failure: output missing required summary. Assignment blocked."

## Workflow Run

The top-level record of a single execution of the work spine.

- **What it records**: start/end times, active system, and links to all generated assignments and results.

## Object & Relationship

The fundamental building blocks of the Earmark corpus.

- **Object**: A versioned unit of data with a declared class and payload.
- **Relationship**: A verified link between two objects that enables context traversal and lineage.

## Summary

| Artifact | Purpose | Command |
|---|---|---|
| **Assignment** | Task tracking | `em assignment explain` |
| **Result** | Rejection/Audit | `em result explain` |
| **Handoff** | Data transfer | `em handoff explain` |
| **Failure** | Feedback | `em failure explain` |
| **Run** | Lifecycle audit | `em run explain` |

## See Also

- [The Durable Work Spine](../concepts/staged-execution.md) — the lifecycle that produces these artifacts
- [Learning from Failure](../concepts/failures.md) — how failed work is preserved
- [Native Orchestration](../concepts/native-orchestration.md) — orchestration-specific objects (Work Items, Dispatches)
