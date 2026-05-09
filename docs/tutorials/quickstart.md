# Quickstart

Get a working Earmark run in under 5 minutes using the built-in research synthesis demo.

## Build the CLI

```bash
REPO_ROOT=$(pwd)
cargo build -p earmark-cli
alias em="$REPO_ROOT/target/debug/earmark-cli"
```

## Initialize a workspace

```bash
mkdir my-workspace && cd my-workspace
em init
```

You should see:

```
{
  "ok": true,
  "summary": "workspace initialized",
  "root": "...",
  "paths": {
    "canonical_dir": ".../.earmark/canonical",
    "declarations_dir": ".../.earmark/declarations",
    "work_surfaces_dir": ".../.earmark/work_surfaces",
    "index_path": ".../.earmark/derived/index.sqlite"
  }
}
```

## Register a system

Use the example system manifest from the repository:

```bash
em system register "$REPO_ROOT/examples/research-synthesis/declarations/systems/system.yaml"
em system activate sys_research_synthesis
```

This registers the research synthesis domain — three object classes (`source_note`, `finding`, `summary`), two instructions, and one two-stage workflow.

## Deposit some data

Put a few source notes into the corpus:

```bash
em deposit --class source_note --title "Context Boundaries" --body "AI context should be bounded, not ambient."
em deposit --class source_note --title "Lineage" --body "Every derived object should trace back to its source."
```

## Run the workflow

Find your deposited objects:

```bash
em query --class source_note
```

Pick an object ID from the output and run the workflow:

```bash
em workflow run research_synthesis --system-id sys_research_synthesis --with <object_id>
```

The output will show the run ID, created artifacts, and suggested next commands.

## Inspect the results

```bash
# What happened in the run
em run explain latest

# Visual timeline of events
em run timeline latest

# Generate an HTML report you can open in a browser
em report run latest --output report.html
```

`em run explain` will show you:
- Which assignments were created
- What objects were produced
- Whether validation passed or failed
- Which handoffs were emitted for successor work

## What just happened?

You ran a two-stage workflow:

1. **Extraction**: Earmark compiled a bounded work surface containing your source notes, then extracted findings. Each finding was linked to its source through a `derived_from` relation.

2. **Synthesis**: Earmark emitted a handoff from Stage 1 containing only the findings — not the original source notes. Stage 2 produced a summary from that bounded input.

The key thing: Stage 2 never saw the raw source notes. It worked from the handoff. That's bounded continuation.

## Next steps

- [Research Synthesis Demo](research-synthesis-demo.md) — deeper walkthrough of staged execution
- [Context Compilation](../concepts/context-compilation.md) — how Earmark decides what a runtime sees
- [Build a Domain Definition](build-a-domain-definition.md) — define your own classes and workflows
