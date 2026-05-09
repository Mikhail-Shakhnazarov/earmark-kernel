# Earmark

[![CI](https://github.com/Mikhail-Shakhnazarov/earmark-workspace/actions/workflows/ci.yml/badge.svg)](https://github.com/Mikhail-Shakhnazarov/earmark-workspace/actions/workflows/ci.yml)

A declarative kernel for governed AI work. Earmark compiles bounded context from a corpus, executes declared transitions over that context, and records what happened as durable, inspectable artifacts. Workspace state lives in a Git-backed canonical store implemented through `gix`, with a rebuildable derived index for fast query and inspection.

## The Problem

AI-assisted work usually runs on ambient context: a chat history, some retrieved snippets, whatever files happen to be open. That's fine for quick tasks. It falls apart when work needs to be inspected, resumed, reviewed, or handed to someone else.

Concrete failure modes:

- A model sees more than the current task requires.
- A later step silently inherits assumptions from an earlier prompt.
- Failed work vanishes into logs or chat transcripts.
- You can't prove which inputs produced which output.
- Review and approval are social conventions, not system state.
- Continuing work means re-reading a long conversation, not loading a durable artifact.

Earmark replaces ambient context with bounded, inspectable continuation.

## How It Works

You declare the shape of your domain — the types of objects, the relations between them, the rules about what each step is allowed to see — and Earmark handles the rest: compiling context, running transitions, validating results, recording failures, and emitting handoffs for successor work.

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

# Register and run the demo
em system register ../examples/research-synthesis/declarations/systems/system.yaml
em system activate sys_research_synthesis
em deposit --class source_note --title "Test Note" --body "AI context should be bounded."
em workflow run research_synthesis --system-id sys_research_synthesis --with <object_id>
```

See the [Quickstart Tutorial](docs/tutorials/quickstart.md) for a complete walkthrough.

## Documentation

| | |
|---|---|
| **[Quickstart](docs/tutorials/quickstart.md)** | Get moving in 5 minutes |
| **[Practical Guide](docs/tutorials/practical-guide.md)** | Plain-language explanation and use cases |
| **[Staged Execution](docs/concepts/staged-execution.md)** | How transitions, assignments, and handoffs work |
| **[Context Compilation](docs/concepts/context-compilation.md)** | How Earmark bounds what a runtime sees |
| **[Research Synthesis Demo](docs/tutorials/research-synthesis-demo.md)** | Walk through a complete two-stage workflow |
| **[Build a Domain](docs/tutorials/build-a-domain-definition.md)** | Define your own classes, workflows, and system |
| **[CLI Reference](docs/reference/cli.md)** | Command lookup |
| **[Runtime Integration](docs/reference/runtime-integration-guide.md)** | Using Earmark from Rust or any language |
| **[Provider Extension](docs/reference/provider-extension.md)** | Register custom providers through the current extension surface |
| **[Declaration Authoring](docs/declarations/README.md)** | Examples and validation rules |

## Current Status

Workspace verification is exposed through the CI workflow, which runs formatting, workspace checks, tests, and Clippy. It is not yet a packaged application.

What works:
- Declaration, validation, and scaffolding of domain definitions
- Bounded context compilation from declared objects and relations
- Staged workflow execution with assignment lifecycle management
- Durable change sets, handoffs, and failure records
- Git-backed canonical storage through `gix`, with commit-backed workspace history
- CLI inspection, explanation, and HTML report generation
- Registry-based custom provider extension through `ProviderRegistry` and `ProviderService`
- Additive async provider traits through `AsyncProviderService` and `AsyncProviderAdapter`
- Optional Google Gemini provider adapter (requires `gemini` feature)
- Versioned JSON CLI contracts at 0.2.0

What doesn't exist yet:
- Package distribution (you build from source)
- GUI or web dashboard

## Features In Development

The current provider extension surface supports in-process custom provider registration through `ProviderRegistry`, `ProviderService`, and provider adapters. That is the active extension mechanism in the repository today.

Planned next-step extension work is focused on fuller plugin functionality around provider loading and discovery. The repository does not present dynamic plugin loading, plugin directories, or marketplace-style plugins as current features.

The repository also exposes additive async provider traits through `AsyncProviderService` and `AsyncProviderAdapter`. The current workflow entrypoints remain synchronous, so async support exists as a preparation seam for future runtime expansion rather than as end-to-end async execution today.

## Build and Test

```bash
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

## Acknowledgments

Earmark owes an architectural debt to [Engram](https://github.com/vincents-ai/engram) and its creator, Vincent Palmer. Engram's treatment of durable project state, explicit context, and structured agent-operable knowledge helped open the direction that Earmark develops here.

## License

AGPL-3.0-or-later OR Commercial. See [LICENSE](./LICENSE).
