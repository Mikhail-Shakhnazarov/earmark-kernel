# CLI Reference

The Earmark CLI (`em`) is the primary interface for operators and developers. All commands support `--json` for machine-readable output.

## Global Flags

| Flag | Description |
|---|---|
| `--root <path>` | Workspace root directory (default: current directory) |
| `--json` | Output results as JSON wrapped in a versioned envelope |
| `--help` | Help for any command or subcommand |

## Workspace

### `em init`

Initialize a new Earmark workspace. Creates the `.earmark/` directory and required storage structure.

### `em doctor`

Check workspace health: store integrity, index state, active system status.

### `em status`

Show counts of objects, assignments, change sets, and active systems.

```bash
em status
# Objects: 12  Assignments: 4  Change Sets: 3  Active System: sys_research_synthesis
```

## Declarations

### `em declare validate --kind <kind> <path>`

Validate a declaration file against its schema. Returns specific errors if validation fails.

Kinds: `class`, `instruction`, `workflow`, `compiled-context`, `provider-profile`, `system`.

```bash
em declare validate --kind system declarations/system.yaml
# OK: System 'sys_research_synthesis' is valid.
```

### `em declare explain --kind <kind> <path>`

Explain what a declaration does in plain language.

### `em declare new <kind> <name>`

Generate a new declaration from a built-in template.

```bash
em declare new class my_finding > declarations/classes/my_finding.yaml
```

### `em declare list-examples`

List built-in declaration examples available for reference.

## Systems

### `em system register <manifest_path>`

Register a path-based system manifest and resolve its referenced declarations.

### `em system activate <system_id>`

Set the active system for the workspace. Subsequent commands use this system by default.

### `em system list`

List all registered systems and show which is active.

## Data

### `em deposit --class <class> [--title <title>] [--body <body>] [--payload-file <path>]`

Deposit an object into the corpus.

```bash
em deposit --class source_note --title "Field Notes" --body "Municipal capacity is limited."
# Created: obj_a1b2c3d4...  Class: source_note
```

### `em query [--class <class>] [--title <query>]`

Search the corpus through the derived index.

```bash
em query --class source_note
# obj_a1b2c3d4  source_note  "Field Notes"
# obj_e5f6g7h8  source_note  "Report Excerpt"
```

### `em context compile --root <object_id> [--depth <n>]`

Compile and preview a work surface from a root object.

## Workflow Execution

### `em workflow run <workflow_id> --system-id <system_id> [--with <object_id>] [--handoff <handoff_id>]`

Execute a declared workflow.

- `--with <id>` — start from specific input objects (repeatable)
- `--handoff <id>` — continue from a previous stage's handoff manifest

```bash
em workflow run research_synthesis --system-id sys_research_synthesis --with obj_a1b2c3d4
```

## Inspection

### `em run list`

List recent workflow runs.

### `em run explain <run_id>`

Summary of a run: status, transitions, created artifacts, and suggested next commands. Use `latest` as shorthand for the most recent run.

### `em run timeline <run_id>`

Visual timeline of events in a run.

### `em run graph <run_id>`

Mermaid relationship graph of artifacts produced during a run.

### `em run artifacts <run_id>`

List all durable artifacts (assignments, change sets, handoffs, failures) created during a run.

### `em assignment explain <assignment_id>`

Explain a transition assignment: what work was claimed, over which inputs, current status.

### `em change-set explain <change_set_id>`

Explain what a transition produced: created objects, validation result, linked handoff.

### `em handoff explain <handoff_id>`

Explain a handoff: which objects it carries, which classes and relations are allowed for successor work.

### `em failure explain <failure_id>`

Explain a failure: what went wrong, which assignment and change set are linked, suggested recovery.

### `em failure list`

List transformation failures, optionally filtered by run.

## Reports

### `em report <run|handoff|system> <id> --output <path>`

Generate a static HTML report for a specific artifact. Reports include timelines, artifact summaries, Mermaid diagrams, and are self-contained for sharing.

```bash
em report run latest --output reports/synthesis_run.html
```

## License

AGPL-3.0-or-later OR Commercial.
