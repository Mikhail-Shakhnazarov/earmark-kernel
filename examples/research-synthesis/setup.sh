#!/usr/bin/env bash
set -e

# Earmark Research Synthesis Demo Setup
# This script initializes an external workspace, registers declarations, and deposits seed data.

if [ -z "${REPO_ROOT:-}" ]; then
  REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
fi

WORKSPACE="${WORKSPACE:-/tmp/earmark-research-synthesis-demo}"

EM_BIN="${EM_BIN:-$REPO_ROOT/target/debug/earmark-cli}"

if [ ! -x "$EM_BIN" ]; then
  echo "Expected CLI at $EM_BIN"
  echo "Build it first from the repository root:"
  echo "  cargo build -p earmark-cli"
  exit 1
fi

em() {
  "$EM_BIN" --root "$WORKSPACE" "$@"
}

echo "--- Initializing Workspace ---"
rm -rf "$WORKSPACE"
em init

echo "--- Registering System ---"
em system register "$REPO_ROOT/examples/research-synthesis/declarations/systems/system.yaml"

echo "--- Activating System ---"
em system activate sys_research_synthesis

echo "--- Depositing Seed Notes ---"
em deposit --class source_note --title "Federated Graphs: Agility and Ownership" --payload-file "$REPO_ROOT/examples/research-synthesis/data/seed_notes/note_1_benefits.md"
em deposit --class source_note --title "The Cost of Heterogeneity" --payload-file "$REPO_ROOT/examples/research-synthesis/data/seed_notes/note_2_challenges.md"
em deposit --class source_note --title "Distributed Query Latency" --payload-file "$REPO_ROOT/examples/research-synthesis/data/seed_notes/note_3_performance.md"
em deposit --class source_note --title "Auditing Federated Transitions" --payload-file "$REPO_ROOT/examples/research-synthesis/data/seed_notes/note_4_governance.md"

echo "--- Setup Complete ---"
echo "Workspace: $WORKSPACE"
echo "Next step: Run the first stage of the synthesis workflow."
echo "Command: $EM_BIN --root \"$WORKSPACE\" workflow run research_synthesis --system-id sys_research_synthesis --with <object_id>"
echo "(Note: You can find object IDs using '$EM_BIN --root \"$WORKSPACE\" query --class source_note')"
