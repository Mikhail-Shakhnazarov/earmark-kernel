# Local Orchestration Artifacts

This directory stores transient artifacts produced by the Codex → OpenCode executor bridge.

Tracked:
- `.gitkeep`
- this README

Ignored:
- manifests
- logs
- reports
- temporary executor output

The orchestrator owns task state, context packaging, branch policy, and global gates. OpenCode is only an executor. It receives a manifest, changes files on the current disposable branch, runs local gates, and stops.
