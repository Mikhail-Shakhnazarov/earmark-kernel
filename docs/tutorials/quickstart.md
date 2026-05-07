# Quickstart: Your First Earmark Run

This tutorial gets you moving with Earmark in under 5 minutes using the built-in research synthesis demo.

## 1. Installation

Build the CLI from the workspace root:

```bash
cargo build -p earmark-cli
alias em="$(pwd)/target/debug/earmark-cli"
```

## 2. Initialize a Workspace

Create a new directory for your work and initialize it:

```bash
mkdir my-workspace && cd my-workspace
em init
```

## 3. Register a System

Use the example system manifest provided in the repository:

```bash
em system register ../examples/research-synthesis/declarations/systems/system.yaml
```

Activate it so your commands know which domain to use:

```bash
em system activate sys_research_synthesis
```

## 4. Deposit Seed Data

Deposit a few source notes into your corpus:

```bash
em deposit --class source_note --title "Test Note 1" --body "AI context should be bounded."
em deposit --class source_note --title "Test Note 2" --body "Lineage matters for auditability."
```

## 5. Run a Workflow

Find an object ID to start with:

```bash
em query --class source_note
```

Run the `research_synthesis` workflow using one of those IDs:

```bash
em workflow run research_synthesis --system-id sys_research_synthesis --with <object_id>
```

## 6. Inspect the Results

Earmark provides rich inspection tools to see what happened:

```bash
# See the run status and next steps
em run explain latest

# View the visual timeline of events
em run timeline latest

# Generate a full HTML report
em report run latest --output report.html
```

## Next Steps

- Explore the [Research Synthesis Demo](research-synthesis-demo.md) for a deeper look at staged execution.
- Read about [Context Compilation](../concepts/context-compilation.md) to understand how Earmark bounds your data.
- Start [Building Your Own Domain](build-a-domain-definition.md).
