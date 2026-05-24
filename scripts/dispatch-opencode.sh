#!/usr/bin/env bash
set -euo pipefail

ROOT_DEFAULT="/home/m/GITHUB/earmark-workspace"
ROOT="${EARMARK_WORKSPACE:-$ROOT_DEFAULT}"

ATTEMPT="1"
TASK_ID=""
MANIFEST_IN=""
MODEL="${OPENCODE_MODEL:-opencode/big-pickle}"
AGENT="${OPENCODE_AGENT:-build}"
USE_ATTACH="${OPENCODE_ATTACH_URL:-}"
SKIP_GATES="${SKIP_GATES:-1}"
KEEP_BRANCH="${KEEP_BRANCH:-1}"
UNIQUE_BRANCH="${UNIQUE_BRANCH:-1}"
OPENCODE_TIMEOUT_SEC="${OPENCODE_TIMEOUT_SEC:-900}"

usage() {
  cat <<'USAGE'
Usage:
  scripts/dispatch-opencode.sh --manifest <path> [--task <id>] [--attempt <n>]

Environment:
  EARMARK_WORKSPACE      repo root; default /home/m/GITHUB/earmark-workspace
  OPENCODE_MODEL         model override; default opencode/big-pickle
  OPENCODE_AGENT         default build
  OPENCODE_ATTACH_URL    optional running opencode serve URL
  OPENCODE_CMD           path to the opencode binary; defaults to 'opencode'
  OPENCODE_TIMEOUT_SEC   hard timeout for opencode execution (seconds, default 900)
  SKIP_GATES             set 1 (default) to skip global gates
  KEEP_BRANCH            default 1; leaves branch for inspection
  UNIQUE_BRANCH          default 1; appends timestamp if branch exists

Notes:
  - Big Pickle free model is the default dispatch profile.
  - Manifest-local gates are executed by the executor from the manifest.
  - Global gates here are optional and opt-in.
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
    echo "error: provide --manifest" >&2
    exit 2
  fi
fi

MANIFEST=".orchestration/manifests/${TASK_ID}-${ATTEMPT}-${timestamp}.md"
LOG=".orchestration/logs/${TASK_ID}-${ATTEMPT}-${timestamp}.log"
REPORT=".orchestration/reports/${TASK_ID}-${ATTEMPT}-${timestamp}.md"
BRANCH="orch/${TASK_ID}/${ATTEMPT}"

echo "dispatch-opencode: root=$ROOT" | tee "$LOG"
echo "dispatch-opencode: task=$TASK_ID attempt=$ATTEMPT" | tee -a "$LOG"

if [[ -n "$MODEL" ]]; then
  echo "dispatch-opencode: model=$MODEL" | tee -a "$LOG"
else
  echo "dispatch-opencode: model=opencode/big-pickle" | tee -a "$LOG"
fi

if [[ -z "$MANIFEST_IN" ]]; then
  echo "error: --manifest is required" | tee -a "$LOG" >&2
  exit 2
fi
cp "$MANIFEST_IN" "$MANIFEST"

resolve_opencode_cmd() {
  if [[ -n "${OPENCODE_CMD:-}" ]]; then
    echo "$OPENCODE_CMD"
    return 0
  fi

  if command -v opencode >/dev/null 2>&1; then
    echo "opencode"
    return 0
  fi

  if command -v opencode-cli >/dev/null 2>&1; then
    echo "opencode-cli"
    return 0
  fi

  return 1
}

OPENCODE_CMD="$(resolve_opencode_cmd || true)"

if [[ -z "$OPENCODE_CMD" ]]; then
  echo "error: neither opencode nor opencode-cli found on PATH; set OPENCODE_CMD explicitly" | tee -a "$LOG" >&2
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

BASE_BRANCH="$BRANCH"

if git show-ref --verify --quiet "refs/heads/$BRANCH"; then
  if [[ "$UNIQUE_BRANCH" == "1" ]]; then
    BRANCH="${BASE_BRANCH}-${timestamp}"
    echo "dispatch-opencode: branch exists; using unique branch=$BRANCH" | tee -a "$LOG"
  else
    echo "error: branch already exists: $BRANCH" | tee -a "$LOG" >&2
    exit 4
  fi
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
echo "dispatch-opencode: timeout_sec=$OPENCODE_TIMEOUT_SEC" | tee -a "$LOG"
TMP_OUTPUT="$(mktemp)"
WATCHDOG_SENTINEL="${TMP_OUTPUT}.watchdog"
QUOTA_SENTINEL="${TMP_OUTPUT}.quota"
set +e
"$OPENCODE_CMD" "${OPENCODE_ARGS[@]}" >"$TMP_OUTPUT" 2>&1 &
OPENCODE_PID=$!
(
  while kill -0 "$OPENCODE_PID" >/dev/null 2>&1; do
    if grep -q "FreeUsageLimitError" "$TMP_OUTPUT"; then
      echo "dispatch-opencode: provider quota error detected; terminating pid=$OPENCODE_PID" >>"$TMP_OUTPUT"
      kill "$OPENCODE_PID" >/dev/null 2>&1 || true
      sleep 2
      if kill -0 "$OPENCODE_PID" >/dev/null 2>&1; then
        kill -9 "$OPENCODE_PID" >/dev/null 2>&1 || true
      fi
      touch "$QUOTA_SENTINEL"
      break
    fi
    sleep 2
  done
) &
QUOTA_WATCH_PID=$!
(
  sleep "$OPENCODE_TIMEOUT_SEC"
  if kill -0 "$OPENCODE_PID" >/dev/null 2>&1; then
    echo "dispatch-opencode: watchdog timeout reached; terminating pid=$OPENCODE_PID" >>"$TMP_OUTPUT"
    kill "$OPENCODE_PID" >/dev/null 2>&1 || true
    sleep 3
    if kill -0 "$OPENCODE_PID" >/dev/null 2>&1; then
      kill -9 "$OPENCODE_PID" >/dev/null 2>&1 || true
    fi
    touch "$WATCHDOG_SENTINEL"
  fi
) &
WATCHDOG_PID=$!

wait "$OPENCODE_PID"
OPENCODE_STATUS=$?
kill "$WATCHDOG_PID" >/dev/null 2>&1 || true
kill "$QUOTA_WATCH_PID" >/dev/null 2>&1 || true
set -e

HAS_JSON_ERROR=0
WATCHDOG_TRIGGERED=0
while IFS= read -r line; do
  echo "$line" | tee -a "$LOG"
  if echo "$line" | grep -q '"type":"error"'; then
    HAS_JSON_ERROR=1
  fi
  if echo "$line" | grep -q "watchdog timeout reached"; then
    WATCHDOG_TRIGGERED=1
  fi
done < "$TMP_OUTPUT"

rm -f "$TMP_OUTPUT"

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
  if [[ "$WATCHDOG_TRIGGERED" -eq 1 || -f "$WATCHDOG_SENTINEL" ]]; then
    OPENCODE_STATUS=124
    echo "dispatch-opencode: opencode watchdog timeout after ${OPENCODE_TIMEOUT_SEC}s; see $LOG" | tee -a "$LOG"
  elif [[ -f "$QUOTA_SENTINEL" ]]; then
    OPENCODE_STATUS=75
    echo "dispatch-opencode: provider quota/rate-limit error; see $LOG" | tee -a "$LOG"
  fi
  echo "dispatch-opencode: opencode exited non-zero; see $LOG" | tee -a "$LOG"
  rm -f "$WATCHDOG_SENTINEL"
  rm -f "$QUOTA_SENTINEL"
  exit "$OPENCODE_STATUS"
fi
rm -f "$WATCHDOG_SENTINEL"
rm -f "$QUOTA_SENTINEL"

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
  echo "dispatch-opencode: running global gates" | tee -a "$LOG"

  set +e
  if command -v nix-shell >/dev/null 2>&1; then
    nix-shell -p pkg-config openssl --run 'cargo test --workspace' 2>&1 | tee -a "$LOG"
  else
    cargo test --workspace 2>&1 | tee -a "$LOG"
  fi
  CARGO_STATUS="${PIPESTATUS[0]}"
  set -e

  {
    echo
    echo "## Global Gate: cargo test --workspace"
    echo
    echo "- status: \`$CARGO_STATUS\`"
  } >> "$REPORT"

  if [[ "$CARGO_STATUS" -ne 0 ]]; then
    echo "dispatch-opencode: global gate failed; see $LOG" | tee -a "$LOG"
    exit "$CARGO_STATUS"
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
