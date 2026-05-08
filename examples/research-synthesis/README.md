# Research Synthesis Demo

This demo shows a two-stage research workflow: extracting findings from raw notes and synthesizing a final summary.

## The Problem

Research work often gets buried in documents. Turning notes into reusable claims (findings) is usually a manual, ambient process. Earmark makes this process explicit, staged, and governed.

## What This Demo Demonstrates

1. **Two-Stage Execution**: `source_note -> finding -> summary`.
2. **Bounded Handoff**: The second stage (summary) only sees the findings, not the original raw notes.
3. **Traceability**: Every finding is linked back to its source note.
4. **Failure Persistence**: If a step fails, the failure is recorded canonically.

## Prerequisites

This demo assumes you have built the Earmark CLI and aliased it as `em`. From the repository root:

```bash
cargo build -p earmark-cli
alias em="$(pwd)/target/debug/earmark-cli"
export REPO_ROOT="$(pwd)"
```

## Quick Start

This example is designed to run against an external workspace, not from inside the example directory itself. The workspace below lives outside the repository so Earmark can manage its own store state cleanly.

```bash
export WORKSPACE=/tmp/earmark-research-synthesis-demo
rm -rf "$WORKSPACE"

# Run the automated setup (init, register, activate, deposit seed data)
WORKSPACE="$WORKSPACE" "$REPO_ROOT/examples/research-synthesis/setup.sh"

# Review the deposited source notes and choose the IDs you want to process
em --root "$WORKSPACE" query --class source_note

# Run the workflow with explicit source-note inputs
em --root "$WORKSPACE" workflow run research_synthesis --system-id sys_research_synthesis --with <ID_1> --with <ID_2>

# Inspect the result
em --root "$WORKSPACE" run explain latest
```

## Detailed Walkthrough

For a step-by-step guide through the entire workflow, including inspection and failure handling, see the [Walkthrough](walkthrough/demo_walkthrough.md).

## Declarations

This system is defined in `declarations/systems/system.yaml`. It coordinates:
- 3 object classes
- 2 instructions
- 1 two-stage workflow
