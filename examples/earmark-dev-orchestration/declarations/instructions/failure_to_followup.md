---
name: failure_to_followup
version: 0.1.0
purpose: Convert failed or blocked work into actionable follow-up tasks.
input_classes:
  - review_decision
output_classes:
  - followup_task
execution_policy: runtime_permitted
provider_profile: null
trace_policy: summary
register: followup_task
---

# Follow-Up Generation

When a review decision is needs_revision or rejected, generate follow-up tasks.

Each follow-up task should:
- Reference the source review and original task.
- State the specific gap or blocker.
- Suggest scope boundaries for the next attempt.
- Set appropriate priority based on impact.
