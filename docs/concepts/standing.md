# Evaluation and Verification

Every piece of work in Earmark carries **evaluation metadata**. This metadata tracks exactly where an object is in its lifecycle — for example, whether it is a "draft" or "verified", "accepted" or "rejected".

## How It Works

This metadata is stored as a simple map of dimensions and tokens:

```yaml
standing:
  kernel:epistemic: working
  kernel:review: unreviewed
  kernel:process: active
  research:status: draft
```

Dimensions like `review` or `status` are defined by the domain author. This allows you to categorize work precisely for your specific use case (e.g., "Medical Review", "Legal Approval").

## Lifecycle Rules

Earmark doesn't care about the *names* of your categories. Instead, it uses **lifecycle rules** to bind your metadata to system behavior.

For example, a rule might say:
> "Only objects with the token `verified` are allowed to be included in the synthesis stage."

This decoupling allows you to use your own terminology while the system enforces your strict evaluation requirements behind the scenes.

## Why This Matters

- **Trust Transitions**: Work only moves forward when it meets your specific criteria.
- **Auditable Quality**: You can see exactly who verified a piece of data and which criteria were used.
- **Customizable Gatekeeping**: Define your own multi-stage review gates without changing any code.

## Related

- [The Durable Work Spine](staged-execution.md) — how evaluation affects work transitions
- [Task-Specific Context](context-compilation.md) — how metadata controls what the AI sees
