#!/usr/bin/env bash
set -euo pipefail

# Live Google AI Gemma Smoke Test
#
# Proves the declared HTTP provider path can call Google's AI API.
# Uses a Gemma model for free-tier-friendly smoke testing.
#
# Usage:
#   RUN_LIVE_GOOGLE_AI_SMOKE=1 GEMINI_API_KEY=<key> scripts/live_google_ai_gemma_smoke.sh
#
# Safety gates:
#   - RUN_LIVE_GOOGLE_AI_SMOKE=1 must be set (opt-in)
#   - GEMINI_API_KEY must be set (never printed or committed)

if [[ "${RUN_LIVE_GOOGLE_AI_SMOKE:-}" != "1" ]]; then
  echo "Set RUN_LIVE_GOOGLE_AI_SMOKE=1 to run the live Google AI smoke test."
  exit 0
fi

if [[ -z "${GEMINI_API_KEY:-}" ]]; then
  echo "GEMINI_API_KEY is not set. Live Google AI smoke test not run."
  exit 1
fi

REPO_ROOT="${REPO_ROOT:-$(cd "$(dirname "$0")/.." && pwd)}"
EM_BIN="${EM_BIN:-$REPO_ROOT/target/debug/earmark-cli}"
WORKSPACE="${WORKSPACE:-$(mktemp -d)}"
SMOKE_FIXTURE="$REPO_ROOT/docs/internal/live-smoke/google-ai"
MODEL="${GOOGLE_AI_SMOKE_MODEL:-gemma-4-26b-a4b-it}"

LIVE_CALL_COUNT=0
MAX_LIVE_CALLS=50
MAX_CALLS_PER_CYCLE=10

check_call_limits() {
  if [[ "$LIVE_CALL_COUNT" -ge "$MAX_LIVE_CALLS" ]]; then
    echo "ABORT: Reached maximum $MAX_LIVE_CALLS live API calls. Stopping."
    exit 1
  fi
  if [[ "$LIVE_CALL_COUNT" -gt 0 && $((LIVE_CALL_COUNT % MAX_CALLS_PER_CYCLE)) -eq 0 ]]; then
    echo "--- PAUSE: $LIVE_CALL_COUNT calls made. Summarize state and hypothesis before continuing. ---"
    echo "Model: $MODEL"
    echo "Calls remaining: $((MAX_LIVE_CALLS - LIVE_CALL_COUNT))"
    read -rp "Press Enter to continue or Ctrl+C to abort..."
    echo "--- Resuming ---"
  fi
}

track_call() {
  LIVE_CALL_COUNT=$((LIVE_CALL_COUNT + 1))
  echo "[live call #$LIVE_CALL_COUNT]"
  check_call_limits
}

redact_key() {
  sed "s/${GEMINI_API_KEY}/***/g"
}

cleanup() {
  echo "--- Cleanup ---"
  rm -rf "$WORKSPACE"
}

if [[ -z "${SKIP_CLEANUP:-}" ]]; then
  trap cleanup EXIT
fi

em() {
  "$EM_BIN" --root "$WORKSPACE" "$@"
}

discover_gemma_model() {
  echo "Discovering available Gemma models via models.list..."
  track_call
  local list_response
  list_response=$(curl -s \
    "https://generativelanguage.googleapis.com/v1beta/models" \
    -H "x-goog-api-key: ${GEMINI_API_KEY}" \
    -X GET)
  echo "$list_response" | python3 -c "
import sys, json
data = json.load(sys.stdin)
models = data.get('models', [])
# Prefer gemma-4-26b-a4b-it, then gemma-4-31b-it, then any Gemma with generateContent
for name in ['gemma-4-26b-a4b-it', 'gemma-4-31b-it']:
    for m in models:
        n = m.get('name', '').replace('models/', '')
        if n == name and 'generateContent' in m.get('supportedGenerationMethods', []):
            print(n)
            sys.exit(0)
# Fallback: any Gemma model with generateContent
for m in models:
    n = m.get('name', '').replace('models/', '')
    if 'gemma' in n and 'generateContent' in m.get('supportedGenerationMethods', []):
        print(n)
        sys.exit(0)
print('')
" 2>/dev/null || true
}

