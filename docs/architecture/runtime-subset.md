# Runtime Subset

© 2026 Mikhail Shakhnazarov

Supported workflow operation/output subset:

| Operation kind | Supported output contracts |
|---|---|
| `compile_context` | one work packet/context object as declared |
| `transform` | zero or one output class |
| `review` | one or more forwarded classes if declared |
| `export` | no output object required |

Current constraints:

- multi-output `transform` declarations are rejected before execution;
- partial execution is represented explicitly as `partial` run status;
- unreached transitions are recorded in run analysis.
