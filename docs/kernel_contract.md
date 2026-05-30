# Earmark Kernel Contract (v0.1)

This document defines the formal guarantees provided by the Earmark Kernel. These guarantees form the technical floor upon which all operational workers and corpora are built.

## 1. Durable Record Model
The Kernel guarantees that all state is captured as **Canonical Records** in the local file system.
- **Object Records**: Define the existence and class of a system object.
- **Version Records**: Capture a specific immutable state (payload, standing, and signal).
- **Storage Format**: Records are stored as atomic JSON files within the `.earmark/objects/` directory.

## 2. Identity and Validation
The Kernel enforces strict structural discipline for all identifiers (`IdSpec`).
- **Uniqueness**: Identifiers must be unique within their kind (Object, Version, Actor, Class).
- **Validation**: Identifiers must reject path separators, whitespace, and empty bodies to ensure filesystem safety and URI compatibility.
- **Integrity**: Any record with a malformed ID will be rejected at the storage boundary.

## 3. Local Sovereignty
The Kernel is **Runtime Independent**.
- **Self-Hosting**: The kernel logic (core, store, index) must function correctly without an internet connection, private provider, or external database (SurrealDB is optional/experimental).
- **Verifiable Index**: The system index (SQLite) must be fully reconstructible from the canonical `.earmark/objects/` store at any time.

## 4. Stability Guarantees
- **Format Stability**: The v0.1 JSON schema for Object and Version records is considered stable.
- **Compatibility**: Future kernel versions must provide automated migration paths for v0.1 stores.
