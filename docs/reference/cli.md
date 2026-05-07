# Reference: CLI

The Earmark CLI (`em`) is the primary interface for operators and developers.

## Global Flags

- `--root <path>`: Specify the workspace root (default: current directory).
- `--json`: Output results in machine-readable JSON format.
- `--help`: View help for any command or subcommand.

## Workspace Management

### `em init`
Initialize a new Earmark workspace. Creates the `.earmark/` directory and required substrate.

### `em doctor`
Check the health of the workspace and canonical store.

### `em status`
Show counts of objects, assignments, change sets, and active systems.

## Declaration Management

### `em declare validate --kind <kind> <path>`
Validate a declaration file against its schema.
Kinds: `class`, `instruction`, `workflow`, `system`, etc.

### `em declare explain --kind <kind> <path>`
Explain what a declaration does in plain language.

### `em declare new <kind> <name>`
Scaffold a new declaration from a template.

### `em declare list-examples`
List built-in declaration examples.

## System Management

### `em system register <manifest_path>`
Register a path-based system manifest and its dependencies.

### `em system activate <system_id>`
Set the active system for the current workspace.

### `em system list`
List registered systems.

## Data Operations

### `em deposit --class <class> [--title <title>] [--body <body>]`
Deposit an object into the corpus.

### `em query [--class <class>] [--title <query>]`
Search the corpus through the derived index.

### `em context compile --root <object_id> [--depth <n>]`
Manually compile and preview a work surface.

## Workflow Execution

### `em workflow run <workflow_id> --system-id <system_id> [--with <object_id>] [--handoff <handoff_id>]`
Execute a declared workflow. 
- `--with`: Start from a specific input object.
- `--handoff`: Continue from a previous Stage's handoff manifest.

## Inspection and Observability

### `em run list`
List recent workflow runs.

### `em run explain <run_id>`
Show a summary of a run and suggested next steps.

### `em run timeline <run_id>`
View a visual timeline of events in a run.

### `em run graph <run_id>`
Generate a Mermaid relationship graph for the artifacts in a run.

### `em run artifacts <run_id>`
List all durable artifacts (assignments, change sets, etc.) created during a run.

### `em assignment explain <assignment_id>`
Explain a specific transition assignment.

### `em change-set explain <change_set_id>`
Explain the changes produced by a transition.

### `em handoff explain <handoff_id>`
Explain the bounded context carried by a handoff.

### `em failure explain <failure_id>`
Explain a transition failure and suggest recovery steps.

### `em report <run|handoff|system> <id> --output <path>`
Generate a static HTML report for a specific artifact.

### `em audit failures`
List and audit transformation failures across the workspace.

## License
AGPL-3.0-or-later OR Commercial.