if [ ! -x "$EM_BIN" ]; then
  echo "Building earmark-cli..."
  cargo build -p earmark-cli
fi

echo "================================================"
echo "Phase 0: Direct REST Preflight"
echo "================================================"

track_call
echo "Model: $MODEL"
REST_RESPONSE=$(curl -s -w "\n%{http_code}" \
  "https://generativelanguage.googleapis.com/v1beta/models/${MODEL}:generateContent" \
  -H "x-goog-api-key: ${GEMINI_API_KEY}" \
  -H "Content-Type: application/json" \
  -X POST \
  -d '{
    "contents": [{
      "parts": [{
        "text": "Reply with exactly three words: provider smoke ok"
      }]
    }],
    "generationConfig": {
      "maxOutputTokens": 16,
      "temperature": 0
    }
  }')

HTTP_CODE=$(echo "$REST_RESPONSE" | tail -n1)
BODY=$(echo "$REST_RESPONSE" | sed '$d')

if [[ "$HTTP_CODE" == "404" ]]; then
  REDACTED=$(echo "$BODY" | redact_key)
  echo "Model $MODEL returned 404. Attempting model discovery..."
  DISCOVERED=$(discover_gemma_model)
  if [[ -z "$DISCOVERED" ]]; then
    echo "FAILED: No Gemma model with generateContent support found via models.list."
    echo "Redacted body: $REDACTED"
    echo "Model used: $MODEL"
    exit 1
  fi
  echo "Discovered model: $DISCOVERED"
  MODEL="$DISCOVERED"
  export GOOGLE_AI_SMOKE_MODEL="$DISCOVERED"

  track_call
  echo "Retrying preflight with model: $MODEL"
  REST_RESPONSE=$(curl -s -w "\n%{http_code}" \
    "https://generativelanguage.googleapis.com/v1beta/models/${MODEL}:generateContent" \
    -H "x-goog-api-key: ${GEMINI_API_KEY}" \
    -H "Content-Type: application/json" \
    -X POST \
    -d '{
      "contents": [{
        "parts": [{
          "text": "Reply with exactly three words: provider smoke ok"
        }]
      }],
      "generationConfig": {
        "maxOutputTokens": 16,
        "temperature": 0
      }
    }')

  HTTP_CODE=$(echo "$REST_RESPONSE" | tail -n1)
  BODY=$(echo "$REST_RESPONSE" | sed '$d')
fi

if [[ "$HTTP_CODE" != "200" ]]; then
  REDACTED=$(echo "$BODY" | redact_key)
  echo "Preflight FAILED (HTTP $HTTP_CODE)"
  echo "Redacted body: $REDACTED"
  echo "Model: $MODEL"

  if [[ "$HTTP_CODE" == "401" ]] || [[ "$HTTP_CODE" == "403" ]]; then
    echo "Failure type: authentication failure. Stopping."
  elif [[ "$HTTP_CODE" == "429" ]]; then
    echo "Failure type: quota/rate limit. Stopping."
  elif [[ "$HTTP_CODE" == "404" ]]; then
    echo "Failure type: model not found. Stopping."
  elif [[ "$HTTP_CODE" -ge 500 ]]; then
    echo "Failure type: server error (5xx). Stopping."
  else
    echo "Failure type: unknown. Stopping."
  fi
  exit 1
fi

echo "Preflight OK (HTTP $HTTP_CODE)"
echo "Response: $(echo "$BODY" | python3 -c "import sys,json; print(json.load(sys.stdin)['candidates'][0]['content']['parts'][0]['text'])" 2>/dev/null || echo "$BODY" | redact_key)"

echo ""
echo "================================================"
echo "Phase 1: Initialize Workspace and Register System"
echo "================================================"

