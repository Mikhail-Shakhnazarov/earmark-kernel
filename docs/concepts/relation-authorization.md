# Verified Relationships

In Earmark, a link between two objects (a **relation**) is only useful if the system can explain *why* that link was permitted.

If a `source_note` links to a `finding`, that relationship affects the lineage of all future work. Earmark doesn't just record the link; it records the **verification proof** that authorized it.

## Why This Matters

Relationships are not just hyperlinks. They define the "mesh" of evidence that supports your AI work. By recording the authorization for every link, Earmark provides a definitive answer to the question: **"Why was this connection allowed?"**

## The Verification Proof

When a relationship is created, Earmark attaches metadata that traces the link back to a specific rule in your system definition:

| Detail | Description |
|---|---|
| **Authorizing Class** | The specific object type (e.g., `source_note`) whose rules allowed the link. |
| **Direction** | Whether it was an `outgoing` rule, an `incoming` rule, or a system-level authority. |
| **Endpoint** | Which side of the link provided the authorization. |

## Inspecting Proofs

You can inspect the authorization trace for any relationship using the `explain` command:

```bash
em relation explain <relation_id>
```

This returns a structured view of the link, including the source and target objects and the exact rule that matched.

---

- [Task-Specific Context](context-compilation.md) — how relationships control visibility
- [Carrying Work Forward](handoffs.md) — how relationships enable transitions
