---
name: synthesize
version: 1.0.0
purpose: Analyze interview notes and synthesize common themes.
input_classes:
  - interview_note
output_classes:
  - synthesis_report
execution_policy: advisory
trace_policy: full
register: synthesis_report
---
# Instruction: Synthesize Interview Notes
Analyze the provided collection of interview notes. Identify the top 3 common themes.
For each theme, provide a summary and cite the source interview.

## Output Format
Markdown report with headers for each theme.
