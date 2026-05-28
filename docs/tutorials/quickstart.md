# Quickstart: The 5-Minute Demo

Get a working Earmark run using the built-in research synthesis demo. This demo runs **100% locally** using a mock provider — no API keys required.

## 1. Install the CLI

The Earmark operator shell is called `em`.

```bash
# In the repository root:
cargo install --path earmark-cli
```

*Verify the installation:*
```bash
em --version
# Expected: earmark 0.1.0
```

## 2. Initialize a Workspace

Create a new directory for your work and initialize it. Earmark will create a `.earmark` folder to store your durable work spine.

> [!IMPORTANT]
> To use the example assets in this demo, create your workspace as a sibling to the `examples` directory in your checkout of the Earmark repository.

```bash
# Assuming you are in the root of the Earmark repository:
mkdir research-workspace && cd research-workspace
em init
```

## 3. Register the Demo Domain

You need a **System Definition** to tell Earmark what kind of work you are doing. We'll use the example research synthesis domain from the repository.

```bash
# Define the path to your source checkout
export REPO_ROOT=".." 

# Register and activate the system
em system register "$REPO_ROOT/examples/research-synthesis/declarations/systems/system.yaml"
em system activate sys_research_synthesis
```

This registers:
- **Classes**: `source_note`, `finding`, `summary`.
- **Instruction**: Logic for how to move between classes.
- **Workflow**: The multi-stage `research_synthesis` pipeline.

## 4. Deposit Source Materials

Put some raw research notes into your workspace. In a real workflow, these might be imported from documents or API results.

```bash
em deposit --class source_note --title "Architecture Goal" --body "AI context must be durable and bounded."
em deposit --class source_note --title "Failure Modes" --body "Ephemeral chat history leads to context bleed."
```

## 5. Run the Coordinated Workflow

Find an object to process and then trigger the workflow. Earmark will execute all stages automatically using the built-in mock provider.

```bash
# Find your object ID
em query --class source_note

# Run the workflow (replace <object_id> with the returned ID)
em workflow run research_synthesis --system-id sys_research_synthesis --with <object_id>
```

**Expected output:**
```json
{
  "status": "completed",
  "summary": "workflow run completed",
  "output_count": 2,
  "packet_count": 4
}
```

## 6. Inspect the Work Spine

Earmark records the full provenance of the result. You can inspect exactly what happened during each stage.

```bash
# See a summary of the latest run
em run explain latest

# Generate a visual HTML report
em report run latest --output report.html
```

Open `report.html` in your browser to see the findings and summary linked to your original notes.

---

## What Just Happened?

You executed a **Coordinated AI Transition**:

1. **Extraction**: Earmark compiled a task-specific input from your raw notes and extracted findings.
2. **Synthesis**: Earmark performed a handoff, passing the findings to the next stage while withholding the original source noise.
3. **Audit**: Every step was recorded into the Git-backed spine.

## Next Steps

- **[Stability Catalog](../reference/stability.md)** — See which commands are ready for production.
- **[Limitations](../limitations.md)** — Understand current constraints.
- **[Build a Domain](build-a-domain-definition.md)** — Define your own classes and workflows.
