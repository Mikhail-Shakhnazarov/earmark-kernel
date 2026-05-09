# Provider Extension Reference

Earmark exposes provider extension through `ProviderService` and `ProviderRegistry`. The current extension surface is code-level and in-process: consumers register provider adapters directly in Rust and pass the resulting registry into execution surfaces.

This is the active extensibility model in the repository today. Dynamic plugin loading and runtime discovery are future extension directions rather than present features.

## Bundled Registration

Use `default_provider_registry()` when you want bundled adapters pre-registered.

```rust
use earmark_exec::default_provider_registry;

let registry = default_provider_registry();
```

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
- mix bundled and custom adapters
- substitute provider service test doubles behind `ProviderService`
- use the additive async seam through `AsyncProviderService` and `AsyncProviderAdapter` with current sync workflow entrypoints

Not in scope:
- dynamic plugin loading
- runtime discovery from plugin directories
- WASM provider execution
