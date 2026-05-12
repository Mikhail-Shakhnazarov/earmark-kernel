# Earmark CLI Machine-Readable Contracts

The Earmark CLI supports a `--json` flag for machine-readable output. All JSON output follows a consistent envelope format to ensure reliability and ease of parsing for automation tools.

## Versioning

The current CLI contract version is `0.2.0`. This version is specified in the `contract_version` field of every JSON response.

## JSON Envelope

All successful and unsuccessful CLI operations in JSON mode return a standard envelope.

### Successful Response

```json
{
  "contract_version": "0.2.0",
  "ok": true,
  "data": {
    // Command-specific data
  }
}
```

### Error Response

```json
{
  "contract_version": "0.2.0",
  "ok": false,
  "error": {
    "message": "Human-readable error message",
    "code": "OPTIONAL_ERROR_CODE"
  }
}
```

## Command-Specific Data Structures

### `standing-request` Family

#### `list`
Returns an array of standing request summaries.
```json
{
  "data": [
    {
      "id": "obj_...",
      "status": "proposed|approved|rejected|applied",
      "target_id": "obj_...",
      "dimension": "...",
      "from": "...",
      "to": "..."
    }
  ]
}
```

#### `show`
Returns full details of a standing request.
```json
{
  "data": {
    "ok": true,
    "id": "obj_...",
    "request": {
      "target_object_id": "obj_...",
      "dimension": "...",
      "from_value": "...",
      "to_value": "...",
      "status": "...",
      "rationale": "..."
    }
  }
}
```

#### `approve` / `reject`
Returns the status of the operation.
```json
{
  "data": {
    "ok": true,
    "id": "obj_...",
    "status": "approved|rejected"
  }
}
```

#### `apply`
Returns the resulting version IDs.
```json
{
  "data": {
    "ok": true,
    "status": "applied",
    "target_id": "obj_...",
    "next_target_version": "ver_...",
    "next_request_version": "ver_..."
  }
}
```

### `explain` Family

The `explain` commands provide an interpreted view of objects, including summaries and related items.

```json
{
  "data": {
    "ok": true,
    "kind": "run|assignment|change_set|handoff|failure|relation",
    "id": "...",
    "summary": "Short human-readable summary",
    "artifact": { /* Original object payload */ },
    "related": {
      "run_id": "...",
      // Other kind-specific relations
    },
    "next_commands": [
      {
        "command": "em ...",
        "description": "..."
      }
    ]
  }
}
```

## Best Practices for Consumers

1. **Always check `ok`**: Ensure the operation succeeded before parsing `data`.
2. **Verify `contract_version`**: Fail or warn if the contract version is not recognized.
3. **Handle missing fields**: New fields may be added in minor versions; code should be resilient to additional fields.
