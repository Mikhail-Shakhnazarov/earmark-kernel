# Earmark Research Synthesis Demo Walkthrough

This walkthrough demonstrates the end-to-end capabilities of the Earmark system using a research synthesis workflow. We will extract findings from raw research notes and synthesize them into a coherent summary, all governed by system-defined classes and workflows.

## 1. Setup and Initialization

First, we initialize a clean workspace and register all necessary declarations (classes, instructions, workflows, etc.).

```bash
# Clean up previous runs
rm -rf .earmark

# Initialize the workspace
em init

# Register core classes
em declare register --kind class declarations/classes/source_note.yaml
em declare register --kind class declarations/classes/finding.yaml
em declare register --kind class declarations/classes/summary.yaml

# Register instructions (transformation logic)
em declare register --kind instruction declarations/instructions/source_to_finding.md
em declare register --kind instruction declarations/instructions/finding_to_summary.md

# Register compiled contexts (work surface definitions)
em declare register --kind compiled-context declarations/compiled_contexts/source_notes_for_extraction.yaml
em declare register --kind compiled-context declarations/compiled_contexts/findings_for_summary.yaml

# Register provider profiles (LLM adapters)
em declare register --kind provider-profile declarations/provider_profiles/local_mock.yaml
em declare register --kind provider-profile declarations/provider_profiles/google_gemini.yaml

# Register the synthesis workflow
em declare register --kind workflow declarations/workflows/research_synthesis.yaml

# Register and activate the system
em declare register --kind system declarations/systems/system.yaml
em system activate sys_research_synthesis
```

## 2. Depositing Seed Data

We deposit raw research notes into the canonical store. These will serve as the starting point for our synthesis.

```bash
em deposit --class source_note --title "Federated Graphs: Agility and Ownership" --payload-file data/seed_notes/note_1_benefits.md
em deposit --class source_note --title "The Cost of Heterogeneity" --payload-file data/seed_notes/note_2_costs.md
```

## 3. Executing the Synthesis Workflow

We run the `research_synthesis` workflow. We provide the IDs of the source notes we want to process.

```bash
# Get IDs of deposited notes
em query --class source_note

# Run the workflow (replace <ID> with actual object IDs)
em workflow run research_synthesis --system-id sys_research_synthesis --with <ID_1> --with <ID_2>
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
em run explain <RUN_ID>

# View the run timeline
em run timeline <RUN_ID>

# Inspect specific artifacts surfaced in the run explain/timeline
# Inspect specific artifacts surfaced in the run explain/timeline
em assignment explain <ASSIGNMENT_ID>
em change-set explain <CHANGE_SET_ID>
em handoff explain <HANDOFF_ID>

# Generate a static HTML report for sharing
em report run <RUN_ID> --output reports/synthesis_run.html
```

## 5. Failure Governance

Earmark handles failures gracefully by persisting `TransformationFailure` objects when a transition fails (e.g., LLM provider error).

```bash
# Activate a system with a failing mock profile
em system activate sys_failing_demo

# Run the workflow again
em workflow run research_synthesis --system-id sys_failing_demo --with <ID_1>

# Locate the failed run
em run list

# Inspect the failure
em run explain <FAILED_RUN_ID>
em audit failures --run-id <FAILED_RUN_ID>

# Explain the specific failure to understand the cause and context
em failure explain <FAILURE_ID>

# Generate a report for the failed run to share with the team
em report run <FAILED_RUN_ID> --output reports/failed_run.html
```

This failure is persisted canonically, ensuring that the system state remains auditable even when things go wrong. Operators can inspect the blocked assignment and failed change set to understand exactly where the process broke down.
