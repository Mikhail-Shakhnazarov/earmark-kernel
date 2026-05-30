# Acceptance Gates

The release standard for this repository is simple: the code must build, the
rules must still be true, and the public docs must match the software that is
actually here.

## Gate A: Build And Test

The whole workspace must pass:

```bash
cargo check --workspace
cargo test --workspace
```

If a future public release wants stricter compiler or lint gates, those can be
added here. The important part is that the published repository can verify
itself with ordinary Rust tooling.

## Gate B: Boundary Hygiene

Kernel crates must remain platform-neutral and durable-first. In practice that
means the core crates should preserve a clean separation between persistent
state, derived lookup, and governance logic.

## Gate C: Durable Record

The canonical file-backed store remains primary, and the derived index remains
rebuildable from that store.

If that relationship breaks, the main architectural claim of the project no
longer holds.

## Gate D: Documentation Match

The public docs must describe the repository that exists.

That includes:

- no stale planning residue in the public docs surface
- no links to missing files
- no references that confuse this repository with some other build or runtime
