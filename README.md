# Earmark: Coordinated AI Work

[![CI](https://github.com/Mikhail-Shakhnazarov/earmark-workspace/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/Mikhail-Shakhnazarov/earmark-workspace/actions/workflows/ci.yml)

AI work usually runs on ephemeral chat history: some retrieved snippets, a long conversation, and whatever files are open. This works for quick questions. It falls apart when work needs to be **inspected, resumed, reviewed, or handed over.**

Without a durable spine, AI work faces common failure modes:
- **Implicit Inheritance:** A later step silently "inherits" an assumption from an earlier prompt.
- **Context Bloat:** A model sees 10,000 lines of history when it only needed 10 specific findings.
- **Lost Evidence:** Failed work or intermediate derivations vanish when a session ends.
- **Review Gap:** Approval is just a social convention in a chat, not a trackable system state.

**Earmark replaces ephemeral history with a durable, inspectable work spine.**

---

## How It Works

Earmark lets you define exactly which **task materials** (objects) each step is allowed to see. It then compiles a **task-specific input set**, runs the operation, and records the authoritative result.

### The Problem: Ephemeral Context
```text
Conversation: [Note A, Note B, Note C] → AI finds Finding 1 → (Prompt continues) → AI writes Briefing
```
In a standard chat, the "Briefing" step sees all three notes, the finding, and any noise in between. Assumptions from Note A might bleed into the Briefing even if they were irrelevant to Finding 1.

### The Earmark Solution: Bounded Transitions
```text
Note A → [Finding 1] → Briefing
```
1. **Stage 1 (Extraction):** Receives Note A. Produces Finding 1. Records exactly what changed.
2. **Stage 2 (Synthesis):** Receives *only* Finding 1. It cannot see Note A.
3. **Outcome:** The synthesis is guaranteed to be based only on the authoritative findings, not on ambient noise from the source.

---

## Core Capabilities

- **Task-Specific Inputs:** Every processing step sees only what its declaration allows. No more silent context inheritance.
- **Durable Provenance:** Every result links back to its specific causes. You can always prove *why* an output exists.
- **Durable Failure:** Failed work is preserved for audit, not discarded.
- **Git-Backed History:** Workspace state is stored in a standard Git repository, providing a full audit trail and easy portability.
- **Native Orchestration:** Self-hosting tools for tracking complex, multi-stage AI work programs.

---

## Documentation (The Flow)

If you are new to Earmark, follow this path:

| Order | Guide | Purpose |
| :--- | :--- | :--- |
| 1 | **README** (You are here) | High-level "Should I use this?" |
| 2 | **[One-Minute Flow](docs/tutorials/one-minute-flow.md)** | See it in action immediately |
| 3 | **[Quickstart](docs/tutorials/quickstart.md)** | Your first successful run in 5 minutes |
| 4 | **[Practical Guide](docs/tutorials/practical-guide.md)** | Concrete examples and plain-language "Why" |
| 5 | **[Research Synthesis Demo](docs/tutorials/research-synthesis-demo.md)** | A complete, multi-stage example |
| 6 | **[Build a Domain](docs/tutorials/build-a-domain-definition.md)** | Create your own classes and workflows |
| 7 | **[Concepts Overview](docs/concepts/coordinated-ai-work.md)** | Deep dive into the "Durable Spine" philosophy |
| 8 | **[CLI Reference](docs/reference/cli.md)** | Command and schema lookup |
| 9 | **[Limitations](docs/limitations.md)** | Known constraints and WIP status |

---

## Getting Started

```bash
# Build the CLI
cargo build -p earmark-cli
alias em="$(pwd)/target/debug/earmark-cli"

# Initialize a workspace
mkdir my-workspace && cd my-workspace
em init

# Register a demo system
em system register ../examples/research-synthesis/declarations/systems/system.yaml
em system activate sys_research_synthesis

# Deposit work materials
em deposit --class source_note --title "Architecture Note" --body "AI context must be limited."

# Run a coordinated task
em workflow run research_synthesis --system-id sys_research_synthesis --with <object_id>
```

---

## Technical Architecture

Earmark is built in Rust and uses:
- **Canonical Store:** File-based, Git-backed object storage for durability.
- **Derived Index:** A local SQLite index for fast search and lifecycle tracking.
- **Declared Types:** YAML-based schemas for objects, relations, and instructions.

### Current Status

Earmark is **Stable** for local development and orchestration but remains pre-release. 
- **What Works:** Full ingestion, staged execution, relation authorization, and native orchestration.
- **WIP:** Multi-actor coordination, advanced provider plugins, and web-based observability.

See **[Stability Catalog](docs/reference/stability.md)** for a detailed look at the current implementation status.

---

## Acknowledgments

Earmark draws architectural inspiration from systems that prioritize durable, explicit context, such as [Engram](https://github.com/vincents-ai/engram). Earmark continues this development as a fully native, Git-backed architecture designed for high-integrity AI coordination.

---

## License

AGPL-3.0-or-later OR Commercial. See [LICENSE](./LICENSE).