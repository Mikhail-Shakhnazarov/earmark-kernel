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

| Command | Stability | Purpose |
|---|---|---|
| `init`, `status` | Stable | Workspace management. |
| `deposit`, `query` | Stable | Data lifecycle. |
| `declare`, `system` | Stable | Domain definition. |
| `workflow`, `relation` | Stable | Managed relationship graph between objects. |
| `orchestration` | Stable | Native workspace management and self-hosting. |
| `report`, `failure` | Stable | Observability and audit. |
| `undo` | Beta | Compensating visibility. |
| `provider` | Beta | External model integration. |

## Documentation Flow

| Ordered Step | Document | Purpose |
|---|---|---|
| 1 | `README.md` | Entry point. |
| 2 | `docs/tutorials/quickstart.md` | 5-minute first run. |
| 3 | `docs/tutorials/practical-guide.md`| Concepts through examples. |
| 4 | `docs/concepts/coordinated-ai-work.md` | Design philosophy. |
| 5 | `docs/reference/cli.md` | Command engineering. |
| 6 | `docs/limitations.md` | Known constraints. |

---

## Current Maturity

Earmark is **pre-release software**. It has completed its initial hardening phase, including:
- **Canonical Relation Authorization**: Guarantees that only declared relations can be created.
- **Transactional Index Rebuilds**: Ensures consistent workspace visibility.
- **Durable Orchestration Ledger**: Provides self-hosting task management with full causality tracking.
- **Git-Backed Durability**: Leverages proven version control for the work spine.

The kernel and native orchestration are usable for coordinated AI work. The native orchestration surface is now **Stable**, providing a reliable ledger for self-hosting development coordination and task management.
