# Contributing

Earmark is public, pre-release software. Small bug fixes, documentation corrections, and tests are easiest to review. Large behavior changes should start as an issue before implementation.

## Local Setup

Build from a source checkout:

```bash
cargo build -p earmark-cli
```

Run the CLI from the repository root:

```bash
cargo run -p earmark-cli -- --help
```

A direct built-binary path may also be used after `cargo build`, but the exact path is platform-dependent.

## Verification

Before opening a pull request, run the workspace verification path:

```bash
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

If a command cannot be run, say which command failed or was skipped and why.

## Issues First for Design Changes

Open an issue before implementing changes that affect command behavior, storage layout, declaration semantics, workflow execution, public documentation structure, provider integration, or compatibility.

A good design issue states the current behavior, the proposed change, alternatives considered, affected surfaces, and compatibility risk.

## Documentation Changes

Public documentation should be concrete and mechanically accurate. Tutorials should show what the reader should expect to see after commands. Reference pages should match the live command surface and avoid describing planned behavior as current behavior.

Use plain language where it preserves precision. Keep project-specific terms only where they do real work.

## Pull Requests

A pull request should include:

* a short summary of the change
* the scope of files or behavior affected
* verification commands run
* tests added or updated, or why tests do not apply
* documentation updated, or why documentation does not apply
* compatibility or breaking-change notes
* a linked issue when the change began as one

Keep unrelated changes out of the same pull request.

## Licensing Note

The repository is distributed under the license terms stated in `LICENSE`. By opening a pull request, contributors should be prepared for their contribution to be distributed under those stated terms. The project owner may require explicit confirmation before accepting substantial contributions.

This repository does not currently use a separate CLA or DCO.

© 2026 Mikhail Shakhnazarov
