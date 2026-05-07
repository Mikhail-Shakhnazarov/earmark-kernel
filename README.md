# Earmark

Earmark is a declarative context and execution kernel for governed AI work. It turns a corpus into bounded runtime context, executes declared transitions over that context, and records what happened as durable assignments, change sets, validations, failures, and handoffs.

## Why This Exists

Most AI systems treat context as something to retrieve, append, or remember. That works for lightweight assistance, but it breaks down when work has to be inspected, resumed, reviewed, or governed.

Ambient context has practical failure modes:

- A model sees more than the task should require.
- A later step inherits hidden assumptions from an earlier prompt.
- Failed work disappears into logs or chat history.
- It is hard to prove which inputs produced which output.
- Review and approval become social conventions instead of system state.
- Continuation depends on a long conversation rather than a durable artifact.

Earmark takes a different approach. It treats context as something to compile from declared objects, relations, compiled contexts, and handoff rules. It treats execution as a series of bounded state transitions. Each transition leaves behind canonical evidence of what was assigned, produced, blocked, validated, and handed forward.

The goal is to replace ambient context with bounded, inspectable continuation.

## What It Is

Earmark is not an agent framework, vector memory layer, workflow app, or document database.

It is a kernel for:

- declaring the shape of a corpus
- compiling bounded runtime context from that corpus
- executing declared transitions over that context
- validating the results
- recording durable execution evidence
- handing bounded continuation to successor work

In short: Earmark makes AI-assisted work explicit enough to audit and constrained enough to govern.

## Core Idea

Instead of giving a runtime a broad repository, a long chat, or a pile of retrieved snippets, Earmark gives it a declared work surface.

A typical staged flow looks like this:

1. Define object classes such as `source_note`, `finding`, and `summary`.
2. Define relations such as `derived_from`.
3. Define standing rules such as draft, reviewed, supported, or completed.
4. Define compiled contexts that compile bounded context.
5. Define transition workflows such as `source_note -> finding -> summary`.
6. Run the workflow.
7. Inspect the resulting assignments, change sets, validations, failures, and handoffs.

The runtime does not continue by remembering the last chat turn. It continues from a canonical handoff manifest.

Learn more: [Concept: Staged Execution](docs/concepts/staged-execution.md) | [Concept: Handoffs](docs/concepts/handoffs.md)

## What Earmark Records

Earmark records runtime work through staged artifacts.

### `TransitionAssignment`

An assignment says: this runtime is doing this transition over this bounded input set.

Assignments make work ownership, continuation, blocking, release, expiration, supersession, and resumption explicit.

### `ChangeSet`

A change set says: this transition created, changed, blocked, or validated these things.

Change sets preserve valid and invalid work. Failed validation and execution errors produce durable failed change sets instead of disappearing.

### `HandoffManifest`

A handoff says: successor work may continue from this bounded surface.

Handoffs carry root objects, inherited inputs, new objects, new relations, allowed classes, allowed relation types, standing constraints, required checks, blocked conditions, and unresolved ambiguities.

### `TransformationFailure`

A failure record says: this transition failed for this reason, linked to the assignment and failed change set.

Failures remain inspectable as canonical state.

## Practical Uses

Earmark is useful when AI work needs traceability, review, staged execution, or controlled continuation.

Good candidate domains:

- research synthesis
- intelligence analysis
- policy review
- contract and compliance workflows
- editorial pipelines
- corpus maintenance
- internal knowledge operations
- safety-sensitive triage where later steps should not see raw input

Example:

```text
source_note -> finding -> summary
```

The first transition extracts findings from source notes and emits a handoff containing only the bounded successor context. The second transition summarizes from that handoff. The summary stage does not need ambient access to the original runtime context.

This fixture is implemented and tested in `earmark-exec/tests/e2e_staged.rs`.

Learn more: [Tutorial: Research Synthesis Demo](docs/tutorials/research-synthesis-demo.md)

## Workspace Layout

The repository is a Rust workspace with kernel crates, outward-facing docs, examples, and templates:

- `earmark-core`: shared types, IDs, standings, declarations, work packets, records, and staged artifacts
- `earmark-store`: Git-backed canonical object storage
- `earmark-index`: rebuildable SQLite index for search, relations, and active systems
- `earmark-declarations`: declaration loading, validation, registration, and activation
- `earmark-exec`: workflow compilation, execution, assignments, change sets, handoffs, and continuation
- `earmark-governance`: review objects, standing transitions, export checks, and governance events
- `earmark-connected-context`: compiled context planning and work-surface materialization
- `earmark-runtime-tools`: runtime-facing API over the shared kernel
- `earmark-cli`: operator-facing command line interface
- `docs/concepts/`: architectural explanations of context compilation, staged execution, handoffs, and failures
- `docs/tutorials/`: guided learning paths, including quickstart and the research synthesis demo
- `docs/reference/`: CLI and artifact reference material
- `docs/declarations/`: declaration examples and schema-oriented authoring material
- `examples/research-synthesis/`: the runnable demo domain definition and walkthrough
- `templates/`: scaffold templates for declaration authoring

## Runtime Storage

An initialized Earmark workspace uses:

- `corpus/`
- `.earmark/canonical/`
- workspace index database path under `.earmark/`
- `.earmark/work_surfaces/`
- `.earmark/declarations/`

Canonical state is authoritative. Derived indexes and work surfaces are rebuildable.

## CLI Overview

All commands support `--json`.

See the complete [CLI Reference](docs/reference/cli.md).

## Build And Test

From the workspace root:

```bash
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

## Documentation

- [Quickstart Tutorial](docs/tutorials/quickstart.md): get moving in 5 minutes
- [Practical Guide](PRACTICAL_GUIDE.md): plain-language explanation and use cases
- [Core Concepts](docs/concepts/): understanding context compilation, staged execution, and handoffs
- [Tutorials](docs/tutorials/): build-run-inspect walkthroughs
- [Reference](docs/reference/): CLI commands, artifact types, and schemas
- [Build a Domain Definition](docs/tutorials/build-a-domain-definition.md): define a domain, classes, workflows, and system manifests
- [Declaration Authoring](docs/declarations/README.md): examples and validation rules
- [Runtime Integration Guide](docs/reference/runtime-integration-guide.md): how to integrate Earmark as a governed execution substrate
- [Runtime Contract](docs/reference/runtime-contract.md): the six-step external contract and artifact shapes

## Acknowledgments

Earmark owes a real architectural debt to [Engram](https://github.com/vincents-ai/engram) and to its creator, Vincent Palmer.

Engram's treatment of durable machine-readable project state, explicit context, and structured agent-operable knowledge helped open the architectural direction that Earmark develops here. Earmark takes that lineage into a more specific kernel for compiled context, staged execution, handoffs, and governed continuation.

## License

AGPL-3.0-or-later OR Commercial.

See [LICENSE](./LICENSE) for the dual-license terms used by this project.
