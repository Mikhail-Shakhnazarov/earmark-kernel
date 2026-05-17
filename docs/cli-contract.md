# Earmark CLI Contract

Current version: `0.3.0`

## JSON Envelope

All JSON output uses a standard top-level envelope:

### Success

```json
{
  "contract_version": "0.3.0",
  "ok": true,
  "data": { ... }
}
```

### Error

```json
{
  "contract_version": "0.3.0",
  "ok": false,
  "error": {
    "message": "human-readable description",
    "code": "error_kind"
  }
}
```

Error `code` values correspond to `CliError::kind_str()`:
- `store`, `index`, `derive`, `exec`, `governance`, `core`
- `json`, `yaml`, `toml`, `io`
- `not_found`, `argument`, `workspace_not_initialized`, `runtime`

## Thread-Local Context

The CLI sets a thread-local `CliContext` before dispatching any command. It carries:
- `command_name` — the resolved command family name (e.g. `"workflow"`, `"run"`, `"status"`)
- `as_json` — whether `--json` mode is active

Context is accessible via `output::with_context()` or the convenience helpers `output::as_json_mode()` and `output::command_name()`.

## Stream Policy

- All JSON envelopes are emitted to **stdout**.
- `stderr` is reserved for diagnostic tracing logs and system panics.
- Consumers should parse stdout line-by-line as JSON.

## Envelope Functions

| Function | Purpose |
|---|---|
| `emit_json_envelope(value)` | Wraps `value` in `{ ok: true, data: value }` |
| `emit_error_envelope(message)` | Wraps `message` in `{ ok: false, error: { message } }` |
| `emit_error_envelope_with_kind(message, kind)` | Adds `code` field to the error object |

## Changelog

See [CLI_CONTRACT_CHANGELOG.md](../CLI_CONTRACT_CHANGELOG.md) for version history.
