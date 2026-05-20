# Derived Index

© 2026 Mikhail Shakhnazarov

The derived index is a projection of canonical state for query and operator inspection.

Hardening guarantees:

- rebuild is transactional;
- dirty marker is set before destructive rebuild and cleared after commit;
- corrupted/missing/payload-mismatch entries are skipped with diagnostics;
- failed rebuilds do not commit silently partial empty projections.

Operational surface:

- `em doctor` reports index health;
- `em doctor --repair-index` performs explicit rebuild and surfaces skipped entries.
