# Coordinated AI Work

AI-assisted work usually runs on ephemeral, ambient context. Earmark replaces this with a **work spine**.

## The Ambient Context Problem

Imagine a research project with three stages:
1. **Extraction:** Read 50 articles and find 10 key findings.
2. **Analysis:** Group findings into 3 themes.
3. **Drafting:** Write a briefing based on the themes.

In a traditional LLM chat or a standard "agent" loop, the **Drafting** stage often has access to the full text of all 50 articles AND the intermediate findings. This creates two major problems:

- **Implicit Inheritance:** The AI might include a detail from an article that didn't make it into the "key findings," silently bypassing the human's review of the extraction stage.
- **Context Noise:** The AI is overwhelmed by 50 full articles when it only needs the 3 validated themes to do its job accurately.

## The Durable Spine Solution

Earmark solves this by treating every stage as a **coordinated handover** of authoritative artifacts:

1. **Declared Task Materials:** You define exactly which objects (e.g., specific Findings) a transition is allowed to see.
2. **Bounded Surface:** The system compiles a task-specific input set that excludes everything else.
3. **Trackable Causality:** Every new object links back to its "causes" through explicit relations, creating a verifiable chain of evidence.

### Benefits of Coordination

- **Inspectability:** You can look at any stage and see exactly what inputs were used to produce exactly what outputs.
- **Resumability:** Because state is durable and locked to Git, anyone can pick up the work spine at any point.
- **High Integrity:** Later steps build only on validated earlier results, reducing the risk of errors born from leftover context noise.

Coordinated AI work isn't just about getting an answer; it's about building a **durable corpus of knowledge** that remains authoritative long after the AI session ends.
