#!/usr/bin/env bash
set -euo pipefail

ROOT_DEFAULT="/home/m/GITHUB/earmark-workspace"
ROOT="${EARMARK_WORKSPACE:-$ROOT_DEFAULT}"
cd "$ROOT"

OPENCODE_MODEL="${OPENCODE_MODEL:-opencode/big-pickle}"
SKIP_GATES="${SKIP_GATES:-1}"
UNIQUE_BRANCH="${UNIQUE_BRANCH:-1}"

export OPENCODE_MODEL SKIP_GATES UNIQUE_BRANCH

if command -v opencode >/dev/null 2>&1; then
  export OPENCODE_CMD="${OPENCODE_CMD:-opencode}"
elif command -v opencode-cli >/dev/null 2>&1; then
  export OPENCODE_CMD="${OPENCODE_CMD:-opencode-cli}"
else
  echo "error: neither opencode nor opencode-cli found on PATH"
  exit 127
fi

echo "smoke: root=$ROOT"
echo "smoke: opencode_cmd=$OPENCODE_CMD"
echo "smoke: model=$OPENCODE_MODEL"

mkdir -p .orchestration/smoke

SMOKE_FILE="scratch/opencode_big_pickle_smoke.txt"
MANIFEST=".orchestration/smoke/opencode-big-pickle-smoke.md"

mkdir -p scratch

cat > "$MANIFEST" <<MANIFEST
# Dispatch Manifest

task_uuid: opencode-big-pickle-smoke
attempt_number: 1

## Objective

Prove that OpenCode running with the selected Big Pickle model can make a tiny bounded repository edit through the dispatch wrapper.

Create or replace the file:

- \`$SMOKE_FILE\`

The file must contain exactly:

\`\`\`text
opencode-big-pickle-smoke-ok
\`\`\`

Do not modify any other tracked source files.

## Target Files

- $SMOKE_FILE

## Local Gates

\`\`\`bash
test "\$(cat $SMOKE_FILE)" = "opencode-big-pickle-smoke-ok"
\`\`\`

## Executor Rules

- Implement only this manifest.
- Do not commit.
- Do not merge.
- Stop after the edit and local gate.
MANIFEST

scripts/dispatch-opencode.sh \
  --manifest "$MANIFEST" \
  --task opencode-big-pickle-smoke \
  --attempt 1

echo "smoke: dispatch completed"
echo "smoke: changed files:"
git status --short
echo "smoke: latest report:"
ls -t .orchestration/reports/opencode-big-pickle-smoke-* 2>/dev/null | head -1 || true
