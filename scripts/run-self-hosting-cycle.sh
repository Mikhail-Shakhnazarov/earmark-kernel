#!/usr/bin/env bash
# scripts/run-self-hosting-cycle.sh
# Demonstrates a complete end-to-end self-hosting development cycle in Earmark.

set -euo pipefail

echo "=== Earmark Self-Hosting Development Cycle ==="
TMP=$(mktemp -d)
echo "1. Initializing clean workspace at $TMP"
cargo run --bin earmark-cli -- --root "$TMP" init

echo "2. Initializing example orchestration declarations"
cargo run --bin earmark-cli -- --root "$TMP" --json orchestration init-example

echo "3. Ingesting Task EODP-A9 from Engram store"
cargo run --bin earmark-cli -- --root "$TMP" --json orchestration ingest-task a98596b8

echo "4. Ingesting Dispatch Manifest"
# We'll use our own EODP-A7 manifest as a realistic example
cargo run --bin earmark-cli -- --root "$TMP" --json orchestration ingest-manifest .orchestration/manifests/EODP-A7.md --attempt 1

echo "5. Ingesting Executor Report"
# We'll use one of the generated reports from EODP-A7
REPORTS_DIR=".orchestration/reports"
LATEST_REPORT=$(find "$REPORTS_DIR" -maxdepth 1 -name "EODP-A7--1-*.md" | sort | tail -n 1)
if [ -n "$LATEST_REPORT" ]; then
  echo "Found report: $LATEST_REPORT"
  cargo run --bin earmark-cli -- --root "$TMP" --json orchestration ingest-report "$LATEST_REPORT" --attempt 1
else
  echo "No report found. Creating dummy report..."
  DUMMY_REPORT="$TMP/dummy_report.md"
  cat <<EOF > "$DUMMY_REPORT"
## Objective
Implement show.

## Changed Files
- earmark-cli/src/app/commands/orchestration.rs
EOF
  cargo run --bin earmark-cli -- --root "$TMP" --json orchestration ingest-report "$DUMMY_REPORT" --task-id eodp-a7 --attempt 1
fi

echo "6. Querying Unified Diagnostic State (Show)"
cargo run --bin earmark-cli -- --root "$TMP" --json orchestration show a98596b8

echo "7. Recording Review Decision (Accept)"
cargo run --bin earmark-cli -- --root "$TMP" --json orchestration review a98596b8 --decision accepted --comment "End-to-end self-hosting loop verified successfully."

echo "8. Verifying Closed Task State"
cargo run --bin earmark-cli -- --root "$TMP" --json orchestration show a98596b8

echo "=== Cycle Completed Successfully! ==="
