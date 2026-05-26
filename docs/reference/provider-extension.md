# Provider Extension Reference

Earmark exposes provider extension through `ProviderService` and `ProviderRegistry`.

There are now two active extension paths:

1. in-process Rust registration for custom adapters
2. runtime discovery of provider-plugin manifests that alias bundled adapters

The manifest path is deliberately bounded. It does not load executable code. It lets a workspace or config surface install provider aliases without recompiling the CLI, as long as the target adapter is already compiled into the binary.

## Bundled Registration

Use `default_provider_registry()` when you want bundled adapters pre-registered.

```rust
use earmark_exec::default_provider_registry;

let registry = default_provider_registry();
```

Use `em provider capabilities` to see which providers are compiled and available in your current binary.

## Provider Plugin Manifests

The CLI bootstraps provider plugins from:

- `<root>/.earmark/plugins/providers`
- any extra directories listed in `provider_plugin_dirs` in CLI config
- any extra directories listed in `EM_PROVIDER_PLUGIN_DIRS`, separated by `:`

Each plugin manifest contributes one or more provider aliases that wrap an existing adapter. This is useful when a deployment wants to expose named provider surfaces such as `openai_compatible_http` or `anthropic_http` without editing bootstrap code.

Example:

```yaml
schema: earmark.provider_plugin.v1
name: openai-http
version: 0.1.0
description: OpenAI-compatible HTTP provider alias
providers:
  - provider: openai_compatible_http
    adapter: http_generation
    required_env:
      - OPENAI_API_KEY
```

With a compiled binary that includes `http_generation`, `em provider capabilities` will now show `openai_compatible_http` as a discovered provider alias. Missing required environment variables are surfaced as `missing_configuration`.

## Custom Registration

Build a registry explicitly, then add custom adapters.

```rust
use std::sync::Arc;
use earmark_exec::{ProviderAdapter, ProviderRegistry};

let mut registry = ProviderRegistry::new();
registry.register_default_adapters();
registry.register(Arc::new(MyAdapter));
```

You can then pass `&registry` anywhere a `&dyn ProviderService` is accepted, including `ExecutionEngine` and `RuntimeToolSurface`.

This is the supported path for adding custom providers without editing core CLI bootstrap code.

## Supported Extension Scope

Supported:
- register custom provider adapters in-process
- discover provider aliases from plugin manifests at runtime
- mix bundled and custom adapters
- substitute provider service test doubles behind `ProviderService`
- use the additive async seam through `AsyncProviderService` and `AsyncProviderAdapter` with current sync workflow entrypoints

Not in scope:
- dynamic executable plugin loading
- WASM provider execution
