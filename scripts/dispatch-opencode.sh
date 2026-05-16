#!/usr/bin/env bash
set -euo pipefail

ROOT_DEFAULT="/home/m/GITHUB/earmark-workspace"
ROOT="${EARMARK_WORKSPACE:-$ROOT_DEFAULT}"

ATTEMPT="1"
TASK_ID=""
MANIFEST_IN=""
MODEL="${OPENCODE_MODEL:-}"
AGENT="${OPENCODE_AGENT:-build}"
USE_ATTACH="${OPENCODE_ATTACH_URL:-}"
SKIP_GATES="${SKIP_GATES:-0}"
KEEP_BRANCH="${KEEP_BRANCH:-1}"

usage() {
  cat <<'USAGE'
Usage:
  scripts/dispatch-opencode.sh --manifest <path> [--task <id>] [--attempt <n>]
  scripts/dispatch-opencode.sh --task <engram-task-id> [--attempt <n>]

Environment:
  EARMARK_WORKSPACE      repo root; default /home/m/GITHUB/earmark-workspace
  OPENCODE_MODEL         optional provider/model override
  OPENCODE_AGENT         default build
  OPENCODE_ATTACH_URL    optional running opencode serve URL
  OPENCODE_CMD           path to the opencode binary; defaults to 'opencode'
  SKIP_GATES             set 1 to skip local/global gates during smoke tests
  KEEP_BRANCH            default 1; leaves branch for inspection

Notes:
  --manifest is the primary v0 path.
  --task is scaffolded and must be adapted to the local engram CLI.
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --manifest)
      MANIFEST_IN="${2:?missing manifest path}"
      shift 2
      ;;
    --task)
      TASK_ID="${2:?missing task id}"
      shift 2
      ;;
    --attempt)
      ATTEMPT="${2:?missing attempt number}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

cd "$ROOT"

mkdir -p .orchestration/manifests .orchestration/logs .orchestration/reports

timestamp="$(date -u +%Y%m%dT%H%M%SZ)"

if [[ -z "$TASK_ID" ]]; then
  if [[ -n "$MANIFEST_IN" ]]; then
    TASK_ID="$(basename "$MANIFEST_IN" | sed 's/\.[^.]*$//' | tr -c 'A-Za-z0-9_.-' '-')"
  else
    echo "error: provide --manifest or --task" >&2
    exit 2
  fi
fi

MANIFEST=".orchestration/manifests/${TASK_ID}-${ATTEMPT}-${timestamp}.md"
LOG=".orchestration/logs/${TASK_ID}-${ATTEMPT}-${timestamp}.log"
REPORT=".orchestration/reports/${TASK_ID}-${ATTEMPT}-${timestamp}.md"
BRANCH="orch/${TASK_ID}/${ATTEMPT}"

echo "dispatch-opencode: root=$ROOT" | tee "$LOG"
echo "dispatch-opencode: task=$TASK_ID attempt=$ATTEMPT" | tee -a "$LOG"

if [[ -n "$MANIFEST_IN" ]]; then
  cp "$MANIFEST_IN" "$MANIFEST"
else
  {
    echo "# Dispatch Manifest"
    echo
    echo "task_uuid: $TASK_ID"
    echo "attempt_number: $ATTEMPT"
    echo
    echo "## Objective"
    echo
    echo "TODO: adapt this section to the local engram CLI."
    echo
    echo "## Engram Discovery Output"
    echo
    echo '```text'
    command -v engram >/dev/null 2>&1 && engram --help || true
    echo '```'
    echo
    echo "## Executor Rules"
    echo
    echo "- Implement only this manifest."
    echo "- Do not query engram."
    echo "- Do not commit."
    echo "- Run local gates if listed."
  } > "$MANIFEST"
fi

OPENCODE_CMD="${OPENCODE_CMD:-opencode}"

if ! command -v "$OPENCODE_CMD" >/dev/null 2>&1; then
  echo "error: $OPENCODE_CMD not found on PATH" | tee -a "$LOG" >&2
  exit 127
fi

if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  echo "error: not inside a git work tree" | tee -a "$LOG" >&2
  exit 2
fi

if [[ -n "$(git status --porcelain)" ]]; then
  echo "error: working tree is dirty before dispatch; commit, stash, or clean manually" | tee -a "$LOG" >&2
  exit 3
fi

if git show-ref --verify --quiet "refs/heads/$BRANCH"; then
  echo "error: branch already exists: $BRANCH" | tee -a "$LOG" >&2
  exit 4
