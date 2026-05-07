# Reference: Declaration Schemas

Earmark declarations use YAML and Markdown formats, which are validated against JSON Schema definitions.

## Schema Location

Canonical JSON Schema files are located in the repository at:

```text
docs/declarations/schema/
```

## Declaration Kinds

### 1. Class Declaration
Defines an object type in the corpus.
- **Fields**: `name`, `kind`, `payload_schema`, `required_headers`, `standing_rules`, `relation_rules`.
- **See Example**: [Finding](../declarations/examples/classes/finding.yaml)

### 2. Instruction Declaration
Defines a specific processing operation.
- **Format**: Markdown with YAML frontmatter.
- **Fields**: `name`, `purpose`, `input_classes`, `output_classes`, `provider_profile`.
- **See Example**: [Source to Finding](../declarations/examples/instructions/source_to_finding.md)

### 3. Workflow Declaration
Defines a sequence of transitions.
- **Fields**: `name`, `operations`, `edges`, `guards`.
- **See Example**: [Source to Finding](../declarations/examples/workflows/source_to_finding.yaml)

### Compiled Context Template
Defines rules for building an admissible work surface.
- **Fields**: `name`, `select` (classes/standing/relations), `render` (mode/format), `visibility`.
- **See Example**: [Source Notes](../declarations/examples/compiled_contexts/source_notes_for_extraction.yaml)

### 5. System Manifest
Binds a collection of declarations into a deployable system.
- **Fields**: `system_id`, `namespace`, `title`, `classes`, `instructions`, `workflows`, `compiled_contexts`, `runtime_profile`.
- **See Example**: [Research Synthesis Demo](../../examples/research-synthesis/declarations/systems/system.yaml)

## Validation Commands

Use the CLI to validate any declaration against these schemas:

```bash
em declare validate --kind <kind> <path>
```

## IDE Support

You can associate these schemas with YAML files in editors like VS Code using the `yaml-language-server` or the Earmark VS Code extension (if installed).
