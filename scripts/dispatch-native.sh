#!/usr/bin/env bash
set -euo pipefail

# Status: internal experimental dogfooding helper.
# This script is not a stable public interface. Prefer the `em orchestration ...`
# CLI commands and `scripts/dispatch-opencode.sh` for the current supported local path.

ROOT_DEFAULT="/home/m/GITHUB/earmark-workspace"
ROOT="${EARMARK_WORKSPACE:-$ROOT_DEFAULT}"

TITLE=""
OBJECTIVE=""
ATTEMPT="1"
MODEL="${OPENCODE_MODEL:-}"
SKIP_GATES="${SKIP_GATES:-1}"

usage() {
  cat <<'USAGE'
Usage:
  scripts/dispatch-native.sh --title <task-title> --objective <text> [--attempt <n>]

Environment:
  EARMARK_WORKSPACE      repo root; default /home/m/GITHUB/earmark-workspace
  OPENCODE_MODEL         model override; default opencode/big-pickle
  SKIP_GATES             set 1 (default) to skip global gates in dispatch-opencode
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --title)
      TITLE="${2:?missing title}"
      shift 2
      ;;
    --objective)
      OBJECTIVE="${2:?missing objective}"
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

if [[ -z "$TITLE" || -z "$OBJECTIVE" ]]; then
  echo "error: both --title and --objective must be provided" >&2
  usage >&2
  exit 2
fi

cd "$ROOT"

if [[ -z "$MODEL" ]]; then
  MODEL="opencode/big-pickle"
fi
export OPENCODE_MODEL="$MODEL"
export SKIP_GATES

resolve_em_cmd() {
  if [[ -x "./target/debug/earmark-cli" ]]; then
    echo "./target/debug/earmark-cli"
    return 0
  fi
  if [[ -x "./target/release/earmark-cli" ]]; then
    echo "./target/release/earmark-cli"
    return 0
  fi
  if command -v em >/dev/null 2>&1; then
    echo "em"
    return 0
  fi
  if command -v earmark-cli >/dev/null 2>&1; then
    echo "earmark-cli"
    return 0
  fi
  return 1
}

EM_CMD="$(resolve_em_cmd || true)"
if [[ -z "$EM_CMD" ]]; then
  echo "error: Earmark CLI binary not found. Build the workspace first." >&2
  exit 127
fi

echo "dispatch-native: using Earmark CLI: $EM_CMD"

# 1. Initialize orchestration classes if needed
"$EM_CMD" orchestration init-example >/dev/null 2>&1 || true

# 2. Deposit work_item
echo "dispatch-native: depositing work_item..."
WI_PAYLOAD="$(printf '{"goal":"%s","status":"proposed","priority":"high"}' "$OBJECTIVE")"
WI_JSON="$("$EM_CMD" --json deposit --class work_item --title "$TITLE" --json-payload "$WI_PAYLOAD")"
WI_ID="$(python3 -c 'import json,sys;print(json.loads(sys.stdin.read())["data"]["object_id"])' <<< "$WI_JSON")"
WI_VER="$(python3 -c 'import json,sys;print(json.loads(sys.stdin.read())["data"]["version_id"])' <<< "$WI_JSON")"
echo "dispatch-native: work_item ID=$WI_ID Version=$WI_VER"

# 3. Deposit context_packet
echo "dispatch-native: depositing context_packet..."
CP_PAYLOAD="$(printf '{"work_item_id":"%s","instructions":"%s"}' "$WI_ID" "$OBJECTIVE")"
CP_JSON="$("$EM_CMD" --json deposit --class context_packet --title "Context Packet for $TITLE" --json-payload "$CP_PAYLOAD")"
CP_ID="$(python3 -c 'import json,sys;print(json.loads(sys.stdin.read())["data"]["object_id"])' <<< "$CP_JSON")"
CP_VER="$(python3 -c 'import json,sys;print(json.loads(sys.stdin.read())["data"]["version_id"])' <<< "$CP_JSON")"
echo "dispatch-native: context_packet ID=$CP_ID Version=$CP_VER"

# 4. Link work_item -> context_packet (has_context)
echo "dispatch-native: linking work_item to context_packet..."
REL_PAYLOAD="$(cat <<EOF
{
  "source": {
    "id": "$WI_ID",
    "version_id": "$WI_VER",
    "kind": "object",
    "class": "work_item"
  },
  "target": {
    "id": "$CP_ID",
    "version_id": "$CP_VER",
    "kind": "object",
    "class": "context_packet"
  },
  "relation_type": "has_context",
  "qualifiers": {},
  "scope": null
}
EOF
)"
"$EM_CMD" deposit --class any --kind relation \
  --header relation_auth_endpoint=source \
  --header relation_auth_class=work_item \
  --header relation_auth_direction=outgoing \
  --header relation_auth_authority=source \
  --json-payload "$REL_PAYLOAD" >/dev/null

