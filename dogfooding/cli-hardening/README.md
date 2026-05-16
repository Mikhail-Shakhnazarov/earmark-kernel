# CLI Hardening Fixture

Purpose: This fixture captures declaration scenarios used to harden CLI registration, validation, and workflow execution behavior.

Behavior under test:
- System manifest path resolution and declaration registration
- Instruction/workflow parsing and strict contract handling
- Diagnostic and audit command output stability for fixture flows

Declarations included:
- `declarations/systems/system.yaml`
- `declarations/classes/*.yaml`
- `declarations/instructions/*.md`
- `declarations/workflows/*.yaml`

How to run from a temporary workspace:
1. Build CLI from repository root: `cargo build -p earmark-cli --all-features`
2. Copy fixture declarations into a temp workspace.
3. Initialize and doctor workspace:
   - `earmark-cli init`
   - `earmark-cli doctor`
4. Register/activate system and run fixture workflow.

Expected artifacts:
- Successful declaration registration and system activation
- Workflow run artifacts (run record, assignments, change sets)
- Queryable output objects for fixture classes

Do not commit generated files:
- `.earmark` state
- `.earmark_lock`
- SQLite index files
- work-surface artifacts
- generated `corpus/obj_*.md` files

Run this fixture from a temporary workspace. Do not commit generated `.earmark` state, `.earmark_lock`, SQLite indexes, work surfaces, or generated corpus object files.
