---
name: report_to_review_summary
version: 0.1.0
purpose: Summarize executor reports for review decision.
input_classes:
  - executor_report
output_classes:
  - review_decision
execution_policy: runtime_permitted
provider_profile: null
trace_policy: summary
register: review_decision
---

# Report Review

Review the executor report against the original task and manifest.

Consider:
- Did the implementation satisfy the acceptance criteria?
- Did all local gates pass?
- Are the deviations acceptable?
- Should follow-up tasks be created?

Output a review decision: accepted, rejected, needs_revision, or superseded.
