# Earmark Research Synthesis Demo Walkthrough

This walkthrough demonstrates the end-to-end capabilities of the Earmark system using a research synthesis workflow. We will extract findings from raw research notes and synthesize them into a coherent summary, all governed by system-defined classes and workflows.

## 0. Preparation

Ensure the Earmark CLI is built and available in your PATH as `em`. From the repository root:

```bash
cargo build -p earmark-cli
alias em="$(pwd)/target/debug/earmark-cli"
export REPO_ROOT="$(pwd)"
export WORKSPACE=/tmp/earmark-research-synthesis-demo
rm -rf "$WORKSPACE"
```

## 1. Setup and Initialization

First, we initialize a clean external workspace and register the demo system.

```bash
# Initialize the workspace
em --root "$WORKSPACE" init

# Register and activate the system
em --root "$WORKSPACE" system register "$REPO_ROOT/examples/research-synthesis/declarations/systems/system.yaml"
em --root "$WORKSPACE" system activate sys_research_synthesis
```

## 2. Depositing Seed Data

We deposit raw research notes into the canonical store. These will serve as the starting point for our synthesis.

```bash
em --root "$WORKSPACE" deposit --class source_note --title "Federated Graphs: Agility and Ownership" --payload-file "$REPO_ROOT/examples/research-synthesis/data/seed_notes/note_1_benefits.md"
em --root "$WORKSPACE" deposit --class source_note --title "The Cost of Heterogeneity" --payload-file "$REPO_ROOT/examples/research-synthesis/data/seed_notes/note_2_challenges.md"
```

## 3. Executing the Synthesis Workflow

We run the `research_synthesis` workflow. We provide the IDs of the source notes we want to process.

```bash
# Get IDs of deposited notes
em --root "$WORKSPACE" query --class source_note

# Run the workflow (replace <ID> with actual object IDs)
em --root "$WORKSPACE" workflow run research_synthesis --system-id sys_research_synthesis --with <ID_1> --with <ID_2>
```

### What happens behind the scenes:
1. **Context Compilation**: The system compiles a "Work Surface" containing the selected source notes.
2. **Transformation 1**: The `source_to_finding` instruction is assigned to an adapter (Mock or Gemini) to extract findings.
3. **Lineage**: The system creates `finding` objects linked back to the source notes.
4. **Context Compilation 2**: Findings are grouped onto a new work surface.
5. **Transformation 2**: The `finding_to_summary` instruction is assigned to synthesize a final `summary`.

## 4. Inspecting Artifacts

After a successful run, we can inspect the generated artifacts.

```bash
# View the run summary and related artifacts
em --root "$WORKSPACE" run explain <RUN_ID>

# View the run timeline
em --root "$WORKSPACE" run timeline <RUN_ID>

# Inspect specific artifacts surfaced in the run explain/timeline
em --root "$WORKSPACE" assignment explain <ASSIGNMENT_ID>
em --root "$WORKSPACE" change-set explain <CHANGE_SET_ID>
em --root "$WORKSPACE" handoff explain <HANDOFF_ID>

# Generate a static HTML report for sharing
em --root "$WORKSPACE" report run <RUN_ID> --output reports/synthesis_run.html
```

## 5. Failure Governance

Earmark handles failures gracefully by persisting `TransformationFailure` objects when a transition fails (e.g., LLM provider error).

```bash
# Activate a system with a failing mock profile
em --root "$WORKSPACE" system activate sys_failing_demo

# Run the workflow again
em --root "$WORKSPACE" workflow run research_synthesis --system-id sys_failing_demo --with <ID_1>

# Locate the failed run
em --root "$WORKSPACE" run list

# Inspect the failure
em --root "$WORKSPACE" run explain latest
em --root "$WORKSPACE" failure list --run-id <FAILED_RUN_ID>

# Explain the specific failure to understand the cause and context
em --root "$WORKSPACE" failure explain <FAILURE_ID>

# Generate a report for the failed run to share with the team
em --root "$WORKSPACE" report run latest --output reports/failed_run.html
```

This failure is persisted canonically, ensuring that the system state remains auditable even when things go wrong. Operators can inspect the blocked assignment and failed change set to understand exactly where the process broke down.
