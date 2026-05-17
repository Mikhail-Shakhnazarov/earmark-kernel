---
name: task_to_manifest
version: 0.1.0
purpose: Generate an executor manifest from an implementation task.
input_classes:
  - implementation_task
output_classes:
  - executor_manifest
execution_policy: runtime_permitted
provider_profile: null
trace_policy: summary
register: executor_manifest
---

# Manifest Generation

Given an implementation task, produce an executor manifest suitable for OpenCode dispatch.

The manifest must include:
- The task objective and scope boundaries.
- Target files and allowed areas.
- Local gates to verify completion.
- Authority boundaries and stop conditions.

Preserve the task_id linkage. Set attempt number from context.