# 5. Deposit dispatch
echo "dispatch-native: depositing dispatch..."
DP_PAYLOAD="$(printf '{"work_item_id":"%s","executor":"opencode","attempt":%d}' "$WI_ID" "$ATTEMPT")"
DP_JSON="$("$EM_CMD" --json deposit --class dispatch --title "Dispatch Attempt $ATTEMPT" --json-payload "$DP_PAYLOAD")"
DP_ID="$(python3 -c 'import json,sys;print(json.loads(sys.stdin.read())["data"]["object_id"])' <<< "$DP_JSON")"
DP_VER="$(python3 -c 'import json,sys;print(json.loads(sys.stdin.read())["data"]["version_id"])' <<< "$DP_JSON")"
echo "dispatch-native: dispatch ID=$DP_ID Version=$DP_VER"

# 6. Link work_item -> dispatch (has_dispatch)
echo "dispatch-native: linking work_item to dispatch..."
REL_PAYLOAD="$(cat <<EOF
{
  "source": {
    "id": "$WI_ID",
    "version_id": "$WI_VER",
    "kind": "object",
    "class": "work_item"
  },
  "target": {
    "id": "$DP_ID",
    "version_id": "$DP_VER",
    "kind": "object",
    "class": "dispatch"
  },
  "relation_type": "has_dispatch",
  "qualifiers": {},
  "scope": null
}
EOF
)"
"$EM_CMD" deposit --class any --kind relation \
  --header relation_auth_endpoint=source \
  --header relation_auth_class=work_item \
  --header relation_auth_direction=outgoing \
  --header relation_auth_authority=source \
  --json-payload "$REL_PAYLOAD" >/dev/null

# 7. Deposit trace_event (started)
echo "dispatch-native: depositing trace_event (started)..."
TE_PAYLOAD="$(printf '{"work_item_id":"%s","event_type":"started","message":"Execution started"}' "$WI_ID")"
TE_JSON="$("$EM_CMD" --json deposit --class trace_event --title "Trace Event: Started" --json-payload "$TE_PAYLOAD")"
TE_ID="$(python3 -c 'import json,sys;print(json.loads(sys.stdin.read())["data"]["object_id"])' <<< "$TE_JSON")"
TE_VER="$(python3 -c 'import json,sys;print(json.loads(sys.stdin.read())["data"]["version_id"])' <<< "$TE_JSON")"

# 8. Link dispatch -> trace_event (emitted_trace)
echo "dispatch-native: linking dispatch to trace_event..."
REL_PAYLOAD="$(cat <<EOF
{
  "source": {
    "id": "$DP_ID",
    "version_id": "$DP_VER",
    "kind": "object",
    "class": "dispatch"
  },
  "target": {
    "id": "$TE_ID",
    "version_id": "$TE_VER",
    "kind": "object",
    "class": "trace_event"
  },
  "relation_type": "emitted_trace",
  "qualifiers": {},
  "scope": null
}
EOF
)"
"$EM_CMD" deposit --class any --kind relation \
  --header relation_auth_endpoint=source \
  --header relation_auth_class=dispatch \
  --header relation_auth_direction=outgoing \
  --header relation_auth_authority=source \
  --json-payload "$REL_PAYLOAD" >/dev/null

# 9. Build OpenCode Manifest File
mkdir -p .orchestration/manifests
ts="$(date -u +%Y%m%dT%H%M%SZ)"
MANIFEST_FILE=".orchestration/manifests/${WI_ID}-${ATTEMPT}-${ts}-native.md"

cat > "$MANIFEST_FILE" <<MANIFEST
# Dispatch Manifest

task_uuid: $WI_ID
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
MANIFEST

echo "dispatch-native: compiled manifest at $MANIFEST_FILE"

# 10. Execute dispatch-opencode wrapper
echo "dispatch-native: executing OpenCode dispatch..."
set +e
scripts/dispatch-opencode.sh --manifest "$MANIFEST_FILE" --task "$WI_ID" --attempt "$ATTEMPT"
status=$?
set -e

