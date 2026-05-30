# Architecture

Most tools for AI-assisted work treat the record of the work as an accident.
There is a prompt somewhere, some output somewhere else, maybe a few saved
files, and a lot of state that only exists because one particular session is
still alive. Earmark takes the opposite approach. The record comes first.

The kernel is a set of five Rust crates that describe, store, and check a
governed work record.

## The Five Crates

### `earmark-core`

`earmark-core` defines the shared language of the system: typed identifiers,
records, standing, and the basic data structures used everywhere else.

This is the crate that answers questions like:

- what is an object
- what is a version
- how are relations identified
- how is standing represented

### `earmark-store`

`earmark-store` is the durable layer. It writes the canonical record to disk
under `.earmark/` and reads it back without requiring a database server or an
external service.

A stored object is not just a blob of text. It sits inside a structure that can
track new versions, relations to other objects, check results, dispatch records,
and governance state.

### `earmark-index`

The file-backed store is the source of truth. The SQLite index is there for
speed and lookup convenience.

That distinction matters. If the index is damaged or deleted, the kernel can
rebuild it from the canonical store. The durable record does not depend on the
derived view.

### `earmark-declarations`

`earmark-declarations` defines the registries for classes, systems, workflows,
packet templates, and related declarations.

That is how the same underlying store can support different kinds of governed
work while keeping the persistent model readable and explicit.

### `earmark-governance`

`earmark-governance` is the crate reserved for governance-facing logic and
interfaces. At this stage it is still small compared with the rest of the
workspace, but its role is clear: keep governance as an explicit part of the
system rather than leaving it as custom behavior around the edges.

## What The Kernel Actually Stores

At a practical level, the kernel stores things like:

- objects and their later versions
- typed relations between those objects
- dispatch and packet records
- check results
- review and standing state

The important part is not the nouns. The important part is that all of this is
kept as inspectable state on disk instead of disappearing into process memory or
chat history.

## Design Choice

This project takes a narrower path than most AI tooling. Its concern is the
work record itself: keep it durable, structured, and checkable, then let other
software build on top of that record.
