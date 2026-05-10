# Store / Index Contract

## Durable State

The canonical store lives under `.earmark/canonical/` and is backed by Git. Every object version is stored as:

- `.earmark/canonical/objects/{object_id}/{version_id}/envelope.json`
- `.earmark/canonical/objects/{object_id}/{version_id}/payload.{json|md|yaml}`
- `.earmark/canonical/heads/{object_id}.json` — points to the current head version

Payloads are content-addressed under `.earmark/canonical/payloads/` and deduplicated across versions.

The canonical store is the source of truth. If the derived index is lost or corrupted, no durable state is harmed.

## Derived Index

The derived index lives at `.earmark/derived/index.sqlite` (SQLite). It exists for fast query and inspection. The index tracks:

| Table | Purpose |
|---|---|
| `objects` | All object versions with parsed metadata |
| `heads` | Current head version per object |
| `relations` | Parsed relation edges |
| `active_systems` | Activated system definitions |
| `active_assignment_claims` | Active transition assignment locks |

The index is always rebuildable from canonical state.

## Coherence

Writes through the normal persistence path (`write_object_and_index` / `write_batch_and_index`) update both the canonical store and the derived index atomically (store write first, index update second). If the index update fails, the error is propagated and the canonical write is preserved.

Some operations intentionally write to the canonical store without updating the index (e.g., sub-declarations during system manifest registration). In those cases a full `rebuild_from_store` is called afterward to bring the index up to date.

## Health Checks

Run `em doctor` to inspect store and index health:

| Field | Meaning |
|---|---|
| `store_scan_ok` | Canonical store can be scanned without errors |
| `canonical_object_count` | Total object versions found in canonical store |
| `index_exists` | Index SQLite file exists |
| `index_open_ok` | Index opens without errors |
| `indexed_object_count` | Total object versions in index |
| `indexed_head_count` | Total head entries in index |
| `counts_match` | Canonical object count equals indexed object count |

If `counts_match` is `false`, the index is stale or inconsistent with the canonical store.

## Rebuilding the Index

The derived index can be rebuilt from canonical state:

- **Programmatic**: `index.rebuild_from_store(&store)`
- **CLI**: registering a system definition (`em system register`) triggers a full rebuild
- **Manual**: any write command that opens the index in write mode will recreate it if missing

Rebuilding does not destroy active system registrations or active assignment claims.

## Key Rules

1. The canonical store never depends on the derived index.
2. The derived index can always be rebuilt from the canonical store.
3. A write through the standard helper updates both layers.
4. `em doctor` reports drift; it does not repair it.
5. Accidental index loss requires no data recovery — only a rebuild.
