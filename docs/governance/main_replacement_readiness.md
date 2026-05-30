# Main Replacement Readiness

Status: candidate pending final verification.

This branch replaces the previous v1-preview surface with the hardened kernel baseline.

## Replacement Claim

The repository now presents Earmark as a durable-record kernel rather than as a complete CLI/runtime product.

## Main Replacement

Merged to `main` on 2026-05-30.

The final pre-merge verification commit was `0ffed90eb09f8c52d08c26877d6e884af074031c`. Later commits before merge updated verification documentation only. The authoritative post-merge gate is the passing CI run on `main`.

## Verification Evidence

Verified on: 2026-05-30
Commit: 0ffed90eb09f8c52d08c26877d6e884af074031c

- `cargo fmt --all -- --check`: pass
- `cargo clippy --workspace --all-targets -- -D warnings`: pass
- `cargo check --workspace`: pass
- `cargo test --workspace`: pass

## Verified Gates

- Workspace builds with `cargo check --workspace`.
- Workspace tests pass with `cargo test --workspace`.
- Formatting passes with `cargo fmt --all -- --check`.
- Public docs describe the kernel branch directly.
- Source headers match the public dual-license posture.
- Archive import/export paths are symmetrical with normal store getters.
- The derived SQLite index remains rebuildable from the canonical file store.

## Deferred Work

- Supported CLI/operator shell.
- Full orchestration executor.
- Complete governance enforcement.
- Provider/runtime plugin surfaces.
- Expanded user-facing examples.
