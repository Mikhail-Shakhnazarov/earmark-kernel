# Provider Plugin Work Trace

Date: 2026-05-26

## Why this work happened

The repo already had a provider registry seam, but the extensibility story was still mostly “compile adapters into Rust and register them in process.”

That was better than hardcoded bootstrap, but still too narrow for the desired application story:

- self-hosted substrate
- one chat on the surface
- runtime-configurable provider surfaces underneath

The immediate goal was not full executable plugins. The goal was a bounded first plugin build that could land quickly and leave a durable development trace.

## What was implemented

Implemented a manifest-based provider plugin slice.

This adds runtime discovery of provider aliases from YAML manifests.

A plugin manifest can now contribute provider names that wrap adapters already compiled into the binary.

Current behavior:

- default discovery path: `<root>/.earmark/plugins/providers`
- extra discovery paths:
  - `provider_plugin_dirs` in CLI config
  - `EM_PROVIDER_PLUGIN_DIRS` as a colon-separated env var
- plugin aliases appear in `em provider capabilities`
- status output also includes loaded provider plugins
- missing required env vars are surfaced as `missing_configuration`

## What this does not do

This is not executable plugin loading.

Not implemented in this slice:

- `.so` / `.dll` dynamic loading
- WASM provider execution
- custom transition or operation plugins
- arbitrary third-party code execution at runtime

This slice is intentionally bounded: runtime discovery without runtime code loading.

## Main code changes

- `earmark-exec/src/provider.rs`
  - provider keys no longer require `'static`
- `earmark-exec/src/provider_plugins.rs`
  - plugin manifest loader
  - alias adapter wrapper
  - manifest validation
- `earmark-exec/src/lib.rs`
  - exported plugin loader surface
- `earmark-cli/src/config.rs`
  - provider plugin directory config/env resolution
- `earmark-cli/src/app/bootstrap.rs`
  - plugin discovery during bootstrap
- `earmark-cli/src/app/common.rs`
  - loaded plugin metadata carried through command context
- `earmark-cli/src/app/dispatch/handlers.rs`
  - provider/status output now reflects bootstrapped registry state

## Verification

Verified in Nix dev shell:

- `nix develop --command cargo test -p earmark-exec provider_plugins`
- `nix develop --command cargo test -p earmark-cli --test output_contracts`

The CLI test now proves:

- a provider plugin manifest dropped into `.earmark/plugins/providers`
- is discovered at runtime
- surfaces a provider alias in `em provider capabilities`
- and reports the loaded plugin metadata

## Architectural reading

This is a meaningful step toward plugin architecture, but not the whole thing.

The present extension model is now:

1. bundled adapters compiled into the binary
2. runtime-discovered provider aliases installed by manifest
3. provider profiles and workflows choose among those visible provider surfaces

That is enough to support a stronger “app substrate” story without pretending full general plugin execution is already solved.

## Next tasks

### Task 1: richer provider plugin manifests

Allow plugin manifests to contribute more than alias names, for example:

- default HTTP request/response templates
- default auth/env wiring
- allowed-domain presets
- plugin-level policy metadata

### Task 2: provider plugin validation command

Add a dedicated CLI surface such as:

- `em provider plugins`
- `em provider validate-plugin <path>`

This would improve observability and debugging.

### Task 3: workspace-facing example plugin pack

Add documented example manifests for:

- OpenAI-compatible HTTP
- Anthropic-compatible HTTP
- local gateway / reverse proxy setups

### Task 4: executable plugin evaluation

Only after the manifest-based seam proves useful:

- evaluate WASM plugins for bounded executable extension
- or evaluate dynamic library loading if the operational tradeoffs are acceptable

## Present conclusion

The repo now has a real plugin-adjacent runtime discovery path.

It is not yet “community executable plugins.”

It is, however, enough to say that provider surfaces can now be installed and discovered through self-hosted plugin manifests rather than only by editing bootstrap code and recompiling.
