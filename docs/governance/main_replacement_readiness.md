# Main Replacement Readiness

Status: candidate pending final verification.

This branch replaces the previous v1-preview surface with the hardened kernel baseline.

## Replacement Claim

The repository now presents Earmark as a durable-record kernel rather than as a complete CLI/runtime product.

## Verified Gates

- Workspace builds with `cargo check --workspace`.
- Workspace tests pass with `cargo test --workspace`.
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
