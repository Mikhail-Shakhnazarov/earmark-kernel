# Standing

Standing is declared domain state attached to objects. It tracks where an object is in its lifecycle — whether it's a draft or accepted, reviewed or unreviewed, active or archived — without hardcoding those categories into the kernel.

## How It Works

Standing is stored as a map from dimension IDs to token IDs:

```yaml
standing:
  kernel:epistemic: working
  kernel:review: unreviewed
  kernel:process: active
  research:status: draft
```

Dimensions and tokens are declared by the active system definition. A system might declare an `epistemic` dimension with tokens `working`, `accepted`, and `rejected`, and a `review` dimension with tokens `unreviewed`, `reviewed`, and `approved`.

## Protocol Bindings

Kernel behavior — review authorization, visibility, immutability — is not tied to specific token names. Instead, the kernel defines protocols (e.g., "review authorization"), and the system definition binds specific standing tokens to those protocols.

This means you can name your tokens whatever makes sense for your domain. The kernel enforces protocols, not token names.

## Format

Only the v0.3 map format is supported. Legacy v0.2 objects that used bare `epistemic`, `review`, or `process` fields without namespace prefixes are not supported. If you encounter v0.2-format objects, they need to be migrated to the namespaced map format shown above.

## Related

- [Staged Execution](staged-execution.md) — how standing interacts with workflow stages
- [Context Compilation](context-compilation.md) — how standing affects what a runtime sees
