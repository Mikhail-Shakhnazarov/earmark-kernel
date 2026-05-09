# Runtime Integration Guide

This guide explains how to use Earmark as a governed execution substrate from your application, either through the Rust SDK or by driving the CLI as a subprocess. Workspace state is stored in a Git-backed canonical store implemented through `gix`, with a derived index for query and inspection.

## Architecture

Earmark doesn't run agents directly. It provides bounded context through work packets, manages assignments over transitions, validates the resulting change sets, and persists durable state in the workspace repository. Your application follows this loop:

1. Compile context from the kernel.
2. Receive a work packet for a transition.
3. Dispatch to a provider (e.g., Gemini).
4. Deposit the result back into the kernel.
5. Continue to the next transition using the handoff manifest.

## Using the Rust SDK

Add the workspace crates to your `Cargo.toml`:

```toml
[dependencies]
earmark-core = { path = "..." }
earmark-exec = { path = "..." }
earmark-store = { path = "..." }
earmark-index = { path = "..." }
earmark-runtime-tools = { path = "..." }
```

### Initializing the Surface

`GitCanonicalStore` manages the canonical workspace repository. It writes objects, relations, assignments, change sets, and handoffs into the repository-backed store and records commit-backed history through `gix`.

```rust
use earmark_store::GitCanonicalStore;
use earmark_index::DerivedIndex;
use earmark_exec::{ProviderRegistry, GeminiAdapter};
use earmark_runtime_tools::RuntimeToolSurface;
use std::sync::Arc;

let store = GitCanonicalStore::new("./workspace");
let index = DerivedIndex::open("./workspace")?;
let mut registry = ProviderRegistry::new();

// Register the Gemini adapter (requires GOOGLE_API_KEY env var)
registry.register(Arc::new(GeminiAdapter::new(
    "gemini-2.5-flash".to_string(),
    std::env::var("GOOGLE_API_KEY").expect("GOOGLE_API_KEY must be set"),
)));

let surface = RuntimeToolSurface {
    store: &store,
    index: &index,
    provider_registry: &registry,
};
```

The provider registry is the current extension seam for custom provider integration. You can register additional adapters in-process and hand the registry to the runtime surface without modifying the core execution engine. If you want the bundled adapters pre-registered, use `default_provider_registry()` or `ProviderRegistry::with_defaults()`.

### Running a Workflow

```rust
use earmark_exec::WorkflowRunRequest;

let outcome = surface.run_workflow(WorkflowRunRequest {
    run_id: "run_123".to_string(),
    system_definition: system_ref,
    workflow: workflow_ref,
    inputs: vec![source_note_ref],
    handoff_manifest: None,
    transition_assignment: None,
    operator_approved: true,
})?;

for object in outcome.emitted_objects {
    println!("Created: {:?}", object);
}
```

## Using the CLI Bridge

You can drive Earmark from any language by spawning the `em` binary and parsing its JSON output.

All JSON output is wrapped in a versioned envelope. Always check `contract_version`:

```json
{
  "contract_version": "0.2.0",
  "data": { ... }
}
```

### Python Example

```python
import subprocess
import json

def em_command(args):
    result = subprocess.run(
        ["em", "--json"] + args,
        capture_output=True,
        text=True
    )
    if result.returncode != 0:
        raise Exception(f"Earmark error: {result.stderr}")
    return json.loads(result.stdout)

# Deposit a source note
resp = em_command(["deposit", "--class", "source_note", "--title", "Hello", "--body", "World"])
note_id = resp["data"]["object_id"]

# Run a workflow
run_resp = em_command([
    "workflow", "run", "research_synthesis",
    "--system-id", "sys_research_synthesis",
    "--with", note_id
])
print(f"Run completed: {run_resp['data']['run_id']}")
```

## Provider Profiles

A provider profile connects a transition to a specific LLM provider. Example for Google Gemini:

```yaml
name: gemini_research
version: 0.2.0
provider: google_gemini
model: gemini-2.5-flash
auth_env: GOOGLE_API_KEY
budget:
  max_output_tokens: 4000
  max_latency_ms: 30000
response_contract:
  format: json
  must_return_candidate_only: true
```

The mock provider (`local_mock`) is the default and requires no API keys. Use it for development and testing.

For direct provider extension patterns, see the [Provider Extension](provider-extension.md) reference.

## Error Handling

- **ExecError**: check the workflow definition and inputs.
- **DispatchFailure**: check API keys, network connectivity, and provider budget limits.
- **RuntimeToolError**: check for resource conflicts (e.g., duplicate assignments) or missing objects.

See the [Runtime Contract](runtime-contract.md) for the full error type reference.
