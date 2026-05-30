# Limitations

Earmark is currently published as a hardened kernel v0.1 baseline, not as a complete end-user runtime.

## 1. Kernel, Not Operator Shell

This repository contains the durable record, store, index, declarations, and governance-facing crates. It does not currently publish a supported CLI operator shell.

## 2. Local File Store

The canonical store is file-backed JSON under `.earmark/`. The store is intended to be inspectable and portable, but large-workspace performance is not yet optimized.

## 3. Derived Index

The SQLite index is derived from the canonical store. It may be deleted and rebuilt, but not every lookup surface is complete yet.

## 4. Runtime Records, Not Runtime Execution

Runs, packets, dispatches, handoffs, provider records, and worker records are represented as durable records. This branch does not provide a full orchestration executor.

## 5. Governance Baseline

Review, standing, checks, and acceptance state are modeled explicitly. Enforcement is partial and should be treated as a kernel baseline rather than a complete policy engine.

## 6. API Stability

The crates are pre-1.0. Record shapes, trait boundaries, and module names may still change before a public v1.
