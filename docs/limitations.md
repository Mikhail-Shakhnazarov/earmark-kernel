# Limitations

Earmark is in early release. While the core "work spine" and orchestration logic are operational, the following limitations apply to the current version:

## 1. Linux & NixOS Focus
The current toolset and verification scripts are optimized for Linux (specifically NixOS). While the Rust core is cross-platform, the helper scripts and automated verification paths may require adjustments for macOS or Windows (WSL2 recommended).

## 2. Local-First Architecture
Earmark currently runs as a local CLI tool using a Git-backed store. There is no central server or multi-user web dashboard in this release. All collaboration happens through the shared Git repository.

## 3. Single-Operator Assumptions
The orchestration logic assumes a single operator is executing dispatches at any given time. While the data model is designed for multiple actors, the current runtime does not perform complex locking for concurrent dispatches from different machines.

## 4. Manual Verification Gates
While the system supports automated gates (e.g., "all tests must pass"), many high-level "acceptance" steps in the orchestration lifecycle currently require manual operator commands (`em review`).

## 5. Storage Performance
The Git-backed store is extremely robust, but the SQLite index may take several seconds to rebuild if the workspace grows to tens of thousands of objects. Optimization for massive corpora (100k+ objects) is ongoing.

## 6. Provider Extensibility
Integrating new AI models or execution environments currently requires minor code additions to the Rust project. A more flexible external plugin system is planned for future releases.
