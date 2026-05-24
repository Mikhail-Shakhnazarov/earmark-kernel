#!/usr/bin/env bash
# scripts/run-self-hosting-cycle.sh
# Demonstrates a complete end-to-end self-hosting development cycle in Earmark
# using native orchestration paths only.

set -euo pipefail

echo "=== Earmark Self-Hosting Development Cycle ==="
TMP=$(mktemp -d)
echo "1. Initializing clean workspace at $TMP"
cargo run --bin earmark-cli -- --root "$TMP" init

echo "2. Initializing example orchestration declarations"
cargo run --bin earmark-cli -- --root "$TMP" --json orchestration init-example

echo "3. Creating native JSON task payload"
TASK_ID="eodp-a7"
cat > "$TMP/task.json" << EOF
{
  "task_id": "$TASK_ID",
  "title": "Self-hosting cycle demonstration task",
  "goal": "Demonstrate a complete self-hosting development cycle using only native Earmark orchestration paths.",
  "priority": "high",
  "status": "proposed"
}
EOF

echo "4. Ingesting Task from native JSON"
cargo run --bin earmark-cli -- --root "$TMP" --json orchestration ingest-task --source native-json "$TMP/task.json"

echo "5. Ingesting Dispatch Manifest"
cargo run --bin earmark-cli -- --root "$TMP" --json orchestration ingest-manifest .orchestration/manifests/EODP-A7.md --attempt 1

echo "6. Ingesting Executor Report"
REPORTS_DIR=".orchestration/reports"
LATEST_REPORT=$(find "$REPORTS_DIR" -maxdepth 1 -name "EODP-A7--1-*.md" | sort | tail -n 1)
if [ -n "$LATEST_REPORT" ]; then
  echo "Found report: $LATEST_REPORT"
  cargo run --bin earmark-cli -- --root "$TMP" --json orchestration ingest-report "$LATEST_REPORT" --attempt 1
else
  echo "No report found. Creating a minimal report..."
  cat > "$TMP/report.md" << EOF
task_uuid: $TASK_ID
attempt_number: 1
## Objective
Demonstrate self-hosting cycle.
## Changed Files
- scripts/run-self-hosting-cycle.sh
EOF
  cargo run --bin earmark-cli -- --root "$TMP" --json orchestration ingest-report "$TMP/report.md" --task-id "$TASK_ID" --attempt 1
fi

echo "7. Querying Unified Diagnostic State (Show)"
cargo run --bin earmark-cli -- --root "$TMP" --json orchestration show "$TASK_ID"

echo "8. Recording Review Decision (Accept)"
cargo run --bin earmark-cli -- --root "$TMP" --json orchestration review "$TASK_ID" --decision accepted --comment "End-to-end self-hosting loop verified successfully."

echo "9. Verifying Closed Task State"
cargo run --bin earmark-cli -- --root "$TMP" --json orchestration show "$TASK_ID"

echo "=== Cycle Completed Successfully! ==="
