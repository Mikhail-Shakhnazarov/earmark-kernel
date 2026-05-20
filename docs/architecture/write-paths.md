# Write Paths

© 2026 Mikhail Shakhnazarov

Earmark has one canonical write path:

1. Validate declaration/runtime constraints.
2. Write immutable object versions into canonical store.
3. Advance object heads.
4. Upsert derived index projections.

Durable write entry points are:

- declaration registration;
- deposit;
- workflow execution transitions;
- governed standing transitions;
- privileged/system relations (only through authorization checks).

Non-init commands must not create workspace layout implicitly. Workspace creation is explicit via `em init`.
