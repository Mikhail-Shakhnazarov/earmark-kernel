# Declaration Schemas

Earmark declarations use YAML for structured definitions and Markdown with YAML frontmatter for instructions. The CLI validates declarations against internal schemas.

## Declaration Kinds

### Class

Defines an object type in the corpus.

**Fields**: `name`, `version`, `kind`, `required_headers`, `payload_schema`, `standing_rules`, `relation_rules`, `validators`.

```yaml
name: finding
version: 0.2.0
kind: object
required_headers:
  - title
payload_schema: inline:any
standing_rules:
  allowed_standing:
    kernel:epistemic: [working, supported]
    kernel:review: [unreviewed, accepted]
    kernel:process: [active, completed]
relation_rules:
  - relation_type: derived_from
    counterparty_classes: [source_note]
validators: []
```

### Instruction

Defines a processing operation as Markdown with YAML frontmatter.

**Frontmatter fields**: `name`, `version`, `purpose`, `input_classes`, `output_classes`, `execution_policy`, `provider_profile`, `trace_policy`, `register`.

**Body**: Markdown prose describing the task, constraints, and expected output structure.

```markdown
---
name: source_to_finding
version: 0.2.0
purpose: Extract discrete findings from source notes.
input_classes: [source_note]
output_classes: [finding]
execution_policy: runtime_permitted
provider_profile: null
trace_policy: summary
register: findings
---

# Finding Extraction

Extract discrete findings from the provided source notes...
```

### Workflow

Defines a sequence of transitions.

**Fields**: `name`, `version`, `description`, `operations`, `edges`, `guards`.

Each operation has: `id`, `kind` (`compile_context` or `transform`), `input_contracts`, `output_contracts`, and either a `compiled_context` reference or an `instruction` reference.

### Compiled Context Template

Defines rules for compiling bounded context from the store.

**Fields**: `name`, `version`, `description`, `select` (classes, standing, relations, time_range), `group_by`, `render` (mode, manifest_format, prose_template), `visibility` (include_lineage, include_constraints, include_provenance).

### Provider Profile

Connects a transition to a specific LLM provider.

**Fields**: `name`, `version`, `provider`, `model`, `auth_env`, `budget` (max_output_tokens, max_latency_ms), `response_contract` (format, must_return_candidate_only).

### System Manifest

Bundles declarations into a deployable domain.

**Fields**: `system_id`, `namespace`, `title`, `description`, `classes`, `instructions`, `workflows`, `compiled_contexts`, `provider_profiles`, `standing_policies`, `default_compiled_context`, `default_provider_profile`, `runtime_profile`.

## Validation

```bash
em declare validate --kind <kind> <path>
```

Kinds: `class`, `instruction`, `workflow`, `compiled-context`, `provider-profile`, `system`.

The system-level validator checks cross-references: classes mentioned in instructions must exist in the system, relation targets must reference declared classes, and workflow operations must reference declared instructions or compiled contexts.
