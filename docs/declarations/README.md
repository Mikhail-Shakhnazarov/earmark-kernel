# Declaration Authoring

Earmark declarations describe the shape of a governed AI workflow.

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

## Validate A Declaration

Use `em declare validate` before registering or depositing declaration objects.

```bash
em declare validate --kind class docs/declarations/examples/classes/finding.yaml
em declare validate --kind instruction docs/declarations/examples/instructions/source_to_finding.md
em declare validate --kind workflow docs/declarations/examples/workflows/source_to_finding.yaml
em declare validate --kind system examples/research-synthesis/declarations/systems/system.yaml
```

## Explain A Declaration

Use `em declare explain` to see a concise machine-readable summary.

```bash
em declare explain --kind compiled-context docs/declarations/examples/compiled_contexts/source_notes_for_extraction.yaml
em declare explain --kind workflow docs/declarations/examples/workflows/source_to_finding.yaml
em declare explain --kind system examples/research-synthesis/declarations/systems/system.yaml
```

## Scaffold A Declaration

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

## Path-Based System Manifests

`--kind system` supports path-based manifests as the primary authoring shape. The system manifest references declaration files by relative path, and CLI register/validate/explain resolves and checks all dependencies.

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

JSON Schema files for active declaration kinds are published in:

```text
docs/declarations/schema/
```