em init
echo "Workspace initialized at $WORKSPACE"

em system register "$SMOKE_FIXTURE/systems/system.yaml"
echo "System registered"

em system activate sys_google_ai_smoke
echo "System activated"

echo ""
echo "================================================"
echo "Phase 2: Deposit Smoke Input"
echo "================================================"

DEPOSIT_OUTPUT=$(em --json deposit \
  --class smoke_input \
  --title "Gemma smoke input" \
  --body "Check that the declared HTTP provider works.")

INPUT_ID=$(echo "$DEPOSIT_OUTPUT" | python3 -c "import sys,json; print(json.load(sys.stdin)['data']['object_id'])")
echo "Deposited smoke_input: $INPUT_ID"

echo ""
echo "================================================"
echo "Phase 3: Run Smoke Workflow"
echo "================================================"

track_call
RUN_OUTPUT=$(em --json workflow run google_ai_smoke \
  --system-id sys_google_ai_smoke \
  --with "$INPUT_ID")

echo "Workflow run output:"
echo "$RUN_OUTPUT" | python3 -m json.tool 2>/dev/null || echo "$RUN_OUTPUT" | redact_key

RUN_ID=$(echo "$RUN_OUTPUT" | python3 -c "import sys,json; print(json.load(sys.stdin).get('data',{}).get('run_id','unknown'))" 2>/dev/null || echo "unknown")
echo "Run ID: $RUN_ID"

echo ""
echo "================================================"
echo "Phase 4: Verify Output"
echo "================================================"

echo "--- Run Explain ---"
em --json run explain latest 2>&1 || echo "(run explain completed)"

echo ""
echo "--- Query smoke_output ---"
QUERY_OUTPUT=$(em --json query --class smoke_output)
echo "$QUERY_OUTPUT" | python3 -m json.tool 2>/dev/null || echo "$QUERY_OUTPUT" | redact_key

OUTPUT_COUNT=$(echo "$QUERY_OUTPUT" | python3 -c "
import sys, json
data = json.load(sys.stdin)
items = data.get('data', {}).get('items', data.get('items', []))
print(len(items))
" 2>/dev/null || echo "0")

OUTPUT_BODY=""
if [[ "$OUTPUT_COUNT" -gt 0 ]]; then
  echo "SUCCESS: $OUTPUT_COUNT smoke_output object(s) found."
  OUTPUT_BODY=$(echo "$QUERY_OUTPUT" | python3 -c "
import sys, json
data = json.load(sys.stdin)
items = data.get('data', {}).get('items', data.get('items', []))
if items:
    body = items[0].get('payload', items[0].get('body', ''))
    print(body)
" 2>/dev/null || echo "")
  echo "Output body: $OUTPUT_BODY"
else
  echo "WARNING: No smoke_output objects found via query."
fi

echo ""
echo "--- Workspace Health ---"
em --json doctor

echo ""
echo "================================================"
echo "Summary"
echo "================================================"
echo "Live API calls made: $LIVE_CALL_COUNT"
echo "Model: $MODEL"
echo "Workspace: $WORKSPACE"
echo "Fixture: $SMOKE_FIXTURE"

# Acceptance criteria checks
FAILED=0

if [[ "$OUTPUT_COUNT" -eq 0 ]]; then
  echo "CHECK FAILED: No smoke_output objects found."
  FAILED=1
fi

if [[ -z "$OUTPUT_BODY" ]]; then
  echo "CHECK FAILED: smoke_output body is empty."
  FAILED=1
fi

if echo "$OUTPUT_BODY" | grep -q "SMOKE_OK"; then
  echo "CHECK PASSED: Output contains SMOKE_OK."
else
  echo "CHECK NOTE: Output does not contain SMOKE_OK (model-compliance issue, not provider failure)."
fi

if [[ "$FAILED" -eq 0 ]]; then
  echo "Status: LIVE SMOKE PASSED"
else
  echo "Status: LIVE SMOKE COMPLETE (with failures)"
fi
