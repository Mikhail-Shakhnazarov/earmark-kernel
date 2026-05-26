---
name: finding_to_briefing
version: 0.2.0
purpose: Synthesize validated findings into a structured briefing card.
input_classes:
  - finding
output_classes:
  - briefing_card
execution_policy: runtime_permitted
provider_profile: null
trace_policy: summary
register: briefing_card
---

# Briefing Card Synthesis

Synthesize the provided findings into a structured briefing card for internal use.

## Output Structure

The briefing card must include the following sections:

### Executive Summary
A 2-3 sentence summary of the key themes across all findings.

### Supported Findings
List each finding that is well-supported by source material. For each, note which finding it came from (by title or reference). Group related findings where appropriate.

### Operational Implications
What do these findings mean for planning or decision-making? Be specific and practical.

### Open Questions
What remains unclear, contested, or insufficiently supported? Include findings that were flagged as uncertain or outdated by the extraction step.

### Blocked or Unusable Material
List any findings that cannot be used in their current form and explain why. This section may be empty if all findings are usable.

## Rules

- The briefing must be grounded in the provided findings only.
- Do not reference source notes directly. The briefing stage receives findings, not raw source material. This is intentional.
- Keep the briefing under 500 words.
- Use plain language suitable for a non-technical decision-maker.
