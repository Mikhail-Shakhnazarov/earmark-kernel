# Earmark Canonical Spine

© 2026 Mikhail Shakhnazarov

## Status

Authoritative architecture note for current pre-release development.

This document defines the project spine that should remain stable while implementation details, integrations, and experimental dogfooding surfaces evolve.

## Definition

The canonical spine is the minimal durable path through which Earmark turns declared domain structure into inspectable work state.

The spine is not every useful tool in the repository. It is the architectural route that must remain coherent, governed, and difficult to bypass.

## Spine

1. **Declarations** define object classes, relation rules, workflows, standing policies, compiled contexts, provider profiles, and systems.
2. **Context compilation** produces bounded work surfaces from declared templates and canonical objects.
3. **Staged execution** consumes bounded inputs and records assignments, work packets, change sets, handoffs, failures, provider records, governance events, and output objects.
4. **Canonical storage** records durable object versions and payloads.
5. **Derived indexing** projects canonical state for query, audit, and inspection.
6. **Operator interfaces** expose controlled actions over the canonical spine, primarily through the CLI.

In compact form:

```text
declarations
  -> bounded context / work surface
  -> staged execution
  -> durable artifacts
  -> derived index
  -> query / audit / report
```

## Canonical write paths

All durable writes must pass through sanctioned creation paths. A sanctioned creation path must perform validation, authorization, persistence, and index update consistently.

Canonical write paths include:

- object deposit;
- declaration registration;
- relation creation;
- workflow execution artifact persistence;
- review and standing changes;
- undo records.

A durable object, relation, or governance artifact should not enter canonical state through an ad hoc helper that bypasses the relevant validation or authorization step.

## Relation to `.earmark/canonical`

The phrase “canonical spine” is architectural. It is related to, but broader than, the `.earmark/canonical` storage directory.

`.earmark/canonical` is the persisted substrate. The canonical spine is the set of declared semantics and sanctioned write paths that make that substrate trustworthy.

## Non-spine surfaces

The following are not part of the canonical spine unless explicitly promoted:

- local orchestration experiments;
- OpenCode dispatch scripts;
- Engram adapters;
- shell-out facade wrappers;
- demo-only declarations;
- speculative async/provider seams.

These surfaces may remain only when clearly marked as experimental, internal, compatibility, or test support.

## Classification rule

Every repository surface should be classified as one of:

- `core` — required for the canonical spine;
- `supporting` — not central, but needed to operate or inspect the core;
- `experimental` — useful but not product-stable;
- `compatibility` — retained to bridge old or external usage;
- `internal-tooling` — development tooling, not product surface;
- `test-fixture` — required by tests only;
- `delete-candidate` — stale, duplicate, misleading, or superseded.

## Promotion rule

Experimental or compatibility surfaces may be promoted into the canonical spine only after they satisfy the same constraints as the core:

1. explicit declarations or schema;
2. sanctioned write paths;
3. validation and authorization at the write boundary;
4. index/report/query visibility;
5. tests for failure and boundary cases;
6. documentation that describes current behaviour rather than aspiration.

## Hard rule

No experimental, internal, or compatibility surface may silently bypass a canonical write path.
