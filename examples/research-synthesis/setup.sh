#!/bin/bash
set -e

# Earmark Research Synthesis Demo Setup
# This script initializes the workspace, registers declarations, and deposits seed data.

echo "--- Initializing Workspace ---"
em init

echo "--- Registering Declarations ---"
em declare register --kind class declarations/classes/source_note.yaml
em declare register --kind class declarations/classes/finding.yaml
em declare register --kind class declarations/classes/summary.yaml
em declare register --kind instruction declarations/instructions/source_to_finding.md
em declare register --kind instruction declarations/instructions/finding_to_summary.md
em declare register --kind compiled-context declarations/compiled_contexts/source_notes_for_extraction.yaml
em declare register --kind compiled-context declarations/compiled_contexts/findings_for_summary.yaml
em declare register --kind provider-profile declarations/provider_profiles/local_mock.yaml
em declare register --kind provider-profile declarations/provider_profiles/google_gemini.yaml
em declare register --kind workflow declarations/workflows/research_synthesis.yaml
em declare register --kind system declarations/systems/system.yaml

echo "--- Activating System ---"
em system activate sys_research_synthesis

echo "--- Depositing Seed Notes ---"
em deposit --class source_note --title "Federated Graphs: Agility and Ownership" --payload-file data/seed_notes/note_1_benefits.md
em deposit --class source_note --title "The Cost of Heterogeneity" --payload-file data/seed_notes/note_2_challenges.md
em deposit --class source_note --title "Distributed Query Latency" --payload-file data/seed_notes/note_3_performance.md
em deposit --class source_note --title "Auditing Federated Transitions" --payload-file data/seed_notes/note_4_governance.md

echo "--- Setup Complete ---"
echo "Next step: Run the first stage of the synthesis workflow."
echo "Command: em workflow run research_synthesis --system-id sys_research_synthesis --with <object_id>"
echo "(Note: You can find object IDs using 'em query --class source_note')"
