# Local Runtime Tickets

STATUS: IMPLEMENTATION TICKETS
SCOPE: LOCAL OPENCODE EXECUTION
PROJECT: Earmark Workspace

These tickets are narrow implementation packets for the local runtime. They are not architecture proposals, roadmap notes, or public documentation improvements. Their only purpose is to restore the current green CI baseline by addressing the observed Clippy failures.

Use one ticket at a time. Apply only the requested edit, then run the listed verification commands. Do not combine these tickets with dependency updates, workflow edits, documentation cleanup, or behavior changes.

## Current failing command

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

## Ticket order

1. `001-restore-clippy-governance.md`
2. `002-restore-clippy-index.md`

## Final verification after all tickets

```bash
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

If any command fails with a new warning not covered by these tickets, stop and capture the exact output before making further edits.

© 2026 Mikhail Shakhnazarov
