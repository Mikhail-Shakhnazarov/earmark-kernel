# Earmark CLI Contract Changelog

This document tracks changes to the machine-readable JSON output of the Earmark CLI.
Downstream orchestrators and tools should monitor this file for breaking changes and schema updates.

Current Contract Version: 0.3.0
Reference Documentation: [docs/cli-contract.md](docs/cli-contract.md)

## [0.3.0] - 2026-05-17

### Summary
Added structured error codes, thread-local context tracking, and enriched error envelopes.

### Added
- `error.code` field in error envelopes, populated by `CliError::kind_str()`.
- `emit_error_envelope_with_kind()` for emitting errors with a machine-readable code.
- Thread-local `CliContext` tracking (`command_name`, `as_json`) accessible via `output::with_context()`.
- `CliError::kind_str()` — returns a static string tag for each error variant.
- `docs/cli-contract.md` — canonical contract reference.

### Changed
- Error path in `main.rs` now uses `emit_error_envelope_with_kind` to include error codes.
- `app::run()` initializes `CliContext` before dispatching commands.

### Deprecated
- `docs/reference/cli-contracts.md` — superseded by `docs/cli-contract.md`.

## [0.2.0] - 2026-05-16

### Summary
Established the "Hardened Contract Baseline" with a unified envelope and error stream policy.

### Added
- Standardized top-level envelope: `{"contract_version": "0.2.0", "ok": bool, "data": {}}`.
- Error reporting via JSON: `{"ok": false, "error": {"message": "..."}}`.
- Early JSON resolution: Config loading errors now respect `--json` or `EM_JSON` environment variable.

### Changed
- **UNIFIED STREAM POLICY**: All JSON envelopes (including errors) are now emitted to `stdout`. `stderr` is strictly reserved for diagnostic logs (tracing) and system panics.
- Refined error messages to be more machine-parseable in common failure modes.

### Fixed
- Fixed inconsistent error routing where some errors bypassed the JSON envelope.
- Aligned integration tests (`cli.rs`, `standing_request.rs`) with the unified stream policy.
