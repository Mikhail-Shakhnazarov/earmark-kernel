# Stdio Bridge

The Earmark CLI (`em`) is designed to work as a subprocess bridge for non-Rust runtimes. Use `--json` to get machine-readable output that follows a stable versioned contract.

## JSON Envelope

Every successful `--json` call returns:

```json
{
  "contract_version": "0.2.0",
  "data": { ... }
}
```

Errors are returned on stderr, also as JSON:

```json
{
  "contract_version": "0.2.0",
  "ok": false,
  "error": {
    "message": "..."
  }
}
```

## Python

```python
import subprocess
import json

def run_em(args):
    proc = subprocess.Popen(
        ["em", "--json"] + args,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True
    )
    stdout, stderr = proc.communicate()

    if proc.returncode != 0:
        try:
            err_data = json.loads(stderr)
            raise RuntimeError(f"[{err_data['contract_version']}] {err_data['error']['message']}")
        except json.JSONDecodeError:
            raise RuntimeError(f"CLI error: {stderr}")

    return json.loads(stdout)["data"]

# Deposit and run
note = run_em(["deposit", "--class", "source_note", "--body", "AI context should be bounded."])
run = run_em([
    "workflow", "run", "research_synthesis",
    "--system-id", "sys_research_synthesis",
    "--with", note["object_id"]
])
print(f"Run ID: {run['run_id']}")
```

## Node.js

```javascript
const { execSync } = require('child_process');

function runEm(args) {
    try {
        const stdout = execSync(`em --json ${args.join(' ')}`).toString();
        return JSON.parse(stdout).data;
    } catch (err) {
        console.error("CLI failed:", err.stderr?.toString());
        return null;
    }
}

const note = runEm(["deposit", "--class", "source_note", "--body", "'Hello world'"]);
console.log("Note ID:", note.object_id);
```

## Best Practices

1. **Check `contract_version`**: if it doesn't match your expected version (`0.2.0`), log a warning. Breaking changes increment the minor version.
2. **Handle exit codes**: a non-zero exit code means failure even if stderr isn't valid JSON.
3. **Use absolute paths**: when specifying `--root` or declaration files, use absolute paths.
4. **Use environment variables for secrets**: Earmark provider adapters read API keys from environment variables (e.g., `GOOGLE_API_KEY`), not from command-line arguments.

## See Also

- [Runtime Contract](runtime-contract.md) — the full six-step flow and JSON artifact shapes
- [Runtime Integration Guide](runtime-integration-guide.md) — Rust SDK and Python examples
