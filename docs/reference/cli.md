# CLI Reference

The Earmark CLI (`em`) is the primary interface for operators and developers. Most commands support `--json` for machine-readable output wrapped in a versioned envelope. For technical details on the JSON structure, see the [CLI Contracts Reference](cli-contracts.md). Output-special commands such as `completions` bypass the JSON envelope.

## Global Flags

| Flag | Description |
|---|---|
| `--root <path>` | Workspace root directory (default: current directory) |
| `--json` | Output results as JSON wrapped in a versioned envelope |
| `--config <path>` | Path to an explicit configuration file |
| `--log-level <level>` | Set log level (error, warn, info, debug, trace) |
| `--verbose` | Increase verbosity (repeatable: -v, -vv) |
| `--help` | Help for any command or subcommand |

## Workspace

### `em init`

Initialize a new Earmark workspace. This creates the workspace layout (`.earmark/`, `corpus/`, `.git/`, and index storage).

### `em doctor`

Check workspace health without repairing it. On an uninitialized root, `doctor` reports missing layout and suggests `em init`.

### `em status`

Show counts of objects, assignments, change sets, and active systems.

### `em completions <shell>`

Generate shell completion scripts (bash, zsh, fish). Emits shell code to stdout.

## Declarations

### `em declare validate --kind <kind> <path>`

Validate a declaration file against its schema.

Kinds: `class`, `instruction`, `standing-policy`, `workflow`, `compiled-context`, `provider-profile`, `system`.

### `em declare explain --kind <kind> <path>`

Explain what a declaration does in plain language.

### `em declare new --kind <kind> <name>`

Generate a new declaration from a built-in template.

### `em declare register --kind <kind> <path>`

Register a declaration in the workspace index.

### `em declare list-examples`
 
Lists declaration examples found in the current workspace under `docs/declarations/examples`.

A fresh workspace does not include declaration examples by default. Add examples under `docs/declarations/examples` to make them appear in this command.

## Systems

### `em system register <manifest_path>`

Register a path-based system manifest and resolve its referenced declarations.

### `em system activate <system_id>`

Set the active system for the workspace.

## Data

### `em deposit --class <class> [--kind <kind>] [--title <title>] [--body <body>] [--payload-file <path>] [--json-payload <json>]`

Deposit an object into the corpus. Default kind is `object`.

**Governed Behavior**: If a system context is active (via `EM_SYSTEM_ID` or `default_system_id`), the deposit is strictly validated against the system's admitted class list and associated schemas.

**Scratch Behavior**: If no system context is resolved, the deposit is "scratch-permissive" and uses the latest available definition for the requested class.

### `em query [--class <class>] [--kind <kind>] [--text <query>] [--object-id <id>]`

Search the corpus through the derived index.

### `em review <object_id> [--version-id <id>] [--reason <text>] [--reject]`

Submit a review for an object. Accepts by default; use `--reject` to deny.

### `em context compile --root <object_id> [--depth <n>] [--relation-type <type>] [--class <class>] [--epistemic <standing>]`

Compile a work surface from one or more root objects. Flags like `--root`, `--relation-type`, and `--class` are repeatable.

## Workflow Execution

### `em workflow run <workflow_id> [--version-id <id>] [--system-id <id>] [--with <object_id>] [--handoff <handoff_id>] [--assignment <id>] [--approve-review]`

Execute a declared workflow.

- `--with <id>` — start from specific input objects (repeatable)
- `--handoff <id>` — continue from a previous stage's handoff
- `--approve-review` — automatically approve the result if required by policy

## Inspection

### `em run list`

List recent workflow runs.

### `em run show <run_id>`

Show the raw run record for a specific run. Returns the stored ledger data directly (run_id, status, events, timestamps). Accepts `latest` for the most recent run.

### `em run explain <run_id>`

Interpreted run context: status, transitions, related artifacts (assignments, change sets, handoffs, failures), and suggested next commands. Use `latest` for the most recent run.

### `em run timeline <run_id>`

Visual timeline of events in a run.

### `em run artifacts <run_id>`

List all durable artifacts created during a run.

### `em run graph <run_id>`

Mermaid relationship graph of artifacts produced during a run.

### `em assignment show <id>`

Show raw data for a transition assignment.

### `em assignment explain <id>`

Explain a transition assignment's status and inputs.

### `em assignment list [--run-id <id>] [--status <status>]`

List assignments, optionally filtered by run or status.

### `em change-set show <id>`

Show raw data for a change set.

### `em change-set explain <id>`

Explain what a transition produced.

### `em change-set list [--run-id <id>]`

List change sets, optionally filtered by run.

### `em handoff show <id>`

Show raw data for a handoff.

### `em handoff explain <id>`

Explain a handoff's constraints and carried objects.

### `em handoff list [--run-id <id>]`

List handoffs, optionally filtered by run.

### `em failure show <id>`

Show the complete failure artifact as stored. Includes run id, transition id, assignment id, error type, error message, failed change set id (if any), input object ids at time of failure, and timestamp.

### `em failure explain <id>`

Explain what went wrong in a transition. Returns interpreted context: run id, assignment id, transition id, failed change set id, error type, and suggested next commands (`em failure show`, `em run explain`, `em change-set explain`, `em assignment explain`, `em run timeline`).

### `em failure list [--run-id <id>] [--transition-id <id>]`

List failures with summary fields (failure id, run id, transition id, assignment id, error type, message, timestamp). Filter by run id, transition id, or both.

### `em relation show <id>`

Show raw data for a specific relation.

### `em relation explain <id>`

Explain a relation: type, endpoints, and authorization trace.

### `em relation list [--source-id <id>] [--target-id <id>] [--relation-type <type>]`

List relations, optionally filtered by source, target, or type.

## Standing Requests

### `em standing-request list [--status <status>] [--target <object_id>]`

List proposed/applied/rejected standing requests.

### `em standing-request show <request_id>`

Show one standing request.

### `em standing-request approve <request_id> [--reason <text>]`

Approve a proposed standing request.

### `em standing-request reject <request_id> [--reason <text>]`

Reject a proposed standing request.

### `em standing-request apply <request_id> [--policy <policy>] [--reason <text>]`

Apply an approved standing request, optionally through a named policy.

## Audit and Providers

### `em audit failures [--run-id <id>] [--transition-id <id>]`

Audit workflow failures. Returns failure count summary and suggested next commands in addition to failure details.

### `em audit show <failure_id>`

Show detailed failure analysis.

### `em provider capabilities`

List capabilities of compiled-in providers.

## Reports

### `em report run <id> --output <path>`

Generate a static HTML report for a specific run.

### `em report handoff <id> --output <path>`

Generate a static HTML report for a specific handoff.

### `em report system <id> --output <path>`

Generate a static HTML report for a specific system.

## License

AGPL-3.0-or-later OR Commercial.
