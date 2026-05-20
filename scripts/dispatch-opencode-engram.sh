#!/usr/bin/env bash
set -euo pipefail

ROOT_DEFAULT="/home/m/GITHUB/earmark-workspace"
ROOT="${EARMARK_WORKSPACE:-$ROOT_DEFAULT}"
ENGRAM_BIN="${ENGRAM_BIN:-/home/m/GITHUB/engram/engram/target/debug/engram}"
DISPATCH_BIN="${DISPATCH_BIN:-$ROOT/scripts/dispatch-opencode.sh}"
ENGRAM_AGENT="${ENGRAM_AGENT:-codex}"
ATTEMPT="1"
TASK_ID=""
TITLE=""
OBJECTIVE=""
CONTEXT_TEXT=""
MANIFEST_IN=""
SKIP_ENGRAM="${SKIP_ENGRAM:-0}"

usage() {
  cat <<'USAGE'
Usage:
  scripts/dispatch-opencode-engram.sh --task-id <uuid> --objective <text> [--attempt <n>]
  scripts/dispatch-opencode-engram.sh --title <task-title> --objective <text> [--context <text>] [--attempt <n>]
  scripts/dispatch-opencode-engram.sh --manifest <path> --task-id <uuid> [--attempt <n>]
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --task-id) TASK_ID="${2:?missing task id}"; shift 2 ;;
    --title) TITLE="${2:?missing title}"; shift 2 ;;
    --objective) OBJECTIVE="${2:?missing objective}"; shift 2 ;;
    --context) CONTEXT_TEXT="${2:?missing context text}"; shift 2 ;;
    --manifest) MANIFEST_IN="${2:?missing manifest path}"; shift 2 ;;
    --attempt) ATTEMPT="${2:?missing attempt}"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) echo "unknown argument: $1" >&2; usage >&2; exit 2 ;;
  esac
done

if [[ -z "$TASK_ID" && -z "$TITLE" ]]; then
  echo "error: provide either --task-id or --title" >&2
  exit 2
fi
if [[ -z "$MANIFEST_IN" && -z "$OBJECTIVE" ]]; then
  echo "error: provide --objective unless using --manifest" >&2
  exit 2
fi

cd "$ROOT"
mkdir -p .orchestration/manifests

ts="$(date -u +%Y%m%dT%H%M%SZ)"
run_tag="engram-opencode-$ts"

if [[ "$SKIP_ENGRAM" != "1" ]]; then
  if [[ ! -x "$ENGRAM_BIN" ]] && ! command -v engram >/dev/null 2>&1; then
    echo "error: engram not found" >&2
    exit 127
  fi
  [[ -x "$ENGRAM_BIN" ]] || ENGRAM_BIN="engram"
fi

if [[ -z "$TASK_ID" ]]; then
  task_json="$($ENGRAM_BIN task create --title "$TITLE" --description "$OBJECTIVE" --priority high --agent "$ENGRAM_AGENT" --output json)"
  TASK_ID="$(python3 -c 'import json,sys;print(json.loads(sys.stdin.read())["id"])' <<< "$task_json")"
fi

if [[ "$SKIP_ENGRAM" != "1" && -n "$CONTEXT_TEXT" ]]; then
  ctx_out="$($ENGRAM_BIN context create --title "Dispatch context: $run_tag" --source "dispatch-opencode-engram" --content "$CONTEXT_TEXT" --source-id "$ROOT" --agent "$ENGRAM_AGENT" --tags "orchestration,dispatch,opencode,earmark" || true)"
  ctx_id="$(echo "$ctx_out" | sed -n "s/.*Context '\([0-9a-f-]\{36\}\)'.*/\1/p" | head -n1)"
  if [[ -n "$ctx_id" ]]; then
    $ENGRAM_BIN relationship create --source-id "$TASK_ID" --source-type task --target-id "$ctx_id" --target-type context --relationship-type informed_by --agent "$ENGRAM_AGENT" >/dev/null 2>&1 || true
  fi
fi

if [[ "$SKIP_ENGRAM" != "1" ]]; then
  pre_out="$($ENGRAM_BIN reasoning create --task-id "$TASK_ID" --title "Dispatch start: $run_tag" --content "Starting opencode dispatch attempt $ATTEMPT. Objective: $OBJECTIVE" --confidence 0.7 --agent "$ENGRAM_AGENT" --tags "orchestration,dispatch,start" || true)"
  pre_id="$(echo "$pre_out" | sed -n "s/.*Reasoning '\([0-9a-f-]\{36\}\)'.*/\1/p" | head -n1)"
  if [[ -n "$pre_id" ]]; then
    $ENGRAM_BIN relationship create --source-id "$TASK_ID" --source-type task --target-id "$pre_id" --target-type reasoning --relationship-type justified_by --agent "$ENGRAM_AGENT" >/dev/null 2>&1 || true
  fi
fi

if [[ -z "$MANIFEST_IN" ]]; then
  MANIFEST_IN=".orchestration/manifests/${TASK_ID}-${ATTEMPT}-${ts}-engram.md"
  cat > "$MANIFEST_IN" <<MANIFEST
# Dispatch Manifest

task_uuid: $TASK_ID
attempt_number: $ATTEMPT

## Objective

$OBJECTIVE

## Constraints

- Work only in this repository.
- Do not commit changes.
- Produce minimal, reviewable diffs.
- Run local gates.

## Local Gates

- cargo test
- em workflow list --json
MANIFEST
fi

set +e
"$DISPATCH_BIN" --manifest "$MANIFEST_IN" --task "$TASK_ID" --attempt "$ATTEMPT"
status=$?
set -e

if [[ "$SKIP_ENGRAM" != "1" ]]; then
  title="Dispatch failure: $run_tag"; conf="0.6"
  if [[ "$status" -eq 0 ]]; then title="Dispatch success: $run_tag"; conf="0.85"; fi
  post_out="$($ENGRAM_BIN reasoning create --task-id "$TASK_ID" --title "$title" --content "Dispatch finished with status $status. Manifest: $MANIFEST_IN" --confidence "$conf" --agent "$ENGRAM_AGENT" --tags "orchestration,dispatch,outcome" || true)"
  post_id="$(echo "$post_out" | sed -n "s/.*Reasoning '\([0-9a-f-]\{36\}\)'.*/\1/p" | head -n1)"
  if [[ -n "$post_id" ]]; then
    $ENGRAM_BIN relationship create --source-id "$TASK_ID" --source-type task --target-id "$post_id" --target-type reasoning --relationship-type justified_by --agent "$ENGRAM_AGENT" >/dev/null 2>&1 || true
  fi
fi

echo "dispatch-opencode-engram: task_id=$TASK_ID"
echo "dispatch-opencode-engram: manifest=$MANIFEST_IN"
echo "dispatch-opencode-engram: dispatch_status=$status"
exit "$status"
