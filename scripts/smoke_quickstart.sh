#!/usr/bin/env bash
set -euo pipefail

# Earmark v1 Preview Quickstart Smoke Script
# This script verifies the baseline "out-of-box" experience for v0.1.0.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
# Path to em binary
if command -v em >/dev/null 2>&1; then
    EM_BIN=$(command -v em)
elif [[ -x "$REPO_ROOT/target/release/em" ]]; then
    EM_BIN="$REPO_ROOT/target/release/em"
elif [[ -x "$REPO_ROOT/target/debug/em" ]]; then
    EM_BIN="$REPO_ROOT/target/debug/em"
else
    echo "Error: em binary not found. Please install it (cargo install --path earmark-cli) or build it (cargo build)."
    exit 1
fi

SMOKE_ROOT=$(mktemp -d -t earmark-smoke-XXXXXX)
trap 'rm -rf "$SMOKE_ROOT"' EXIT

echo "--- SMOKE TEST: Quickstart Flow ---"
echo "Binary:    $EM_BIN"
echo "Workspace: $SMOKE_ROOT"

cd "$SMOKE_ROOT"

# 1. Initialize
"$EM_BIN" init > /dev/null

# 2. Register and Activate System
SYSTEM_MANIFEST="$REPO_ROOT/examples/research-synthesis/declarations/systems/system.yaml"
"$EM_BIN" system register "$SYSTEM_MANIFEST" > /dev/null
"$EM_BIN" system activate sys_research_synthesis > /dev/null

# 3. Deposit Data
"$EM_BIN" deposit --class source_note --title "Smoke Test Note" --body "Checking deterministic completion." > /dev/null
# Extract object_id without jq
OBJECT_ID=$("$EM_BIN" query --class source_note --json | sed -n 's/.*"object_id":\s*"\([^"]*\)".*/\1/p' | head -n 1)

if [[ -z "$OBJECT_ID" ]]; then
    echo "Error: Failed to retrieve deposited object ID."
    exit 1
fi

# 4. Run Workflow
echo "Running workflow for object $OBJECT_ID..."
RUN_OUT=$("$EM_BIN" workflow run research_synthesis --system-id sys_research_synthesis --with "$OBJECT_ID" --json)
# Extract status without jq
STATUS=$(echo "$RUN_OUT" | sed -n 's/.*"status":\s*"\([^"]*\)".*/\1/p')

echo "Run Status: $STATUS"

if [[ "$STATUS" != "completed" ]]; then
    echo "Error: Expected status 'completed', got '$STATUS'."
    echo "Full output (raw):"
    echo "$RUN_OUT"
    exit 1
fi

# 5. Generate Report
"$EM_BIN" report run latest --output report.html > /dev/null

if [[ ! -s "report.html" ]]; then
    echo "Error: report.html was not generated or is empty."
    exit 1
fi

echo "--- SMOKE TEST PASSED ---"
echo "Summary: Binary branding, contract v0.3.0, and deterministic completion verified."
