# Relation Authorization

Relations created through the declared runtime path are authorized before creation. The authorization decision is recorded as headers on the relation object, making it inspectable after the fact.

## Why This Matters

A relation is a directed link between two objects. When you see a relation in the workspace, you need to know why it was allowed: which class's rule authorized it, and under what authority.

## What Earmark Records

When a relation is created, Earmark writes four headers:

| Header | Description |
|---|---|
| `relation_auth_endpoint` | Which endpoint's rule matched: `source` or `target` |
| `relation_auth_class` | The class whose rule authorized the relation |
| `relation_auth_authority` | The authorization authority: `source`, `target`, `either_endpoint`, or `privileged` |
| `relation_auth_direction` | The rule direction that matched: `outgoing`, `incoming`, `bidirectional`, `either`, or `system` |

## Inspecting Authorization

Use `em relation explain <relation_id>` to see the authorization trace:

```bash
em relation explain obj_abc123
```

The `related.authorization` section surfaces the evidence:

```json
{
  "related": {
    "source": { "object_id": "obj_...", "class": "source_note" },
    "target": { "object_id": "obj_...", "class": "finding" },
    "relation_type": "mentions",
    "creation_mode": "declared",
    "authorization": {
      "endpoint": "source",
      "class": "source_note",
      "authority": "source",
      "direction": "outgoing"
    }
  }
}
```

## Examples

### Source-Authorized Relation

A `source_note` declares an outgoing `mentions` rule targeting `finding`. When a relation of type `mentions` is created from a `source_note` to a `finding`, the source's rule authorizes it:

```json
{
  "authorization": {
    "endpoint": "source",
    "class": "source_note",
    "authority": "source",
    "direction": "outgoing"
  }
}
```

### Target-Authorized Relation

A `finding` declares an incoming `referenced_by` rule targeting `source_note`. A relation from `source_note` to `finding` is authorized by the target's rule:

```json
{
  "authorization": {
    "endpoint": "target",
    "class": "finding",
    "authority": "target",
    "direction": "incoming"
  }
}
```

### Either-Endpoint Authorization

A rule with `authorizing_endpoint: either_endpoint` allows either the source-side rule or the target-side rule to authorize the relation, provided the rule direction matches that endpoint's position in the relation. The recorded authority is `either_endpoint`; the recorded endpoint still shows which side's rule matched.

## Relation Show vs Explain

- `em relation show <id>` returns the raw stored object.
- `em relation explain <id>` returns a structured summary including the authorization trace, source/target references, and suggested next commands.
