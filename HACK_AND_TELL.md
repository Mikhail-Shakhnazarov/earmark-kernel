# Earmark Hack-and-Tell

## What this presentation is

This presentation should not frame Earmark as a terminal tool for a human operator.

It should frame Earmark as substrate for a model.

The visible product surface is a single chat inside the IDE. The model uses Earmark underneath to keep durable state, preserve bounded context, route work, and continue across steps without treating ambient chat history as memory.

That is the core reveal.

## Core claim

Earmark makes it possible to build a chat-first application surface on top of governed external state.

What appears to be one coherent conversational agent is actually a model working against:

- durable objects
- bounded handoffs
- inspectable work state
- resumable continuity
- self-hosted storage and orchestration

## Framing

Two angles belong together.

### 1. Governed AI work

The important distinction is between ephemeral chat continuity and governed continuity.

Ordinary AI tools often rely on long prompt history, retrieval haze, and hidden inheritance between steps.

Earmark introduces:

- declared objects instead of loose pasted context
- bounded successor surfaces instead of ambient prompt carryover
- durable handoffs instead of invisible continuation
- explicit work state instead of “the model probably remembers”

### 2. Die App

Because the substrate is self-hosted and general, the chat surface is not the product itself. It is one surface over a more general continuity layer.

That means the same foundation can support many application forms where a model needs:

- memory
- staged work
- inspectable evidence
- bounded synthesis
- resumable task state
- durable artifacts

The point is not “a nice CLI exists.” The point is “a model-facing backend exists.”

## Non-goals for the presentation

- Do not lead with terminal commands.
- Do not present Earmark as a developer utility first.
- Do not make model output quality the main proof.
- Do not claim that conversational continuity is magical memory.
- Do not overclaim the current demo path as a fully completed autonomous loop.

## The interface

The interface should be the Codex chat in VS Code.

The terminal may exist in the background if needed for setup before the talk, but during the live presentation the interaction surface should remain the IDE chat plus selective file reveals inside the editor.

The audience should come away with this mental model:

“one chat on the surface, governed substrate underneath”

## Recommended demo case

### Recommendation: knowledge briefing from scattered notes

This is the strongest current case for a short IDE-only demo.

Why it works:

- understandable in under 20 seconds
- looks like a real app pattern rather than an infra stunt
- makes state and continuity matter
- allows one clean artifact reveal inside the editor
- does not depend on showing the terminal
- fits the current Earmark research/briefing examples well

### Narrative

A model is asked, in chat, to turn a small pile of messy source notes into a reusable briefing.

The important move is not “the model wrote a summary.”

The important move is:

1. the notes become governed objects
2. findings are extracted as durable intermediate state
3. the synthesis surface is bounded
4. the result is resumable and inspectable

That reads like an app.

### Why this case is better than a coding-task demo

A coding-task demo inside VS Code risks collapsing the presentation into “agent coding with extra storage.”

A briefing demo keeps the architecture legible:

- input material
- governed intake
- durable intermediate state
- bounded continuation
- final artifact

That makes the Earmark layer easier to perceive.

## Alternate cases

If the briefing case feels too document-heavy, two alternates are plausible.

### 1. Incident triage

Chat asks for a brief incident assessment from scattered notes, logs, and observations.

Strengths:

- operational
- obviously multi-step
- strong case for evidence and auditability

Weakness:

- can feel more enterprise and less immediately universal

### 2. Research assistant with persistent line of thought

Chat builds a position or synthesis over several turns and later resumes from durable objects.

Strengths:

- makes “memory” and continuity vivid

Weakness:

- risks becoming abstract unless a single artifact reveal is very clear

## Recommended live sequence

### Stage 1: ordinary chat surface

Start in the IDE chat with a natural request:

“Create a briefing from these notes and preserve anything worth reusing later.”

No terminal.

No CLI framing.

The opening should feel like ordinary chat software.

### Stage 2: show that the chat is doing more than chatting

Have the model describe its own work in app terms:

- ingesting notes
- extracting findings
- preserving durable intermediate state
- preparing a bounded synthesis surface

The language should imply governed backend behavior without dropping into tool-demo mode too early.

### Stage 3: one reveal in the editor

Open one artifact in the workspace.

Best reveal candidates:

- a generated briefing artifact
- a finding object
- a handoff-related record if it is legible enough

The reveal should prove that continuity is externalized into durable workspace state.

The reveal should not become a file safari.

### Stage 4: return to chat

Ask for revision, extension, or resumption:

- revise the briefing for a different audience
- continue from the saved findings
- exclude one weak source note
- produce a shorter executive version

This is the moment where the architectural claim lands:

the chat is continuing from governed external state, not merely from conversational drift.

### Stage 5: close on the architecture

End with the generalization:

This is not a demo of terminal tooling.

This is a demo of a chat-first app surface backed by self-hosted governed model infrastructure.

## Suggested talk track

### Opening

“Most AI products use the chat as both interface and memory. That works until continuity matters.”

“This setup separates those layers. The chat remains the interface, but the continuity layer lives underneath.”

### Middle

“What matters here is not that a model can produce a summary. What matters is that the work leaves durable, inspectable state.”

“The visible surface is one ongoing conversation. Underneath, the model is working against governed artifacts and bounded handoffs.”

### Close

“That is why this is not mainly a CLI story. It is an application substrate story.”

“One chat on top; durable governed continuity underneath.”

## Current implementation honesty

The presentation should stay aligned with the code as it exists now.

Important current-state note:

- the research-synthesis example currently lands in `partial` on the verified local path
- the CLI test surface currently asserts that `partial` status for the demo path
- therefore the live presentation should not hinge on claiming a fully completed autonomous pipeline in the default path

That does not weaken the core point.

It actually supports the governance claim: the system records bounded continuation explicitly instead of pretending unfinished work is complete.

## Practical presentation rule

Only reveal backend structure at the exact moment needed to prove that the chat has real governed continuity behind it.

Everything else should stay inside the IDE chat surface.

## Immediate next step

Build the live demo around one concise briefing scenario:

- 3 to 5 source notes
- one briefing target
- one durable artifact reveal
- one follow-up revision that proves resumability
