# Earmark — v1 Preview (v0.1.0)

[![CI](https://github.com/Mikhail-Shakhnazarov/earmark-workspace/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/Mikhail-Shakhnazarov/earmark-workspace/actions/workflows/ci.yml)

Earmark is a local-first, Git-backed work spine for coordinated AI processing. 

AI work usually runs on ephemeral chat history. This leads to common failure modes:
- **Context Bleed:** A synthesis step sees original source noise it shouldn't have inherited.
- **Context Bloat:** Models receive thousands of irrelevant lines instead of specific findings.
- **Lost Lineage:** Work that cannot be traced, audited, or reliably resumed.

**Earmark replaces ephemeral history with a durable, inspectable work spine.**

---

## The Vision: Coordinated AI Work

Instead of one long conversation, Earmark breaks work into **coordinated transitions**. Each step sees only the specific **task materials** (objects) it is allowed to see. It then records the authoritative result back into the workspace.

```text
Source Notes → [Extraction Stage] → Findings → [Synthesis Stage] → Summary
```

1. **Extraction:** Receives raw notes. Produces findings. Links them to sources.
2. **Synthesis:** Receives *only* the validated findings. Cannot see the original notes.
3. **Outcome:** A summary derived strictly from authoritative evidence, not ambient noise.

---

## Installation

The primary way to install the Earmark operator shell (`em`) is via Cargo:

```bash
cargo install --path earmark-cli
```

*Nix users: A runnable app is available via the included `flake.nix` (`nix run .`).*

*NixOS note: run build and test commands through the provided dev shell (`nix develop` or `nix develop --command ...`) so OpenSSL and `pkg-config` are wired correctly.*

---

## Quickstart (The 5-Minute Demo)

Initialize a workspace and run a deterministic research synthesis demo offline:

```bash
# 1. Initialize
mkdir my-work && cd my-work
em init

# 2. Register example domain
em system register ../examples/research-synthesis/declarations/systems/system.yaml
em system activate sys_research_synthesis

# 3. Deposit a note
em deposit --class source_note --title "Context Limits" --body "AI context should be task-specific."

# 4. Run the workflow
# (Replace <object_id> with the ID returned by 'em query --class source_note')
em workflow run research_synthesis --system-id sys_research_synthesis --with <object_id>

# 5. Inspect
em run explain latest
em report run latest --output report.html
```

---

## Documentation

- **[Quickstart Guide](docs/tutorials/quickstart.md)** — Step-by-step through your first run.
- **[Limitations](docs/limitations.md)** — What Earmark is (and isn't) today.
- **[Stability Catalog](docs/reference/stability.md)** — Maturity of commands and crates.
- **[CLI Reference](docs/reference/cli.md)** — Command lookup and schema guides.

---

## Status: v1 Preview

Earmark is **pre-1.0 software**. This preview (v0.1.0) packages the local execution kernel and the native orchestration ledger for developer dogfooding.

- **Included:** Git-backed storage, staged execution, relation authorization, HTML reporting, and native orchestration.
- **Excluded:** Hosted services, multi-user sync, and dynamic plugin loading.

---

## Acknowledgments

Earmark draws architectural inspiration from systems that prioritize durable, explicit context, such as [Engram](https://github.com/vincents-ai/engram). Earmark continues this development as a fully native, Git-backed architecture designed for high-integrity AI coordination.

---

## License

AGPL-3.0-or-later OR Commercial. See [LICENSE](./LICENSE).