# 11. Record outcome
if [[ "$status" -eq 0 ]]; then
  echo "dispatch-native: execution succeeded. Depositing evidence and closure..."
  
  # Deposit evidence
  EV_PAYLOAD="$(printf '{"work_item_id":"%s","evidence_type":"git_diff","description":"Git changes produced by executor"}' "$WI_ID")"
  EV_JSON="$("$EM_CMD" --json deposit --class evidence --title "Evidence: Git Diff" --json-payload "$EV_PAYLOAD")"
  EV_ID="$(python3 -c 'import json,sys;print(json.loads(sys.stdin.read())["data"]["object_id"])' <<< "$EV_JSON")"
  EV_VER="$(python3 -c 'import json,sys;print(json.loads(sys.stdin.read())["data"]["version_id"])' <<< "$EV_JSON")"

  # Link work_item -> evidence (has_evidence)
  REL_PAYLOAD="$(cat <<EOF
{
  "source": {
    "id": "$WI_ID",
    "version_id": "$WI_VER",
    "kind": "object",
    "class": "work_item"
  },
  "target": {
    "id": "$EV_ID",
    "version_id": "$EV_VER",
    "kind": "object",
    "class": "evidence"
  },
  "relation_type": "has_evidence",
  "qualifiers": {},
  "scope": null
}
EOF
)"
  "$EM_CMD" deposit --class any --kind relation \
    --header relation_auth_endpoint=source \
    --header relation_auth_class=work_item \
    --header relation_auth_direction=outgoing \
    --header relation_auth_authority=source \
    --json-payload "$REL_PAYLOAD" >/dev/null

  # Deposit closure
  CL_PAYLOAD="$(printf '{"work_item_id":"%s","disposition":"completed","summary":"Task executed successfully via native dispatch"}' "$WI_ID")"
  CL_JSON="$("$EM_CMD" --json deposit --class closure --title "Closure: Completed" --json-payload "$CL_PAYLOAD")"
  CL_ID="$(python3 -c 'import json,sys;print(json.loads(sys.stdin.read())["data"]["object_id"])' <<< "$CL_JSON")"
  CL_VER="$(python3 -c 'import json,sys;print(json.loads(sys.stdin.read())["data"]["version_id"])' <<< "$CL_JSON")"

  # Link work_item -> closure (has_closure)
  REL_PAYLOAD="$(cat <<EOF
{
  "source": {
    "id": "$WI_ID",
    "version_id": "$WI_VER",
    "kind": "object",
    "class": "work_item"
  },
  "target": {
    "id": "$CL_ID",
    "version_id": "$CL_VER",
    "kind": "object",
    "class": "closure"
  },
  "relation_type": "has_closure",
  "qualifiers": {},
  "scope": null
}
EOF
)"
  "$EM_CMD" deposit --class any --kind relation \
    --header relation_auth_endpoint=source \
    --header relation_auth_class=work_item \
    --header relation_auth_direction=outgoing \
    --header relation_auth_authority=source \
    --json-payload "$REL_PAYLOAD" >/dev/null

else
  echo "dispatch-native: execution failed with exit code $status. Depositing failed trace and closure..."

  # Deposit trace_event (failed)
  TE_PAYLOAD="$(printf '{"work_item_id":"%s","event_type":"failed","message":"Execution failed with exit code %d"}' "$WI_ID" "$status")"
  TE_JSON="$("$EM_CMD" --json deposit --class trace_event --title "Trace Event: Failed" --json-payload "$TE_PAYLOAD")"
  TE_ID="$(python3 -c 'import json,sys;print(json.loads(sys.stdin.read())["data"]["object_id"])' <<< "$TE_JSON")"
  TE_VER="$(python3 -c 'import json,sys;print(json.loads(sys.stdin.read())["data"]["version_id"])' <<< "$TE_JSON")"

  # Link dispatch -> trace_event (emitted_trace)
  REL_PAYLOAD="$(cat <<EOF
{
  "source": {
    "id": "$DP_ID",
    "version_id": "$DP_VER",
    "kind": "object",
    "class": "dispatch"
  },
  "target": {
    "id": "$TE_ID",
    "version_id": "$TE_VER",
    "kind": "object",
    "class": "trace_event"
  },
  "relation_type": "emitted_trace",
  "qualifiers": {},
  "scope": null
}
EOF
)"
  "$EM_CMD" deposit --class any --kind relation \
    --header relation_auth_endpoint=source \
    --header relation_auth_class=dispatch \
    --header relation_auth_direction=outgoing \
    --header relation_auth_authority=source \
    --json-payload "$REL_PAYLOAD" >/dev/null

  # Deposit closure
  CL_PAYLOAD="$(printf '{"work_item_id":"%s","disposition":"failed","summary":"Task execution failed with status %d"}' "$WI_ID" "$status")"
  CL_JSON="$("$EM_CMD" --json deposit --class closure --title "Closure: Failed" --json-payload "$CL_PAYLOAD")"
  CL_ID="$(python3 -c 'import json,sys;print(json.loads(sys.stdin.read())["data"]["object_id"])' <<< "$CL_JSON")"
  CL_VER="$(python3 -c 'import json,sys;print(json.loads(sys.stdin.read())["data"]["version_id"])' <<< "$CL_JSON")"

  # Link work_item -> closure (has_closure)
  REL_PAYLOAD="$(cat <<EOF
{
  "source": {
    "id": "$WI_ID",
    "version_id": "$WI_VER",
    "kind": "object",
    "class": "work_item"
  },
  "target": {
    "id": "$CL_ID",
    "version_id": "$CL_VER",
    "kind": "object",
    "class": "closure"
  },
  "relation_type": "has_closure",
  "qualifiers": {},
  "scope": null
}
EOF
)"
  "$EM_CMD" deposit --class any --kind relation \
    --header relation_auth_endpoint=source \
    --header relation_auth_class=work_item \
    --header relation_auth_direction=outgoing \
    --header relation_auth_authority=source \
    --json-payload "$REL_PAYLOAD" >/dev/null
fi

echo "dispatch-native: finished. Work Item ID: $WI_ID. Status: $status."
exit "$status"
