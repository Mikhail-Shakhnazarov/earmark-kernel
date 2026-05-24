# Declared Domains

Earmark lets you define exactly how AI work happens through **Declarations**. Instead of code, you use YAML and Markdown to describe your data types, LLM prompts, and workflows.

## The Scaffolding Workflow

The easiest way to start is by scaffolding a new system:

1. **Scaffold a class:**
   ```bash
   em declare new --kind class finding
   ```
2. **Scaffold an instruction:**
   ```bash
   em declare new --kind instruction source_to_finding
   ```
3. **Scaffold a system:**
   ```bash
   em declare new --kind system research_synthesis
   ```

## Validating Your Work

Earmark enforces strict validation. Use `em declare validate` to check your schemas before registering them:

```bash
# Check a class definition
em declare validate --kind class my_classes/finding.yaml

# Check a workflow graph
em declare validate --kind workflow my_workflows/extraction.yaml

# Check an entire system manifest
em declare validate --kind system my_system/system.yaml
```

## Exploratory Inspection

Use `em declare explain` to see how the system interprets your declarations:

```bash
em declare explain --kind system sys_research_synthesis
```

---

## What's in a Declaration?

- **[Classes](REFERENCE.md#declaration-kinds)**: Define object types and who is allowed to create them.
- **[Instructions](REFERENCE.md#declaration-kinds)**: The prompts and purposes sent to the AI.
- **[Workflows](REFERENCE.md#declaration-kinds)**: The staged graph of operations.
- **[Systems](REFERENCE.md#declaration-kinds)**: The manifest that ties everything together.

For technical details, schemas, and validation tables, see the **[Declaration Reference](REFERENCE.md)**.

---

## Examples

Standalone examples can be found in:
`docs/declarations/examples/`

For a production-grade example, see the **[Research Synthesis Demo](../../examples/research-synthesis/declarations/systems/system.yaml)**.
