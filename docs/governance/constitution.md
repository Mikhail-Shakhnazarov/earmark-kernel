# Constitution

The kernel exists to keep governed work durable, inspectable, and portable. If
that goal is compromised, the kernel stops being the kernel and turns into just
another runtime-dependent toolchain.

## Non-Negotiable Boundaries

1. The kernel stays independent of workstation-specific runtime assumptions.
2. The canonical store is the source of truth. Derived layers may help with
   speed or lookup, but they do not own the record.
3. Review, standing, and lifecycle state belong to the system itself. They are
   not optional annotations added later.
4. The kernel crates remain composable and understandable as a coherent Rust
   workspace.
5. The repository must stay buildable and testable with ordinary local Rust
   tooling.

## Why These Rules Exist

Without those limits, the project becomes harder to inspect and easier to lose.
A durable record should not depend on one shell script, one local service, or
one still-running process.

That is the standard this repository is trying to meet.
