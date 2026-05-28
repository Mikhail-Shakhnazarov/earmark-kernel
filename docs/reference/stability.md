# Stability Catalog

This document provides an overview of the current implementation status and stability guarantees for Earmark's core components, CLI commands, and documentation.

## Classification Labels

| Label | Meaning |
|---|---|
| `core` | Essential for the durable work spine. |
| `supporting` | Tools for integration or inspection. |
| `stable` | Operational; reliable for development and dogfooding. |
| `beta` | Functional but may undergo minor semantic refinements. |
| `experimental` | Usable for dogfooding but subject to structural changes. |

## Core Components (Crates)

| Component | Class | Description | Status |
|---|---|---|---|
| `earmark-core` | core | Domain model, IDs, and serialization primitives. | Stable |
| `earmark-store` | core | Git-backed durable storage and write locking. | Stable |
| `earmark-index` | core | Query projection and relation visibility. | Stable |
| `earmark-exec` | core | Staged execution and relation authorization. | Stable |
| `earmark-cli` | core | Primary operator interface. | Stable |
| `earmark` | supporting | In-process Rust developer API. | Stable |

## CLI Command Stability

| Command family             | Stability | Purpose                                      |
| -------------------------- | --------- | -------------------------------------------- |
| `init`                     | Stable    | Workspace management.                        |
| `status`                   | Stable    | Workspace status.                            |
| `deposit`                  | Stable    | Object creation.                             |
| `query`                    | Stable    | Object/relation lookup.                      |
| `review`                   | Stable    | Governance review.                           |
| `system`                   | Stable    | System definition lifecycle.                 |
| `workflow`                 | Stable    | Staged execution.                            |
| `run`                      | Stable    | Run inspection.                              |
| `assignment`               | Stable    | Task assignment.                             |
| `changeset`                | Stable    | Change set management.                       |
| `handoff`                  | Stable    | Handoff inspection.                          |
| `failure`                  | Stable    | Failure inspection.                          |
| `context`                  | Stable    | Context compilation.                         |
| `relation`                 | Stable    | Relation management.                         |
| `standing-request`         | Stable    | Standing lifecycle.                          |
| `report`                   | Stable    | Report generation.                           |
| `commands`                 | Stable    | Command catalog.                             |
| `orchestration`            | Stable    | Native workspace self-hosting.               |
| `doctor`                   | Beta      | Workspace diagnostics.                       |
| `declare`                  | Beta      | Declaration registration.                    |
| `audit`                    | Beta      | Aggregated diagnostic views.                 |
| `provider`                 | Beta      | Provider integration.                        |
| `completions`              | Beta      | Shell integration.                           |
| `undo`                     | Beta      | Compensating visibility.                     |

## Documentation Flow

| Ordered Step | Document | Purpose |
|---|---|---|
| 1 | `README.md` | Core vision and installation. |
| 2 | `docs/tutorials/quickstart.md` | 5-minute first run (deterministic). |
| 3 | `docs/reference/stability.md` | Implementation status (you are here). |
| 4 | `docs/reference/cli.md` | Command engineering. |
| 5 | `docs/limitations.md` | Known constraints. |

---

## Current Maturity

Earmark is **pre-release software**. It has completed its initial hardening phase, including:
- **Canonical Relation Authorization**: Guarantees that only declared relations can be created.
- **Transactional Index Rebuilds**: Ensures consistent workspace visibility.
- **Durable Orchestration Ledger**: Provides self-hosting task management with full causality tracking.
- **Git-Backed Durability**: Leverages proven version control for the work spine.

The kernel and native orchestration are usable for coordinated AI work. The native orchestration surface is now **Stable**, providing a reliable ledger for self-hosting development coordination and task management.
