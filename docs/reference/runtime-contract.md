# Runtime Contract

**Version**: 0.2.0 (Pre-stable)

This document defines the contract between Earmark and external runtimes that want to participate in governed execution. It covers the six-step execution flow, the assignment lifecycle, and the JSON shapes for stable integration.

## The Six-Step Flow

External runtimes (agents, humans, or automated systems) participate in Earmark's execution model by following this sequence:

### Step 1: Request Context

The runtime requests a compiled context — the bounded set of objects it will work with.

- **Rust API**: `compile_work_surface(compiled_context_ref)` or `compile_connected_context(root_ids, depth, filters)`
- **CLI**: `em context compile --root <object_id> --depth 2 --json`
- **Returns**: `WorkSurfaceManifest` or `ConnectedContextManifest`

Default compiled-context filter semantics are conservative: class and standing filters apply to both seed objects and objects reached through relation expansion, while relation filters control which edges are traversed. A context selecting `review = accepted` must not pull in `rejected` neighbors unless expansion is explicitly widened in the declaration.

### Step 2: Receive Work Packet

During workflow execution, the engine emits a `WorkPacket` containing inputs, constraints, the instruction, and the provider profile.

### Step 3: Produce Candidate

The runtime (or a registered provider adapter) processes the packet through the provider registry and produces a `ProviderResponse` with a `candidate_payload`.

### Step 4: Deposit Output

The runtime deposits the resulting objects back into the kernel.

- **Rust API**: `deposit_object(...)` or `complete_transition_assignment(assignment_id, draft, agent_id)`
- **CLI**: `em deposit --class <class> --body "..." --json`
- **Returns**: `ObjectRef` or `ChangeSet`

### Step 5: Validate and Persist

The kernel validates the deposited change set against the transition's declared contracts. If valid, the change set is persisted and a handoff is emitted. If invalid, a `TransformationFailure` is recorded and the invalid change set is preserved for audit.

### Step 6: Continue from Handoff

On success, a `HandoffManifest` is emitted. Successor transitions use the `handoff_manifest` field in a `WorkflowRunRequest` to continue.

## Assignment Lifecycle

Transitions are protected by an assignment system to prevent race conditions and ensure traceability.

| Operation | Status | Description |
|---|---|---|
| `assign_transition` | `Assigned` | Claim work on a transition |
| `complete_transition_assignment` | `Completed` | Finish work and link to change set |
| (validation fails) | `Blocked` | Assignment blocked due to error |
| `release_assignment` | `Released` | Voluntarily give up the claim |
| (lease exceeded) | `Expired` | Timed out |
| `supersede_assignment` | `Superseded` | Replaced by a newer assignment |
| `resume_assignment` | `Assigned` (new) | Re-open blocked or expired work |

## JSON Contract

All CLI commands support `--json`. Output is wrapped in a versioned envelope:

```json
{
  "contract_version": "0.2.0",
  "data": { ... }
}
```

### ObjectRef

```json
{
  "id": "obj_...",
  "version_id": "...",
  "kind": "object",
  "class": "source_note"
}
```

### ChangeSet

```json
{
  "id": "change_set_...",
  "run_id": "run_...",
  "transition_id": "extract_findings",
  "assignment_id": "assignment_...",
  "created_object_ids": ["obj_..."],
  "created_at": "2026-05-06T14:00:00Z"
}
```

## Error Types

When using `--json`, errors are returned as structured objects:

- **ExecError**: workflow execution failures (invalid workflow, missing input)
- **DispatchFailure**: provider dispatch failures (provider unavailable, budget exceeded)
- **RuntimeToolError**: runtime surface failures (resource conflict, missing object)

## Stability

The runtime contract is at version `0.2.0`. Breaking changes to types or CLI JSON output will increment the minor version during the pre-1.0 phase. Consumers should check the `contract_version` field.
