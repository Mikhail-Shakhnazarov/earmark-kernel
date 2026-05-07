# Earmark Runtime Integration Guide

This guide explains how to integrate Earmark as a governed execution substrate into your application, either via the Rust SDK or the CLI bridge.

## 1. Architecture Overview

Earmark operates as a **governance kernel**. It doesn't "run" agents directly; instead, it provides bounded context through work packets, manages assignments over transitions, and validates the resulting change sets.

The runtime (your application) follows this loop:
1. Compile context from the kernel.
2. Receive a work packet for a transition.
3. Dispatch to a provider (e.g., Gemini).
4. Deposit the result back into the kernel.
5. Continue to the next transition using the handoff manifest.

## 2. Using the Rust SDK

Add the following to your `Cargo.toml`:
```toml
[dependencies]
earmark-core = { path = "..." }
earmark-exec = { path = "..." }
earmark-store = { path = "..." }
earmark-index = { path = "..." }
earmark-runtime-tools = { path = "..." }
```

### Initializing the Surface

```rust
use earmark_store::GitCanonicalStore;
use earmark_index::DerivedIndex;
use earmark_exec::{ProviderRegistry, GeminiAdapter};
use earmark_runtime_tools::RuntimeToolSurface;
use std::sync::Arc;

let store = GitCanonicalStore::new("./workspace");
let index = DerivedIndex::open("./workspace")?;
let mut registry = ProviderRegistry::default();

// Register the Gemini adapter
registry.register(Arc::new(GeminiAdapter::new(
    "gemini-2.5-flash".to_string(),
    "GOOGLE_API_KEY".to_string()
)));

let surface = RuntimeToolSurface {
    store: &store,
    index: &index,
    provider_registry: &registry,
};
```

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

## 3. Using the CLI Bridge (Stdio Workflow)

You can drive Earmark from any language by spawning the `em` binary as a subprocess and parsing its JSON output.

### Versioning
All JSON output is wrapped in a versioned envelope. Always check `contract_version`.

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
    "workflow", "run", "research_workflow", 
    "--system-id", "research_system", 
    "--with", note_id
])
print(f"Run completed: {run_resp['data']['run_id']}")
```

## 4. Authoring Provider Profiles

A provider profile connects a transition to a specific provider.

Example `docs/declarations/examples/provider_profiles/google_gemini.yaml`:
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

## 5. Error Handling

- **ExecError**: Check the workflow definition and inputs.
- **DispatchFailure**: Check your API keys, network, and provider budget.
- **RuntimeToolError**: Check for resource conflicts (e.g., duplicate assignments) or missing objects.
