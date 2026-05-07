# Earmark Stdio Bridge

The Earmark CLI (`em`) is designed to be used as a subprocess bridge for non-Rust runtimes. By using the `--json` flag, you get machine-readable output that follows a stable contract.

## 1. The JSON Envelope

Every successful CLI call with `--json` returns a JSON object with this structure:

```json
{
  "contract_version": "0.2.0",
  "data": { ... }
}
```

Errors are returned on `stderr` and also formatted as JSON:

```json
{
  "contract_version": "0.2.0",
  "ok": false,
  "error": {
    "message": "..."
  }
}
```

## 2. Integration Patterns

### Python

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
            print(f"Error [{err_data['contract_version']}]: {err_data['error']['message']}")
        except:
            print(f"CLI Error: {stderr}")
        return None

    return json.loads(stdout)["data"]

# Example: Deposit and Run
note = run_em(["deposit", "--class", "source_note", "--body", "Search for AI safety."])
if note:
    run = run_em(["workflow", "run", "search", "--system-id", "sys1", "--with", note["object_id"]])
    print(f"Run ID: {run['run_id']}")
```

### Node.js

```javascript
const { execSync } = require('child_process');

function runEm(args) {
    try {
        const stdout = execSync(`em --json ${args.join(' ')}`).toString();
        const response = JSON.parse(stdout);
        return response.data;
    } catch (err) {
        console.error("CLI Failed:", err.stderr.toString());
        return null;
    }
}

const note = runEm(["deposit", "--class", "source_note", "--body", "'Hello world'"]);
console.log("Note ID:", note.object_id);
```

## 3. Best Practices

1. **Check Contract Version**: If `contract_version` does not match your expected version (e.g., `0.2.0`), emit a warning as breaking changes might have occurred.
2. **Handle Exit Codes**: A non-zero exit code indicates failure even if `stderr` is not valid JSON.
3. **Use Absolute Paths**: When using `--root` or referring to declaration files, use absolute paths to avoid ambiguity.
4. **Environment Variables**: Use environment variables for sensitive data (like `GOOGLE_API_KEY`) rather than passing them as arguments. Earmark adapters read these directly from the environment.
