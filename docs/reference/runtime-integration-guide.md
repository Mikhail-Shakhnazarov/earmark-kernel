# Runtime Integration Guide

This guide explains how to use Earmark as a governed execution substrate from your application, either through the Rust SDK or by driving the CLI as a subprocess. Workspace state is stored in a Git-backed canonical store implemented through `gix`, with a derived index for query and inspection.

## Architecture

Earmark doesn't run agents directly. It provides bounded context through work packets, manages assignments over transitions, validates the resulting change sets, and persists durable state in the workspace repository. Your application follows this loop:

1. Compile context from the kernel.
2. Receive a work packet for a transition.
3. Dispatch to a provider through the registry.
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
use earmark_exec::{ProviderRegistry, HttpGenerationAdapter};
use earmark_runtime_tools::RuntimeToolSurface;
use std::sync::Arc;

let store = GitCanonicalStore::new("./workspace");
let index = DerivedIndex::open("./workspace")?;
let mut registry = ProviderRegistry::new();

// Register the declarative HTTP provider adapter
registry.register(Arc::new(HttpGenerationAdapter));

let surface = RuntimeToolSurface {
    store: &store,
    index: &index,
    provider_service: &registry,
};
```

The provider registry is the current extension seam for custom provider integration. The bundled `HttpGenerationAdapter` handles most REST-based LLM providers (OpenAI, Anthropic, Gemini, etc.) through declarative profiles. For standalone or legacy integrations, you can still register custom `ProviderAdapter` implementations.

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

With `--json`, most command output is wrapped in a versioned envelope. Always check `contract_version`:

```json
{
  "contract_version": "0.2.0",
  "data": { ... }
}
```

Some output-special commands (`completions`, `run explain`) return shell code or formatted text even with `--json`; they are not designed for machine-driven parsing.

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

A provider profile connects a transition to a specific LLM provider.

### Declarative HTTP Provider Example

```yaml
name: gemini_3_1_flash
version: 0.1.0
provider: http_generation
model: gemini-3.1-flash-lite
auth_env: GEMINI_API_KEY
budget:
  max_output_tokens: 2048
response_contract:
  format: markdown
  must_return_candidate_only: true
http:
  method: POST
  url_template: "https://generativelanguage.googleapis.com/v1beta/models/{{model}}:generateContent"
  auth:
    kind: query_parameter
    param_name: key
  request:
    body:
      contents:
        - parts:
            - text: "{{input_text}}"
  response:
    text_path: "$.candidates[0].content.parts[0].text"
    input_tokens_path: "$.usageMetadata.promptTokenCount"
    output_tokens_path: "$.usageMetadata.candidatesTokenCount"
```

### OpenAI-Compatible Example

```yaml
name: gpt_4o
version: 0.1.0
provider: http_generation
model: gpt-4o
auth_env: OPENAI_API_KEY
http:
  method: POST
  url_template: "https://api.openai.com/v1/chat/completions"
  auth:
    kind: bearer
  request:
    body:
      model: "{{model}}"
      messages:
        - role: user
          content: "{{input_text}}"
  response:
    text_path: "$.choices[0].message.content"
    input_tokens_path: "$.usage.prompt_tokens"
    output_tokens_path: "$.usage.completion_tokens"
```

The mock provider (`mock`) is available for local-only development and testing without requiring API keys.
Outputs produced through the mock provider are marked as synthetic in provider metadata.

For direct provider extension patterns, see the [Provider Extension](provider-extension.md) reference.

## Error Handling

- **ExecError**: check the workflow definition and inputs.
- **DispatchFailure**: check API keys, network connectivity, and provider budget limits.
- **RuntimeToolError**: check for resource conflicts (e.g., duplicate assignments) or missing objects.

See the [Runtime Contract](runtime-contract.md) for the full error type reference.
