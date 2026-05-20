# Runtime Integration Guide

© 2026 Mikhail Shakhnazarov

This guide explains how to integrate with Earmark as a governed execution substrate.

There are currently two integration modes:

1. **In-process API** through `earmark::EarmarkWorkspace` (or direct crate composition). This is the canonical Rust integration path.
2. **CLI-backed compatibility facade** through `earmark::CliBackedWorkspace` or by spawning `earmark-cli` directly.

Workspace state is stored in a Git-backed canonical store implemented through `gix`, with a derived index for query and inspection.

## Architecture

Earmark does not run agents directly. It provides bounded context through work packets, manages assignments over transitions, validates resulting change sets, and persists durable state in the workspace repository. A host application follows this loop:

1. Compile context from declared objects and context templates.
2. Receive a work packet for a transition.
3. Dispatch to a provider through a provider service or registry.
4. Persist the result back into the canonical store.
5. Continue to the next transition using the handoff manifest.

## In-process crate composition

Add the workspace crates to `Cargo.toml`:

```toml
[dependencies]
earmark-core = { path = "..." }
earmark-exec = { path = "..." }
earmark-store = { path = "..." }
earmark-index = { path = "..." }
earmark-runtime-tools = { path = "..." }
```

### Initializing the surface

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

registry.register(Arc::new(HttpGenerationAdapter));

let surface = RuntimeToolSurface {
    store: &store,
    index: &index,
    provider_service: &registry,
};
```

The provider registry is the current extension seam for custom provider integration. The bundled `HttpGenerationAdapter` handles REST-based LLM providers through declarative profiles. For standalone or legacy integrations, custom `ProviderAdapter` implementations can still be registered.

### Running a workflow

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

## CLI-backed compatibility facade

`CliBackedWorkspace` shells out to `earmark-cli` and parses JSON output. It is retained as a compatibility surface.

Use it when a simple Rust wrapper around the CLI is sufficient. Prefer `EarmarkWorkspace` for in-process Rust integration.

Runtime requirements:

- set `EARMARK_CLI_BIN` to the intended `earmark-cli` executable; or
- ensure `earmark-cli` is available on `PATH`.

`EarmarkWorkspace` already composes canonical store, derived index, declaration registration, deposit, and workflow execution without spawning a subprocess.

## Direct CLI bridge

Any language can drive Earmark by spawning the `em` / `earmark-cli` binary and parsing JSON output.

With `--json`, most command output is wrapped in a versioned envelope. Always check `contract_version`:

```json
{
  "contract_version": "0.2.0",
  "ok": true,
  "data": { }
}
```

Error envelopes are also emitted to stdout in JSON mode:

```json
{
  "contract_version": "0.2.0",
  "ok": false,
  "error": {
    "message": "human-readable description",
    "code": "error_kind"
  }
}
```

Some output-special commands, such as shell completions or formatted explanations, may return shell code or formatted text rather than machine-oriented JSON.

### Python example

```python
import subprocess
import json

def em_command(args):
    result = subprocess.run(
        ["em", "--json"] + args,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        try:
            payload = json.loads(result.stdout)
            message = payload.get("error", {}).get("message", "unknown Earmark error")
            code = payload.get("error", {}).get("code")
            raise Exception(f"Earmark error {code}: {message}")
        except json.JSONDecodeError:
            raise Exception(f"Earmark error: {result.stderr}")
    return json.loads(result.stdout)

resp = em_command([
    "deposit",
    "--class", "source_note",
    "--title", "Hello",
    "--body", "World",
])
note_id = resp["data"]["object_id"]

run_resp = em_command([
    "workflow", "run", "research_synthesis",
    "--system-id", "sys_research_synthesis",
    "--with", note_id,
])
print(f"Run completed: {run_resp['data']['run_id']}")
```

## Provider profiles

A provider profile connects a transition to a specific LLM provider.

### Declarative HTTP provider example

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

### OpenAI-compatible example

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

The mock provider (`mock`) is available for local-only development and testing without requiring API keys. Outputs produced through the mock provider are marked as synthetic in provider metadata.

For direct provider extension patterns, see the [Provider Extension](provider-extension.md) reference.

## Standing model

Standing is stored as a map from standing dimension IDs to token IDs:

```json
"standing": {
  "kernel:epistemic": "working",
  "kernel:review": "unreviewed",
  "kernel:process": "active",
  "research:status": "draft"
}
```

Built-in kernel dimensions use the `kernel:` prefix. Custom dimensions are declared in the system definition. Kernel behavior, including review authorization, visibility, and immutability, is projected from declared standing tokens through protocol bindings rather than by matching token names.

Provider exposure requires two gates:

1. The standing projection must return `expose_to_provider: true`.
2. The provider profile must permit the object/content type.

`include_in_standard_context = true` does not imply provider exposure. Visibility defaults are:

```json
"include_in_standard_context": true,
"expose_to_provider": false
```

Only the v0.3 map format is supported. Objects using the legacy v0.2 `epistemic` / `review` / `process` fields will not deserialize correctly.

## Error handling

- **ExecError**: check the workflow definition and inputs.
- **ProviderFailure**: check API keys, network connectivity, provider policy, and provider budget limits.
- **RuntimeToolError**: check for resource conflicts, duplicate assignments, or missing objects.
- **CLI JSON errors**: parse stdout before falling back to stderr.

See the [Runtime Contract](runtime-contract.md) for the full error type reference.
