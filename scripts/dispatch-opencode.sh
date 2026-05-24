#!/usr/bin/env bash
set -euo pipefail

CALLER_PWD="$(pwd)"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DEFAULT="$(cd "$SCRIPT_DIR/.." && pwd)"
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
STRICT_CLEAN="${STRICT_CLEAN:-0}"
REQUIRE_EARMARK="${REQUIRE_EARMARK:-1}"
OPENCODE_TIMEOUT_SEC="${OPENCODE_TIMEOUT_SEC:-900}"
OPENCODE_LIVE_LOG="${OPENCODE_LIVE_LOG:-1}"
OPENCODE_QUOTA_PATTERN="${OPENCODE_QUOTA_PATTERN:-FreeUsageLimitError|free usage limit|usage limit|rate limit|quota|429}"

usage() {
  cat <<'USAGE'
Usage:
  scripts/dispatch-opencode.sh --manifest <path> [--task <id>] [--attempt <n>]

Environment:
  EARMARK_WORKSPACE      repo root; defaults to parent of this script directory
  EARMARK_CMD            Earmark CLI command; default 'cargo run --bin earmark-cli --'
  REQUIRE_EARMARK        1 (default) requires Earmark ingest/capture setup before dispatch; 0 logs and continues
  OPENCODE_MODEL         model override; default opencode/big-pickle
  OPENCODE_AGENT         default build
  OPENCODE_ATTACH_URL    optional running opencode serve URL
  OPENCODE_CMD           path to the opencode binary; defaults to 'opencode', then 'opencode-cli'
  OPENCODE_TIMEOUT_SEC   hard timeout for opencode execution (seconds, default 900)
  OPENCODE_LIVE_LOG      1 (default) streams opencode output while it runs
  OPENCODE_QUOTA_PATTERN regex used to detect quota/rate-limit failures in output
  SKIP_GATES             set 1 (default) to skip global gates
  STRICT_CLEAN           set 1 to refuse a dirty working tree before dispatch
  KEEP_BRANCH            1 (default) leaves branch for inspection; 0 deletes clean successful branch
  UNIQUE_BRANCH          1 (default) appends timestamp if branch exists

Notes:
  - Big Pickle free model is the default dispatch profile.
  - Manifest-local gates are executed by the executor from the manifest.
  - Global gates here are optional and opt-in.
  - Failure, timeout, and quota outcomes are still written to a dispatch report.
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

if [[ -z "$MANIFEST_IN" ]]; then
  echo "error: --manifest is required" >&2
  usage >&2
  exit 2
fi

if [[ "$MANIFEST_IN" = /* ]]; then
  MANIFEST_SRC="$MANIFEST_IN"
else
  MANIFEST_SRC="$CALLER_PWD/$MANIFEST_IN"
fi

if [[ ! -f "$MANIFEST_SRC" ]]; then
  echo "error: manifest not found: $MANIFEST_SRC" >&2
  exit 2
fi

cd "$ROOT"

mkdir -p .orchestration/manifests .orchestration/logs .orchestration/reports

timestamp="$(date -u +%Y%m%dT%H%M%SZ)"

if [[ -z "$TASK_ID" ]]; then
  TASK_ID="$(basename "$MANIFEST_SRC" | sed 's/\.[^.]*$//' | tr -c 'A-Za-z0-9_.-' '-')"
fi

MANIFEST=".orchestration/manifests/${TASK_ID}-${ATTEMPT}-${timestamp}.md"
LOG=".orchestration/logs/${TASK_ID}-${ATTEMPT}-${timestamp}.log"
REPORT=".orchestration/reports/${TASK_ID}-${ATTEMPT}-${timestamp}.md"
BRANCH="orch/${TASK_ID}/${ATTEMPT}"
START_BRANCH="$(git branch --show-current 2>/dev/null || true)"

log() {
  echo "dispatch-opencode: $*" | tee -a "$LOG"
}

warn() {
  log "warning: $*"
}

cp "$MANIFEST_SRC" "$MANIFEST"

log "root=$ROOT"
log "task=$TASK_ID attempt=$ATTEMPT"
log "manifest_src=$MANIFEST_SRC"
log "manifest_copy=$MANIFEST"
log "model=${MODEL:-opencode/big-pickle}"
log "agent=$AGENT"

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
  log "error: neither opencode nor opencode-cli found on PATH; set OPENCODE_CMD explicitly"
  exit 127
fi

if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  log "error: not inside a git work tree"
  exit 2
fi

if [[ -n "$(git status --porcelain)" ]]; then
  if [[ "$STRICT_CLEAN" == "1" ]]; then
    log "error: working tree is dirty before dispatch; commit, stash, or clean manually, or set STRICT_CLEAN=0"
    git status --short | tee -a "$LOG"
    exit 3
  fi
  warn "working tree is dirty before dispatch; changed files will be carried into the orchestration branch"
  git status --short | head -20 | tee -a "$LOG"
fi

# Parse the Earmark command into a simple argv. This intentionally supports the default
# "cargo run --bin earmark-cli --" form and simple OP-provided overrides.
if [[ -n "${EARMARK_CMD:-}" ]]; then
  # shellcheck disable=SC2206
  EARMARK_CMD_ARR=($EARMARK_CMD)
else
  EARMARK_CMD_ARR=(cargo run --bin earmark-cli --)
fi

run_earmark() {
  "${EARMARK_CMD_ARR[@]}" "$@"
}

DISPATCH_ID=""

log "registering dispatch in Earmark"
set +e
DISPATCH_JSON="$(run_earmark orchestration ingest-manifest "$MANIFEST_SRC" --task-id "$TASK_ID" --attempt "$ATTEMPT" 2>&1)"
DISPATCH_STATUS=$?
set -e

if [[ "$DISPATCH_STATUS" -ne 0 ]]; then
  log "error: Earmark ingest-manifest failed with status=$DISPATCH_STATUS"
  printf '%s\n' "$DISPATCH_JSON" | tee -a "$LOG"
  if [[ "$REQUIRE_EARMARK" == "1" ]]; then
    exit "$DISPATCH_STATUS"
  fi
  warn "continuing without Earmark dispatch_id because REQUIRE_EARMARK=0"
else
  DISPATCH_ID=$(echo "$DISPATCH_JSON" | grep -oE '"object_id": *"[^"]+"' | cut -d'"' -f4 | head -n 1 || true)
  if [[ -z "$DISPATCH_ID" ]]; then
    log "error: failed to capture dispatch_id from: $DISPATCH_JSON"
    if [[ "$REQUIRE_EARMARK" == "1" ]]; then
      exit 1
    fi
    warn "continuing without Earmark dispatch_id because REQUIRE_EARMARK=0"
  fi
fi

capture_git_phase() {
  local phase="$1"
  if [[ -z "$DISPATCH_ID" ]]; then
    return 0
  fi

  log "capturing git state phase=$phase dispatch_id=$DISPATCH_ID"
  set +e
  run_earmark orchestration capture-git --task-id "$TASK_ID" --dispatch-id "$DISPATCH_ID" --phase "$phase" 2>&1 | tee -a "$LOG"
  local status="${PIPESTATUS[0]}"
  set -e

  if [[ "$status" -ne 0 ]]; then
    warn "Earmark capture-git failed for phase=$phase status=$status"
    if [[ "$REQUIRE_EARMARK" == "1" && "$phase" == "pre-dispatch" ]]; then
      exit "$status"
    fi
  fi
}

ingest_report() {
  if [[ -z "$DISPATCH_ID" ]]; then
    return 0
  fi

  log "ingesting executor report dispatch_id=$DISPATCH_ID"
  set +e
  run_earmark orchestration ingest-report "$REPORT" --task-id "$TASK_ID" --manifest "$DISPATCH_ID" --attempt "$ATTEMPT" 2>&1 | tee -a "$LOG"
  local status="${PIPESTATUS[0]}"
  set -e

  if [[ "$status" -ne 0 ]]; then
    warn "Earmark ingest-report failed status=$status"
  fi
}

capture_git_phase "pre-dispatch"

BASE_BRANCH="$BRANCH"

if git show-ref --verify --quiet "refs/heads/$BRANCH"; then
  if [[ "$UNIQUE_BRANCH" == "1" ]]; then
    BRANCH="${BASE_BRANCH}-${timestamp}"
    log "branch exists; using unique branch=$BRANCH"
  else
    log "error: branch already exists: $BRANCH"
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

log "running $OPENCODE_CMD"
log "timeout_sec=$OPENCODE_TIMEOUT_SEC"
log "live_log=$OPENCODE_LIVE_LOG"

TMP_OUTPUT="$(mktemp)"
REPORT_CONTENT_TMP="$(mktemp)"
JSON_STATUS_TMP="$(mktemp)"
WATCHDOG_SENTINEL="${TMP_OUTPUT}.watchdog"
QUOTA_SENTINEL="${TMP_OUTPUT}.quota"
: > "$TMP_OUTPUT"

TAIL_PID=""
if [[ "$OPENCODE_LIVE_LOG" == "1" ]]; then
  (
    tail -n +1 -f "$TMP_OUTPUT" 2>/dev/null | while IFS= read -r line; do
      printf '%s\n' "$line" | tee -a "$LOG"
    done
  ) &
  TAIL_PID=$!
fi

set +e
"$OPENCODE_CMD" "${OPENCODE_ARGS[@]}" >"$TMP_OUTPUT" 2>&1 &
OPENCODE_PID=$!
(
  while kill -0 "$OPENCODE_PID" >/dev/null 2>&1; do
    if grep -Eiq "$OPENCODE_QUOTA_PATTERN" "$TMP_OUTPUT"; then
      echo "dispatch-opencode: provider quota/rate-limit pattern detected; terminating pid=$OPENCODE_PID" >>"$TMP_OUTPUT"
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
if [[ -n "$TAIL_PID" ]]; then
  sleep 0.2
  kill "$TAIL_PID" >/dev/null 2>&1 || true
fi
set -e

if [[ "$OPENCODE_LIVE_LOG" != "1" ]]; then
  cat "$TMP_OUTPUT" | tee -a "$LOG"
fi

NORMALIZED_STATUS="$OPENCODE_STATUS"
OUTCOME="success"
if [[ "$OPENCODE_STATUS" -ne 0 ]]; then
  OUTCOME="opencode_failed"
  if [[ -f "$WATCHDOG_SENTINEL" ]] || grep -q "watchdog timeout reached" "$TMP_OUTPUT"; then
    NORMALIZED_STATUS=124
    OUTCOME="timeout"
  elif [[ -f "$QUOTA_SENTINEL" ]]; then
    NORMALIZED_STATUS=75
    OUTCOME="quota_or_rate_limit"
  fi
fi

HAS_JSON_ERROR=0
if command -v python3 >/dev/null 2>&1; then
  python3 - "$TMP_OUTPUT" "$REPORT_CONTENT_TMP" "$JSON_STATUS_TMP" <<'PY'
import json
import sys
from pathlib import Path

src = Path(sys.argv[1])
report = Path(sys.argv[2])
status = Path(sys.argv[3])

has_json_error = False
text_chunks = []

for raw in src.read_text(errors="replace").splitlines():
    try:
        event = json.loads(raw)
    except Exception:
        continue
    if event.get("type") == "error":
        has_json_error = True
    if event.get("type") == "text":
        text = event.get("text")
        if isinstance(text, str) and text:
            text_chunks.append(text)

report.write_text("\n".join(text_chunks), encoding="utf-8")
status.write_text("1" if has_json_error else "0", encoding="utf-8")
PY
  HAS_JSON_ERROR="$(cat "$JSON_STATUS_TMP")"
else
  while IFS= read -r line; do
    if echo "$line" | grep -q '"type":"error"'; then
      HAS_JSON_ERROR=1
    fi
    if echo "$line" | grep -q '"type":"text"'; then
      echo "$line" | grep -oP '"text":"\K.*(?=","time")' | sed 's/\\n/\n/g' | sed 's/\\"/"/g' >> "$REPORT_CONTENT_TMP" || true
    fi
  done < "$TMP_OUTPUT"
fi

if [[ "$HAS_JSON_ERROR" -ne 0 && "$OUTCOME" == "success" ]]; then
  OUTCOME="json_error_event"
  NORMALIZED_STATUS=1
fi

write_report() {
  local gate_status="${1:-}"
  {
    echo "# OpenCode Dispatch Report"
    echo
    if [[ -s "$REPORT_CONTENT_TMP" ]]; then
      cat "$REPORT_CONTENT_TMP"
      echo
    else
      echo "No structured text events were extracted from OpenCode output."
      echo
      echo "## Raw OpenCode Output Tail"
      echo
      echo '```text'
      tail -200 "$TMP_OUTPUT" || true
      echo '```'
      echo
    fi

    echo "## Dispatch State"
    echo
    echo "- task: \`$TASK_ID\`"
    echo "- attempt: \`$ATTEMPT\`"
    echo "- branch: \`$BRANCH\`"
    echo "- manifest: \`$MANIFEST\`"
    echo "- source_manifest: \`$MANIFEST_SRC\`"
    echo "- log: \`$LOG\`"
    echo "- dispatch_id: \`${DISPATCH_ID:-none}\`"
    echo "- opencode_status: \`$OPENCODE_STATUS\`"
    echo "- normalized_status: \`$NORMALIZED_STATUS\`"
    echo "- outcome: \`$OUTCOME\`"
    echo "- json_error_events: \`$HAS_JSON_ERROR\`"
    if [[ -n "$gate_status" ]]; then
      echo "- global_gate_status: \`$gate_status\`"
    fi
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
}

finish_dispatch() {
  local exit_status="$1"
  write_report "${2:-}"
  capture_git_phase "post-dispatch"
  ingest_report
  rm -f "$WATCHDOG_SENTINEL" "$QUOTA_SENTINEL" "$REPORT_CONTENT_TMP" "$JSON_STATUS_TMP" "$TMP_OUTPUT"

  if [[ "$exit_status" -eq 0 && "$KEEP_BRANCH" != "1" && -n "$START_BRANCH" ]]; then
    if [[ -z "$(git status --porcelain)" ]]; then
      git switch "$START_BRANCH" >/dev/null 2>&1 || true
      git branch -D "$BRANCH" >/dev/null 2>&1 || true
      log "deleted clean successful branch because KEEP_BRANCH=$KEEP_BRANCH"
    else
      warn "KEEP_BRANCH=$KEEP_BRANCH requested, but branch has changes; leaving branch for inspection"
    fi
  fi

  log "complete outcome=$OUTCOME report=$REPORT branch=$BRANCH dispatch_id=${DISPATCH_ID:-none}"
  exit "$exit_status"
}

if [[ "$NORMALIZED_STATUS" -ne 0 ]]; then
  case "$OUTCOME" in
    timeout)
      log "opencode watchdog timeout after ${OPENCODE_TIMEOUT_SEC}s; see $LOG"
      ;;
    quota_or_rate_limit)
      log "provider quota/rate-limit error; see $LOG"
      ;;
    json_error_event)
      log "JSON error events detected in output; see $LOG"
      ;;
    *)
      log "opencode exited non-zero status=$OPENCODE_STATUS; see $LOG"
      ;;
  esac
  finish_dispatch "$NORMALIZED_STATUS"
fi

if [[ "$SKIP_GATES" != "1" ]]; then
  log "running global gates"

  set +e
  if command -v nix-shell >/dev/null 2>&1; then
    nix-shell -p pkg-config openssl --run 'cargo test --workspace' 2>&1 | tee -a "$LOG"
  else
    cargo test --workspace 2>&1 | tee -a "$LOG"
  fi
  CARGO_STATUS="${PIPESTATUS[0]}"
  set -e

  if [[ "$CARGO_STATUS" -ne 0 ]]; then
    OUTCOME="global_gate_failed"
    NORMALIZED_STATUS="$CARGO_STATUS"
    log "global gate failed; see $LOG"
    finish_dispatch "$CARGO_STATUS" "$CARGO_STATUS"
  fi

  write_report "$CARGO_STATUS"
fi

finish_dispatch 0
