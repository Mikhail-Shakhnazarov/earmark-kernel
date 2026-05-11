# Declaration Authoring

Earmark declarations describe the shape of a domain: its object types, relations, workflows, and execution rules.

Declarations are the authoring surface for:

- object classes
- relation rules
- standing policies
- compiled contexts
- instructions
- workflow definitions
- provider profiles
- system definitions

The current authoring format is YAML for structured declarations and Markdown with YAML frontmatter for instructions.

## Validate a Declaration

Use `em declare validate` before registering or depositing declaration objects.

```bash
em declare validate --kind class docs/declarations/examples/classes/finding.yaml
em declare validate --kind instruction docs/declarations/examples/instructions/source_to_finding.md
em declare validate --kind workflow docs/declarations/examples/workflows/source_to_finding.yaml
em declare validate --kind system examples/research-synthesis/declarations/systems/system.yaml
```

## Explain a Declaration

Use `em declare explain` to see a plain-language summary of what a declaration does.

```bash
em declare explain --kind compiled-context docs/declarations/examples/compiled_contexts/source_notes_for_extraction.yaml
em declare explain --kind workflow docs/declarations/examples/workflows/source_to_finding.yaml
em declare explain --kind system examples/research-synthesis/declarations/systems/system.yaml
```

## Scaffold a Declaration

Use `em declare new` to scaffold from in-repo templates.

```bash
em declare new --kind class finding
em declare new --kind instruction source_to_finding
em declare new --kind compiled-context findings_for_summary
em declare new --kind provider-profile local_mock
em declare new --kind workflow research_synthesis
em declare new --kind system research_synthesis
```

## Declaration Kinds

Supported kinds:

- `class`
- `instruction`
- `standing-policy`
- `workflow`
- `compiled-context`
- `provider-profile`
- `system`

## System Manifests

A system manifest references its declaration files by relative path. When you validate or register a system, the CLI resolves all dependencies and checks for missing references.

## Examples

Example declarations live in:

```text
docs/declarations/examples/
```

The examples are intentionally small. They teach declaration shape directly. For a complete reference domain definition, use the research synthesis demo under `examples/research-synthesis/`.

Key examples:
- `class`: [Finding](examples/classes/finding.yaml)
- `instruction`: [Source to Finding](examples/instructions/source_to_finding.md)
- `workflow`: [Source to Finding](examples/workflows/source_to_finding.yaml)
- `provider-profile`: [Google Gemini](examples/provider_profiles/google_gemini.yaml), [Local Mock](examples/provider_profiles/local_mock.yaml)

## Schemas

JSON Schema files for the seven declaration kinds are published in:

```text
docs/declarations/schema/
```

CLI and Rust validation is authoritative. Schema files may lag behind the Rust validator for some edge cases. When the CLI rejects a declaration, rely on the CLI error message rather than the schema file alone.

### Validation Coverage

Validation is active across all seven declaration kinds:

- **Class**: validates kind, name, relations (source/target class references, direction, authorizing endpoint), and payload constraints.
- **Instruction**: validates metadata fields (title, class tokens, standing constraints) and YAML frontmatter structure.
- **Standing Policy**: validates transition references, allowed epistemic/review/process standings, and escalation targets.
- **Workflow**: validates operation kinds, edges, entry/terminal transitions, guards (standing checks), and compiled context references.
- **Compiled Context**: validates class/standing/relation/render constraints and template structure.
- **Provider Profile**: validates required fields (`provider`, `model`, `auth_env`), response contract format, and budget constraints.
- **System Manifest**: validates declaration reference resolution, required declaration coverage, and manifest completeness.
