# Incident Response Fixture

Purpose: This fixture is a committed, reproducible declaration set for incident triage and response flows.

Behavior under test:
- Path-system manifest registration and activation
- Admission checks for incident-oriented classes
- Workflow execution from incident input through analysis outputs
- Handoff and run artifact generation

Declarations included:
- `declarations/systems/system.yaml`
- `declarations/classes/*.yaml`
- `declarations/contexts/*.yaml`
- `declarations/instructions/*.md`
- `declarations/profiles/*.yaml`
- `declarations/workflows/*.yaml`

How to run from a temporary workspace:
1. Build CLI from repository root: `cargo build -p earmark-cli --all-features`
2. Create temp workspace and copy fixture declarations.
3. Initialize workspace: `earmark-cli init`
4. Register and activate system:
   - `earmark-cli system register declarations/systems/system.yaml`
   - `earmark-cli system activate incident_response_v1`
5. Deposit incident input and run workflow.

Expected artifacts:
- Input `incident` object accepted in active namespace
- Run record, assignments, change sets, and handoffs
- Output objects matching workflow output contracts

Do not commit generated files:
- `.earmark` state
- `.earmark_lock`
- SQLite index files
- work-surface artifacts
- generated `corpus/obj_*.md` files

Run this fixture from a temporary workspace. Do not commit generated `.earmark` state, `.earmark_lock`, SQLite indexes, work surfaces, or generated corpus object files.
