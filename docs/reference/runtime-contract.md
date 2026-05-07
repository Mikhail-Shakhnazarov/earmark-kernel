# Earmark Runtime Contract

**Version**: 0.2.0 (Pre-stable)

This document formalizes the interaction contract between the Earmark kernel and external runtimes (agents, humans, or automated systems). It defines the six-step execution flow, the assignment lifecycle, and the JSON shapes for stable integration.

## 1. The Six-Step External Flow

Earmark governs intelligence as a public process. External runtimes participate in this process by executing transitions defined in workflows.

### Step 1: Request Bounded Context
The runtime requests the materialization of a context to work within.
- **Rust API**: `compile_work_surface(compiled_context_ref)` or `compile_connected_context(root_ids, depth, filters)`
- **CLI**: `em context compile --root <object_id> --depth 2 --json`
- **Returns**: `WorkSurfaceManifest` or `ConnectedContextManifest`.

### Step 2: Receive Work Packet
A `WorkPacket` is emitted by the kernel during workflow execution.
- **Type**: `WorkPacket` struct.
- **Content**: Contains inputs, constraints, instruction, and the provider profile.

### Step 3: Produce Candidate
The external runtime (or a provider adapter) processes the packet and produces a candidate payload.
- **Output Type**: `ProviderResponse` containing `candidate_payload`.

### Step 4: Deposit Output
The runtime deposits the resulting objects back into the kernel.
- **Rust API**: `deposit_object(class, kind, title, payload, provenance)` or `complete_transition_assignment(assignment_id, draft, agent_id)`.
- **CLI**: `em deposit --class <class> --body "..." --json`.
- **Returns**: `ObjectRef` or `ChangeSet`.

### Step 5: Validate and Persist
The kernel validates the deposited change set against transition contracts.
- **Mechanism**: `ChangeSetValidationResult` is applied.
- **Failure**: If invalid, a `TransformationFailure` is recorded, and the change set is persisted as an invalid artifact for audit.

### Step 6: Continue from Handoff
Upon successful completion, a `HandoffManifest` is emitted.
- **Usage**: Successor transitions use the `handoff_manifest` field in a `WorkflowRunRequest` to continue the lineage.

## 2. Assignment Lifecycle

Transitions are protected by an assignment system to prevent race conditions and ensure auditability.

- **Assignment**: `assign_transition(...)` creates a `TransitionAssignment` in `Assigned` status.
- **Complete**: `complete_transition_assignment(...)` transitions status to `Completed` and links to the `ChangeSet`.
- **Block**: If validation fails or a manual block is requested, status becomes `Blocked`.
- **Release**: Voluntarily giving up an assignment (status: `Released`).
- **Expire**: If a lease is provided and exceeded, the system marks it `Expired`.
- **Supersede**: When a new assignment replaces an old one (status: `Superseded`).
- **Resume**: Re-opening a `Blocked` or `Expired` transition creates a new assignment linked to the old one.

## 3. JSON Contract Shapes

All CLI commands support a `--json` flag. The output is wrapped in a versioned envelope:

```json
{
  "contract_version": "0.2.0",
  "data": { ... }
}
```

### Key Artifact Shapes

#### ObjectRef
```json
{
  "id": "obj_...",
  "version_id": "...",
  "kind": "object",
  "class": "source_note"
}
```

#### ChangeSet
```json
{
  "id": "change_set_...",
  "run_id": "run_...",
  "transition_id": "transform_1",
  "assignment_id": "assignment_...",
  "created_object_ids": ["obj_..."],
  "created_at": "2026-05-06T14:00:00Z"
}
```

## 4. Error Contract

Errors are returned as structured JSON when using `--json`.

- `ExecError`: Failures during workflow execution (e.g., `InvalidWorkflow`, `MissingInput`).
- `DispatchFailure`: Failures during provider dispatch (e.g., `ProviderUnavailable`, `BudgetExceeded`).
- `RuntimeToolError`: Failures in the runtime surface (e.g., `Conflict`, `MissingObject`).

## 5. Stability Statement

The runtime contract is currently at version `0.2.0`. Breaking changes to types or CLI JSON output will increment the minor version during the pre-1.0 phase. Consumers should verify the `contract_version` field in the JSON envelope.
