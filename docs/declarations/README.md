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

CLI and Rust validation are authoritative. JSON Schema files are useful authoring aids but may not capture every semantic check enforced by the validator.

### Validation Coverage

CLI and Rust validation is active across all seven declaration kinds.

| Kind | Current validation coverage |
|---|---|
| `class` | Class name, non-empty version, `kind` value, standing-rule tokens, relation type tokens, counterparty class tokens, relation direction, authorizing endpoint, and dead direction/authority combinations |
| `instruction` | Instruction name, non-empty version, non-empty purpose, non-empty body, input class tokens, and output class tokens |
| `standing-policy` | Policy name, non-empty version, transition-rule dimensions and standing tokens, operation requirement dimensions/tokens, and non-empty escalation trigger/message |
| `workflow` | Workflow name, operation id tokens and uniqueness, operation kind, required `compiled_context` for `compile_context` operations, required instruction for `transform` operations, one output contract for transform operations, input/output class tokens, guard id tokens and uniqueness, edge endpoints, and guard references |
| `compiled-context` | Template name, non-empty version, selected class tokens, non-empty render mode, standing dimensions/tokens, and relation type tokens |
| `provider-profile` | Profile name, non-empty version, provider/model presence, response format, non-negative budget, auth/endpoint environment variable names, and HTTP provider template/auth/body constraints when `provider: http_generation` is used |
| `system` | System id, namespace, referenced object existence, referenced kind/class where required, referenced payload decodability, title, and non-empty runtime profile fields |
