# Release Notes: Earmark v0.1.0 (v1 Preview)

This release marks the first public packaging of Earmark, a local-first work spine for coordinated AI processing. 

## Key Highlights

- **Binary Branding**: The operator CLI has been renamed to `em` for a streamlined interaction flow.
- **Durable Work Spine**: Full implementation of the Git-backed canonical store and SQLite-based derived index.
- **Staged Execution**: Support for `compile_context` and `transform` operations with explicit contract boundaries.
- **Deterministic Quickstart**: A flagship research synthesis demo that runs 100% locally with zero configuration.
- **Native Orchestration**: A stable, self-hosting task management ledger for coordinating complex AI development programs.
- **Interactive Reports**: HTML-based run reporting with visual timelines and full provenance tracking.

## Technical Details

- **CLI Contract Version**: Aligned at `0.3.0`.
- **Core Crates**: `earmark-core`, `earmark-store`, `earmark-exec`, and `earmark-index` are now baseline-stable.
- **Nix Support**: Standardized `flake.nix` exposing `em` as a runnable package and app.

## Credits & Inspiration

Earmark draws from the design principles of systems like [Engram](https://github.com/vincents-ai/engram), evolving them into a native, high-integrity architecture for local AI sovereignty.

---
*Visit the [Quickstart Guide](docs/tutorials/quickstart.md) to begin.*
