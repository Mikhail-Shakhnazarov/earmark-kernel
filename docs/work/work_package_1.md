# Work Packet: Relation Authorization Explain Surface

STATUS: IMPLEMENTATION PACKET
BRANCH: feature/relation-auth-explain
BASE: dev
PROJECT: earmark-workspace
OWNER: Mikhail Shakhnazarov
© 2026 Mikhail Shakhnazarov

## Intent

Make relation authorization inspectable from the operator surface.

The runtime already authorizes relation creation and persists authorization evidence into relation object headers. The operator should be able to inspect a relation and see why it was allowed: source rule, target rule, either-endpoint rule, bidirectional rule, or privileged system relation.

This feature does not change relation authorization semantics. It exposes and tests the existing decision trace.

## Current Evidence

Known current implementation points:

* `earmark-runtime-tools/src/modules/relations.rs`

  * `create_relation(...)` reads source and target heads.
  * It calls `authorize_relation_creation(...)`.
  * It writes headers:

    * `relation_auth_endpoint`
    * `relation_auth_class`
    * `relation_auth_authority`
    * `relation_auth_direction`

* `earmark-exec/src/relation_logic.rs`

  * `RelationAuthorizationResolver` supports:

    * source outgoing rule
    * target incoming rule
    * source/target bidirectional rule
    * either endpoint source/target rule
    * privileged system relation

* `earmark-declarations/src/lib.rs`

  * declaration validation rejects invalid relation directions
  * declaration validation rejects invalid authorizing endpoints
  * declaration validation rejects dead direction/authority combinations

* `earmark-cli/src/cli.rs`

  * CLI already declares:

    * `em relation show <relation_id>`
    * `em relation explain <relation_id>`
    * `em relation list ...`

## Scope

Implement or harden the explain surface for relation authorization.

Expected CLI JSON shape for `em relation explain <relation_id>`:

```json
{
  "ok": true,
  "kind": "relation",
  "id": "obj_...",
  "summary": "relation <id> <relation_type> from <source_class> to <target_class>",
  "artifact": {},
  "related": {
    "source": {
      "object_id": "obj_...",
      "version_id": "ver_...",
      "class": "source_note",
      "kind": "object"
    },
    "target": {
      "object_id": "obj_...",
      "version_id": "ver_...",
      "class": "finding",
      "kind": "object"
    },
    "relation_type": "mentions",
    "creation_mode": "declared",
    "authorization": {
      "authority": "source",
      "endpoint": "source",
      "class": "source_note",
      "direction": "outgoing"
    }
  },
  "next_commands": [
    "em query --object-id <source_id>",
    "em query --object-id <target_id>"
  ]
}
```

The exact existing output structure may differ. Preserve the current CLI conventions unless there is a clear reason to change them. The key requirement is that authorization evidence is present, stable, and test-covered.

## Required Behavior

### 1. Relation explain includes authorization evidence

For relations created through `RuntimeToolSurface::create_relation`, `em relation explain <relation_id>` should surface:

* relation type
* source id, version id, class, kind
* target id, version id, class, kind
* relation creation mode
* authorization endpoint
* authorization class
* authorization authority
* authorization direction

If authorization headers are missing, the command should not panic. It should expose `authorization: null` or an equivalent explicit absence state.

### 2. Relation show remains raw or near-raw

`em relation show <relation_id>` should remain close to artifact display. Avoid overloading `show` with explanatory derived fields unless that is already the project convention.

### 3. Relation list remains filter-oriented

`em relation list` should continue to list/filter relation objects. It does not need full explanation for every relation unless the current implementation already does that cheaply.

### 4. No semantic change to authorization

Do not alter the resolver’s allow/block logic unless a bug is discovered. This packet is about visibility and test coverage.

## Likely Files

Inspect before editing:

* `earmark-cli/src/cli.rs`
* `earmark-cli/src/app.rs`
* any relation handler module if one exists under `earmark-cli/src/handler/`
* `earmark-runtime-tools/src/modules/relations.rs`
* `earmark-runtime-tools/src/tests.rs`
* `earmark-exec/tests/wp02_relations.rs`
* `docs/reference/cli.md`
* `docs/declarations/README.md`
* possibly `docs/concepts/` for relation authorization explanation

## Tests

Add or harden tests at the narrowest useful level.

Minimum expected tests:

1. Runtime tool creates a declared relation authorized by source rule and persists authorization headers.
2. Runtime tool creates a declared relation authorized by target incoming rule and persists target authorization headers.
3. Runtime tool creates an either-endpoint-authorized relation and persists `relation_auth_authority = either_endpoint`.
4. CLI relation explain output includes authorization data for a created relation, if CLI test harness exists.
5. Declaration validation rejects:

   * invalid `direction`
   * invalid `authorizing_endpoint`
   * dead `outgoing + target`
   * dead `incoming + source`

Some declaration tests may already exist. Do not duplicate; extend only if coverage is missing.

## Documentation

Add a concise documentation note explaining relation authorization in reader-facing language.

Use project documentation style:

* Start from the problem: relation creation should be explainable after the fact.
* State what Earmark records.
* Show one source-authorized example and one target-authorized example.
* Avoid defensive “not X” framing.
* Avoid inside-out prose about what the page is doing.

Candidate location:

* `docs/declarations/README.md`, if relation rules already live there.
* Otherwise create `docs/concepts/relation-authorization.md` and link it from the docs table if appropriate.

Use the actual field name `authorizing_endpoint`, not the older planning phrase `relation_authority`, unless the code is intentionally renamed. Do not rename the field as part of this packet.

## Acceptance Commands

Run:

```bash
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

If Clippy or full workspace tests fail for unrelated pre-existing reasons, report exact failures and run the narrow relevant crate tests.

## Non-Goals

Do not implement dynamic plugins.

Do not migrate the index.

Do not make provider execution async.

Do not redesign relation semantics.

Do not introduce fallback authorization behavior.

Do not rename `authorizing_endpoint` unless a separate migration packet is approved.

## Deliverable Back to Coordinator

Return:

1. Changed files
2. Summary of behavior added
3. Tests added or modified
4. Exact commands run
5. Exact test results
6. Any unresolved ambiguity
7. Any suggested follow-up issue

## Coordination Rule

The first pass should be small and strict. If OpenCode finds the CLI already exposes all authorization fields, the implementation should stop and convert the packet into a test + documentation hardening pass. That is not failure; that is correct workflow behavior. The goal is not to force code churn. The goal is to make the project’s existing semantic capability durable, visible, and reviewable.
