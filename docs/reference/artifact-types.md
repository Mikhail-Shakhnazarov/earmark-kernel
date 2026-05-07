# Reference: Artifact Types

Earmark produces durable, canonical artifacts during execution. These artifacts are first-class objects in the store and can be used for audit, governance, and continuation.

## 1. TransitionAssignment
An assignment represents a "claim" on a specific piece of work.

- **Purpose**: Tracks ownership and status of a single transition.
- **Statuses**: `Assigned`, `Completed`, `Blocked`, `Released`, `Expired`, `Superseded`.
- **Contains**: Link to the Run, Transition ID, and bounded input objects.

## 2. ChangeSet
A change set is the atomic collection of modifications produced by a transition.

- **Purpose**: Records the "delta" of a transition.
- **Validity**: Can be `Valid` or `Invalid`.
- **Contains**: Created objects, created relations, and updated standings.
- **Audit**: Even if a change set is rejected by a validator, the "invalid" change set is persisted for audit.

## 3. HandoffManifest
A handoff defines the bounded continuation surface for the next transition.

- **Purpose**: Bridges the gap between stages without using ambient context.
- **Contains**: Root objects, inherited context, and admission rules (allowed classes/relations).
- **Continuation**: A successor run uses the handoff ID to reconstruct its admissible work surface.

## 4. TransformationFailure
A failure record produced when a transition fails to complete successfully.

- **Purpose**: Provides a first-class audit trail for errors.
- **Contains**: Error message, link to the failed Assignment, and link to the failed ChangeSet (if any).

## 5. WorkflowRunRecord
The top-level record of a single workflow execution.

- **Purpose**: Tracks the lifecycle and event history of a run.
- **Contains**: Start/end times, system ID, and links to all generated artifacts.

## 6. Object (Canonical)
The fundamental unit of data in the Earmark corpus.

- **Purpose**: Stores the actual content (payload) and metadata (headers).
- **Versioning**: Every object is immutable; changes create a new version linked to the previous one.

## 7. Relation (Canonical)
A first-class link between two objects.

- **Purpose**: Records typed connections such as `derived_from` or `supports`.
- **Lineage**: Relations are the foundation of context compilation and graph visualization.

## Summary Table

| Artifact | Durability | Purpose | Primary CLI Command |
| :--- | :--- | :--- | :--- |
| **Assignment** | Canonical | Work Tracking | `em assignment explain` |
| **ChangeSet** | Canonical | Data Delta | `em change-set explain` |
| **Handoff** | Canonical | Continuation | `em handoff explain` |
| **Failure** | Canonical | Error Audit | `em failure explain` |
| **Run** | Canonical | History Trace | `em run explain` |

## See Also
- [Concept: Staged Execution](../concepts/staged-execution.md)
- [Concept: Failures](../concepts/failures.md)
