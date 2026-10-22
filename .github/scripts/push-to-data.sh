#!/usr/bin/env bash
# Pushes one format's results.yaml to serialbench/data via the GitHub
# Contents API (atomic per-file, no merge conflicts from parallel legs).
# Same contract as the ruby/python/c harnesses, with a runtime segment:
# runs/{date}/{platform}-rust-{version}.{format}.yaml
set -euo pipefail

FMT="$1"
RUNTIME_VERSION="$2"
PLATFORM="$3"
RESULTS_FILE="results/${FMT}/results.yaml"
DATA_TOKEN="${DATA_REPO_TOKEN:-}"
RUNTIME="rust"

if [ -z "$DATA_TOKEN" ]; then
  echo "::warning::DATA_REPO_TOKEN not set — skipping data push"
  exit 0
fi

if [ ! -f "$RESULTS_FILE" ]; then
  echo "::warning::No results at $RESULTS_FILE — skipping"
  exit 0
fi

DATE="${DATA_RUN_DATE:-$(date -u +%Y-%m-%d)}"
TARGET_PATH="runs/${DATE}/${PLATFORM}-${RUNTIME}-${RUNTIME_VERSION}.${FMT}.yaml"
CONTENT=$(base64 < "$RESULTS_FILE" | tr -d '\r\n')

echo "Pushing ${TARGET_PATH} to serialbench/data..."

# Try create; 409 (branch race) and 422 (exists) are retried with a fresh
# sha and backoff. The body goes through a file: passing it as a curl
# argument hits the ~32KB exec limit on windows/msys.
put_file() {
  local body
  body=$(mktemp /tmp/push-body.XXXXXX)
  if [ -n "${1:-}" ]; then
    printf '{"message": "%s (update)", "content": "%s", "sha": "%s", "branch": "main"}' \
      "${PLATFORM} ${RUNTIME}-${RUNTIME_VERSION} ${FMT}" "$CONTENT" "$1" > "$body"
  else
    printf '{"message": "%s", "content": "%s", "branch": "main"}' \
      "${PLATFORM} ${RUNTIME}-${RUNTIME_VERSION} ${FMT}" "$CONTENT" > "$body"
  fi
  curl -s -o /tmp/data-push-resp.json -w "%{http_code}" -X PUT \
    -H "Authorization: Bearer ${DATA_TOKEN}" \
    -H "Accept: application/vnd.github+json" \
    "https://api.github.com/repos/serialbench/data/contents/${TARGET_PATH}" \
    --data-binary @"$body" || echo 000
  rm -f "$body"
}

# Sha of the existing data file, empty on any failure — guarded so a dead
# pipe (curl -f, grep miss, SIGPIPE under pipefail) cannot kill the leg.
fetch_sha() {
  curl -s -H "Authorization: Bearer ${DATA_TOKEN}" \
    "https://api.github.com/repos/serialbench/data/contents/${TARGET_PATH}" \
    | grep -m1 -oE '"sha": ?"[a-f0-9]{40}"' | grep -oE '[a-f0-9]{40}' || true
}

HTTP_CODE=$(put_file "")
for TRY in 1 2 3 4 5; do
  if [ "$HTTP_CODE" = "200" ] || [ "$HTTP_CODE" = "201" ]; then break; fi
  if [ "$HTTP_CODE" = "409" ] || [ "$HTTP_CODE" = "422" ]; then
    SHA=$(fetch_sha)
    [ -z "$SHA" ] && break
    echo "attempt $TRY got $HTTP_CODE — refetching sha and retrying"
    sleep $((TRY * 3))
    HTTP_CODE=$(put_file "$SHA")
  elif [ "$HTTP_CODE" = "000" ]; then
    echo "attempt $TRY got 000 (no response) — retrying"
    sleep $((TRY * 5))
    HTTP_CODE=$(put_file "")
  else
    break
  fi
done

if [ "$HTTP_CODE" = "200" ] || [ "$HTTP_CODE" = "201" ]; then
  echo "Pushed ${TARGET_PATH}"
else
  echo "::warning::Push failed (${HTTP_CODE}): $(cat /tmp/data-push-resp.json 2>/dev/null | head -c 200)"
fi

# Trigger site rebuild (debounced by concurrency group on the .io repo)
curl -sf -X POST \
  -H "Authorization: Bearer ${DATA_TOKEN}" \
  -H "Accept: application/vnd.github+json" \
  "https://api.github.com/repos/serialbench/serialbench.github.io/dispatches" \
  -d '{"event_type": "data-updated"}' \
  && echo "Site rebuild triggered" \
  || echo "::warning::Failed to trigger site rebuild"
