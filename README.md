# Earmark Hardened Kernel (v0.1)

![CI](https://github.com/Mikhail-Shakhnazarov/earmark-workspace/actions/workflows/ci.yml/badge.svg)

AI-assisted work usually starts in a chat window and stays there too long. A task gets split across messages, notes, copied snippets, half-remembered files, and whatever happened to be open at the time. That is fine for a quick answer. It is bad for work that needs to be resumed, checked, reviewed, or handed to someone else.

Earmark is a Rust kernel for keeping that kind of work on durable ground. It stores objects, versions, relations, standing, and review state on disk in a plain directory structure, then builds indexing and governance on top of that record instead of hiding the process inside a service.

## What Lives Here

This repository contains the hardened kernel crates for Earmark.

- `earmark-core`: record types, typed identifiers, and shared data structures
- `earmark-store`: the canonical file-backed store and verification routines
- `earmark-index`: a rebuildable SQLite index for lookup and reporting
- `earmark-declarations`: systems, classes, workflows, and packet templates
- `earmark-governance`: governance-facing types and extraction points

Taken together, these crates define a durable work record that can be stored on disk, inspected directly, indexed for lookup, and checked through explicit governance rules.

## Why The Kernel Matters

Stable state and stable rules should survive changes in tools, providers, and day-to-day runtime habits.

The kernel is written so that the durable record is primary:

- the canonical store is JSON on disk under `.earmark/`
- the derived index can be rebuilt from that store
- the crates keep runtime assumptions out of the core record
- review and standing are part of the data model, not comments around it

That makes the kernel useful anywhere work needs custody, traceability, and a clean handoff path.

## Repository Layout

```text
.
|-- Cargo.toml
|-- Cargo.lock
|-- earmark-core/
|-- earmark-store/
|-- earmark-index/
|-- earmark-declarations/
|-- earmark-governance/
|-- docs/
|   |-- architecture.md
|   |-- limitations.md
|   `-- governance/
|-- LICENSE
```

## Start Here

- `docs/architecture.md`: what the kernel does and how the pieces fit together
- `docs/limitations.md`: the current rough edges and present scope of the code
- `docs/kernel_contract.md`: the formal guarantees for v0.1 records
- `docs/governance/README.md`: the release rules and hardening notes

## Build And Test

The repository builds from source with ordinary Rust tooling.

```bash
cargo check --workspace
cargo test --workspace
```

## License

This repository is dual-licensed under `AGPL-3.0-or-later` and a commercial license. See `LICENSE`.
