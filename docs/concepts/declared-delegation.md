# Declared Delegation

Earmark is built on the principle of **Declared Delegation**. Instead of giving an AI full autonomy over a mess of files, you explicitly delegate specific work over specific materials.

## The Gap in Autonomous Agents

When you tell an autonomous agent to "fix the bugs in this repo," the agent often spends 80% of its time (and your tokens) scanning files, making assumptions about architecture, and "exploring" ambient context. When it finally makes a change, you have no easy way to see:
- Exactly which files it actually *understood* versus just *glanced at*.
- Which specific architectural goal it was trying to satisfy.
- How to "undo" just its exploration without rolling back your entire project.

## The Earmark Model: Explicit Handoffs

Declared delegation turns exploration into **coordinated stages**:

1. **Declared Intent:** You define a `work_item` (the goal) and its `task_materials` (the inputs).
2. **Scaffolded Workspace:** Earmark creates a bounded surface. The AI sees *only* those materials.
3. **Evidence-Based Results:** The AI produces a `change_set` and associated `evidence` (logs, tests, derivations).
4. **Formal Closure:** The work only becomes part of the "canonical spine" once a `review` relation validates the outcome.

### Contrast: Autonomous vs. Declared

| Feature | Autonomous Agent | Earmark Declared Delegation |
| :--- | :--- | :--- |
| **Context** | Open/Ambient | Bounded/Declared |
| **Trust** | Assumed | Verified via Gates & Review |
| **Durability** | Ephemeral Logs | Persistent Spine Artifacts |
| **Safety** | High Risk (unbounded access) | Low Risk (sandboxed materials) |

By declaring delegation, you maintain the "human-in-the-loop" where it matters most: at the **boundaries of authority**. You let the AI do the heavy lifting within those boundaries, while Earmark ensures the results are trackable, verifiable, and durable.
