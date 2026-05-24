# Earmark

[![CI](https://github.com/Mikhail-Shakhnazarov/earmark-workspace/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/Mikhail-Shakhnazarov/earmark-workspace/actions/workflows/ci.yml)

AI-assisted work usually runs on ambient context: a chat history, some retrieved snippets, whatever files happen to be open. That works for quick tasks. It falls apart when work needs to be inspected, resumed, reviewed, or handed to someone else.

Concrete failure modes:

- A model sees more than the current task requires.
- A later step silently inherits assumptions from an earlier prompt.
- Failed work vanishes into logs or chat transcripts.
- You can't prove which inputs produced which output.
- Review and approval are social conventions, not system state.
- Continuing work means re-reading a long conversation, not loading a durable artifact.

Earmark replaces ambient context with structured, inspectable work.

## What Earmark Does

You declare your domain: the types of objects you work with, the relations between them, and what each processing step is allowed to see. Earmark then compiles the right context for each step, runs the step, records what happened, and passes bounded results to the next step.

Every result is stored as a durable artifact. Failures are preserved, not discarded. When work moves from one stage to the next, a handoff defines exactly what the next stage receives — no implicit inheritance from earlier conversation.

Workspace state is Git-backed (through `gix`), with a rebuildable derived index for fast query and inspection.

## How It Works

A staged flow looks like this:

```
source_note → finding → briefing_card
```

**Stage 1** receives raw source notes. It extracts discrete findings and records a change set showing what was created. Each finding links back to its source through a `derived_from` relation.

**Stage 2** receives only the findings — not the original source notes. It produces a briefing card from that bounded input. The runtime never sees the full corpus. It sees exactly what the declarations say it should see.

Between stages, a **handoff manifest** defines the bounded surface for successor work: which objects are included, which relations are allowed, which constraints apply.

After the run, you can inspect every artifact: what was assigned, what was produced, what passed or failed validation, and what the next step can see.

```bash
# Inspect the run
em run explain latest

# See the handoff between stages
em handoff explain <handoff_id>

# Generate an HTML report
em report run latest --output report.html
```

## What Earmark Records

Every run produces durable artifacts:

- **Assignments** track what work was claimed, by whom, over which inputs, and whether it completed, failed, or was blocked.
- **Change sets** record what a transition created or changed. Invalid change sets are preserved for audit — failed work doesn't disappear.
- **Handoffs** define the bounded continuation surface: which objects, relations, and constraints the next step is allowed to use.
- **Failures** are first-class records linked to the assignment and change set that produced them. They remain inspectable as persistent state.

## When It's Useful

Earmark is worth considering when:

- Outputs need provenance — you need to show which inputs produced which results.
- Failures need to remain inspectable, not just logged.
- Work spans multiple stages, and later steps should not inherit all earlier context.
- Review and standing matter — a draft is different from an accepted finding.
- Multiple runtimes or operators need to resume work from the same place.
- A durable corpus is more valuable than a chat transcript.

Good candidate domains: research synthesis, policy review, compliance workflows, editorial pipelines, knowledge management, safety-sensitive triage.

It's probably too heavy for one-off prompting, quick scripts, or casual personal notes.

## Getting Started

```bash
# Build the CLI
cargo build -p earmark-cli
alias em="$(pwd)/target/debug/earmark-cli"

# Initialize a workspace
mkdir my-workspace && cd my-workspace
em init

# Register the demo system
em system register ../examples/research-synthesis/declarations/systems/system.yaml
em system activate sys_research_synthesis

# Deposit a source note
em deposit --class source_note --title "Test Note" --body "AI context should be bounded."

# Find the deposited object's ID
em query --class source_note

# Run the workflow (use the object_id from the deposit or query output)
em workflow run research_synthesis --system-id sys_research_synthesis --with <object_id>
```

See the [Quickstart Tutorial](docs/tutorials/quickstart.md) for a complete walkthrough.

## Documentation

| | |
|---|---|
| **[Quickstart](docs/tutorials/quickstart.md)** | Get moving in 5 minutes |
| **[Practical Guide](docs/tutorials/practical-guide.md)** | Plain-language explanation and use cases |
| **[Canonical Spine](docs/architecture/canonical-spine.md)** | Authoritative architecture note for the core durable path |
| **[Staged Execution](docs/concepts/staged-execution.md)** | How transitions, assignments, and handoffs work |
| **[Context Compilation](docs/concepts/context-compilation.md)** | How Earmark bounds what a runtime sees |
| **[Standing](docs/concepts/standing.md)** | How domain lifecycle state is declared and enforced |
| **[Relation Authorization](docs/concepts/relation-authorization.md)** | How relation creation records the rule that authorized it |
| **[Research Synthesis Demo](docs/tutorials/research-synthesis-demo.md)** | Run and inspect a complete research synthesis workflow |
| **[Build a Domain](docs/tutorials/build-a-domain-definition.md)** | Define your own classes, workflows, and system |
| **[CLI Reference](docs/reference/cli.md)** | Command lookup |
| **[Runtime Integration](docs/reference/runtime-integration-guide.md)** | Using Earmark from Rust or any language |
| **[Provider Extension](docs/reference/provider-extension.md)** | Register custom providers through the current extension surface |
| **[Declaration Authoring](docs/declarations/README.md)** | Examples and validation rules |


## Current Status

Earmark is pre-release software. Build from source with Cargo. Binary packaging is a later release step.

What works today:

- Workspace initialization, object deposit, and querying
- Declaration, validation, and scaffolding of domain definitions
- Bounded context compilation from declared objects and relations
- Staged workflow execution with assignment lifecycle management
- Durable change sets, handoffs, and failure records
- Git-backed canonical storage with commit-backed workspace history
- CLI inspection, explanation, and HTML report generation

For developers building on Earmark:

- Registry-based custom provider extension through `ProviderRegistry` and `ProviderService`
- Additive async provider traits (`AsyncProviderService`, `AsyncProviderAdapter`) as a preparation seam for future async runtime support
- Versioned JSON CLI contracts at 0.2.0

Not yet available: package distribution (build from source), GUI or web dashboard.

Workspace verification is exposed through the [CI workflow](.github/workflows/ci.yml), which runs formatting, workspace checks, tests, and Clippy.

## Build and Test

```bash
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

For local development and pull-request expectations, see [CONTRIBUTING.md](./CONTRIBUTING.md). For vulnerability reporting, see [SECURITY.md](./SECURITY.md).

## Acknowledgments

Earmark draws architectural inspiration from the concept of durable, explicit context, a direction significantly explored by creators in the AI tools space. Systems like [Engram](https://github.com/vincents-ai/engram) helped demonstrate the value of structured knowledge management for agent-operable workflows, providing a conceptual foundation that Earmark continues to develop in a fully native, Git-backed architecture.

## License

AGPL-3.0-or-later OR Commercial. See [LICENSE](./LICENSE).