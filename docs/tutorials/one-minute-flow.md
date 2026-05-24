# The One-Minute Flow

If you have Earmark built, you can run a complete multi-stage loop in under 60 seconds.

## 1. Setup
```bash
# Alias the binary
alias em="$(pwd)/target/debug/earmark-cli"

# Initialize a clean workspace
mkdir flow-demo && cd flow-demo
em init
```

## 2. Register & Input
```bash
# Register the research-synthesis demo
em system register ../examples/research-synthesis/declarations/systems/system.yaml
em system activate sys_research_synthesis

# Deposit a source note
em deposit --class source_note --title "Quick Test" --body "AI context should be task-specific."
```

## 3. Run & Inspect
```bash
# Run the extraction-and-synthesis loop
# (We pipe to jq to extract the object_id automatically)
OBJ_ID=$(em query --class source_note --json | jq -r '.[0].object_id')
em workflow run research_synthesis --system-id sys_research_synthesis --with $OBJ_ID

# See the result
em run explain latest
```

## What happened?
In those 60 seconds, Earmark:
1. Created a durable, Git-backed workspace.
2. Validated your input against the system's class definitions.
3. Compiled a **task-specific input set** for the AI.
4. Performed a **coordinated transition**, passing the extracted finding but withholding the source note for the final synthesis.
5. Recorded the entire chain of evidence in the **durable work spine**.

View your results in a browser:
```bash
em report run latest --output report.html
```
