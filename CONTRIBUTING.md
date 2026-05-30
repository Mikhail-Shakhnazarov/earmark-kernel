# Contributing

Earmark Hardened Kernel is public, pre-1.0 software. Small bug fixes,
documentation corrections, tests, and tightly scoped crate improvements are the
easiest changes to review. Larger design changes should begin as an issue.

## Local Setup

Build from a source checkout at the repository root:

```bash
cargo build --workspace
```

Inspect the workspace with:

```bash
cargo check --workspace
cargo test --workspace
```

If the local environment uses Nix, `nix develop` provides a shell with the Rust
toolchain and OpenSSL-related dependencies configured.

## Verification

Before opening a pull request, run:

```bash
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
```

If a command cannot be run, say which command was skipped and why.

## Issues First For Design Changes

Open an issue before implementing changes that affect:

- record formats
- storage layout
- declaration semantics
- index behavior
- governance semantics
- public documentation structure
- compatibility expectations

A good design issue states the current behavior, the proposed change,
alternatives considered, and the compatibility risk.

## Documentation Changes

Public documentation should explain the kernel as a standalone project. It
should stay concrete, mechanically accurate, and free of release-planning
residue.

Tutorial material should show expected outcomes after commands. Reference
material should match the repository that exists.

## Pull Requests

A pull request should include:

- a short summary of the change
- the scope of files or behavior affected
- verification commands run
- tests added or updated, or why tests do not apply
- documentation updated, or why documentation does not apply
- compatibility notes when behavior changes

Keep unrelated changes out of the same pull request.

## Licensing Note

The repository is distributed under the terms in `LICENSE`. By contributing,
the contributor agrees that accepted changes may be distributed under those
terms.

This repository does not currently use a separate CLA or DCO.

Copyright (c) 2026 Mikhail Shakhnazarov