fi

git switch -c "$BRANCH" 2>&1 | tee -a "$LOG"

OPENCODE_ARGS=(run --agent "$AGENT" --file "$MANIFEST" --format json)

if [[ -n "$MODEL" ]]; then
  OPENCODE_ARGS+=(--model "$MODEL")
fi

if [[ -n "$USE_ATTACH" ]]; then
  OPENCODE_ARGS+=(--attach "$USE_ATTACH")
fi

OPENCODE_ARGS+=(
  "Execute the attached manifest using /execute-manifest. Do not commit. Stop after local gates and final report."
)

echo "dispatch-opencode: running $OPENCODE_CMD" | tee -a "$LOG"
set +e
HAS_JSON_ERROR=0
while IFS= read -r line; do
  echo "$line" | tee -a "$LOG"
  if echo "$line" | grep -q '"type":"error"'; then
    HAS_JSON_ERROR=1
  fi
done < <("$OPENCODE_CMD" "${OPENCODE_ARGS[@]}" 2>&1)
OPENCODE_STATUS=$?
# Wait for the process substitution to finish and get its exit code
wait $! || OPENCODE_STATUS=$?
set -e

{
  echo "# OpenCode Dispatch Report"
  echo
  echo "- task: \`$TASK_ID\`"
  echo "- attempt: \`$ATTEMPT\`"
  echo "- branch: \`$BRANCH\`"
  echo "- manifest: \`$MANIFEST\`"
  echo "- log: \`$LOG\`"
  echo "- opencode_status: \`$OPENCODE_STATUS\`"
  echo
  echo "## Changed Files"
  echo
  echo '```text'
  git status --short
  echo '```'
  echo
  echo "## Diff Stat"
  echo
  echo '```text'
  git diff --stat
  echo '```'
} > "$REPORT"

if [[ "$OPENCODE_STATUS" -ne 0 ]]; then
  echo "dispatch-opencode: opencode exited non-zero; see $LOG" | tee -a "$LOG"
  exit "$OPENCODE_STATUS"
fi

if [[ "$HAS_JSON_ERROR" -ne 0 ]]; then
  {
    echo
    echo "## JSON Error Events Detected"
    echo
    echo "Opencode output contained JSON error events."
  } >> "$REPORT"
  echo "dispatch-opencode: JSON error events detected in output; see $LOG" | tee -a "$LOG"
  exit 1
fi

if [[ "$SKIP_GATES" != "1" ]]; then
  echo "dispatch-opencode: running default gates" | tee -a "$LOG"

  set +e
  cargo test 2>&1 | tee -a "$LOG"
  CARGO_STATUS="${PIPESTATUS[0]}"
  set -e

  {
    echo
    echo "## Default Gate: cargo test"
    echo
    echo "- status: \`$CARGO_STATUS\`"
  } >> "$REPORT"

  if [[ "$CARGO_STATUS" -ne 0 ]]; then
    echo "dispatch-opencode: cargo test failed; see $LOG" | tee -a "$LOG"
    exit "$CARGO_STATUS"
  fi

  if command -v em >/dev/null 2>&1; then
    set +e
    em workflow list --json 2>&1 | tee -a "$LOG"
    EM_STATUS="${PIPESTATUS[0]}"
    set -e

    {
      echo
      echo "## Default Gate: em workflow list --json"
      echo
      echo "- status: \`$EM_STATUS\`"
    } >> "$REPORT"

    if [[ "$EM_STATUS" -ne 0 ]]; then
      echo "dispatch-opencode: em workflow list --json failed; see $LOG" | tee -a "$LOG"
      exit "$EM_STATUS"
    fi
  else
    echo "dispatch-opencode: em not found; skipped em workflow sanity gate" | tee -a "$LOG"
  fi
fi

echo "dispatch-opencode: complete" | tee -a "$LOG"
echo "dispatch-opencode: report=$REPORT" | tee -a "$LOG"
echo "dispatch-opencode: branch=$BRANCH" | tee -a "$LOG"

CHANGED_FILES=$(git status --short | wc -l)
if [[ "$CHANGED_FILES" -gt 0 ]]; then
  echo "dispatch-opencode: files changed: $CHANGED_FILES" | tee -a "$LOG"
  git status --short | head -10 | tee -a "$LOG"
  if [[ "$CHANGED_FILES" -gt 10 ]]; then
    echo "dispatch-opencode: ... and $((CHANGED_FILES - 10)) more" | tee -a "$LOG"
  fi
fi
