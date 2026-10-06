#!/usr/bin/env bash
# Tests for plugin/bin/claude-hook.sh — the Claude Code adapter. It maps a Claude Code hook payload
# to an ntfyer envelope (quiet for subagents and busy turns) and calls `ntfyer signal --json`.
# Runs against the built binary in dry-run mode; bash 3.2 safe (stock macOS).
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
root="$here/.."
hook="$root/plugin/bin/claude-hook.sh"
bin="${NTFYER_BIN:-$root/target/debug/ntfyer}"
[[ -x "$bin" ]] || { echo "build ntfyer first (cargo build), or set NTFYER_BIN" >&2; exit 1; }
bindir="$(mktemp -d)"
ln -s "$bin" "$bindir/ntfyer"
pass=0 fail=0
cleanup_dirs=("$bindir")
trap 'rm -rf "${cleanup_dirs[@]}"' EXIT

fresh() {
  local d; d="$(mktemp -d)"
  mkdir -p "$d/home/.config/ntfyer" "$d/proj"
  # OS-neutral channels: no chime (the default sound file differs per OS)
  printf '{"sound":false}' > "$d/home/.config/ntfyer/config.json"
  cleanup_dirs+=("$d")
  printf '%s' "$d"
}
# run <case-dir> <payload> [PATH] → stdout of the hook
run() {
  local d="$1" payload="$2" path="${3:-$bindir:/usr/bin:/bin:/usr/sbin:/sbin}"
  (cd "$d/proj" && env -i HOME="$d/home" PATH="$path" NTFYER_DRY_RUN=1 /bin/bash "$hook" <<<"$payload")
}
check() {
  if [[ "$3" == "$2" ]]; then pass=$((pass + 1)); else
    fail=$((fail + 1)); printf 'FAIL %s\n  want: %q\n  got:  %q\n' "$1" "$2" "$3"; fi
}
log_of() { cat "$1/home/.local/state/ntfyer/log" 2>/dev/null || true; }

d="$(fresh)"
check "stop with message" $'bell\npopup:Claude Code · proj|done' \
  "$(run "$d" '{"hook_event_name":"Stop","message":"done","cwd":"'"$d/proj"'","session_id":"s1"}')"
case "$(log_of "$d")" in *"event=stop verdict=signal project=proj session=s1"*) pass=$((pass + 1)) ;;
  *) fail=$((fail + 1)); echo "FAIL log records event and session: $(log_of "$d")" ;; esac

d="$(fresh)"
check "stop without message" $'bell\npopup:Claude Code · proj|Finished and waiting for you.' \
  "$(run "$d" '{"hook_event_name":"Stop","cwd":"'"$d/proj"'"}')"

d="$(fresh)"
check "notification" $'bell\npopup:Claude Code · proj|Claude needs your permission' \
  "$(run "$d" '{"hook_event_name":"Notification","message":"Claude needs your permission","cwd":"'"$d/proj"'"}')"

d="$(fresh)"
check "subagent stop is quiet" "" "$(run "$d" '{"hook_event_name":"SubagentStop","cwd":"'"$d/proj"'"}')"
case "$(log_of "$d")" in *"verdict=skip:quiet"*) pass=$((pass + 1)) ;;
  *) fail=$((fail + 1)); echo "FAIL quiet is logged: $(log_of "$d")" ;; esac

d="$(fresh)"
check "agent_id is quiet" "" "$(run "$d" '{"hook_event_name":"Stop","agent_id":"a1","cwd":"'"$d/proj"'"}')"

d="$(fresh)"
check "subagent transcript is quiet" "" \
  "$(run "$d" '{"hook_event_name":"Stop","transcript_path":"/x/subagents/y.jsonl","cwd":"'"$d/proj"'"}')"

d="$(fresh)"
check "stop with running background task is quiet" "" \
  "$(run "$d" '{"hook_event_name":"Stop","cwd":"'"$d/proj"'","background_tasks":[{"status":"running"}]}')"

d="$(fresh)"
check "stop with finished background tasks signals" $'bell\npopup:Claude Code · proj|Finished and waiting for you.' \
  "$(run "$d" '{"hook_event_name":"Stop","cwd":"'"$d/proj"'","background_tasks":[{"status":"completed"}]}')"

d="$(fresh)"
check "notification while tasks run still signals" $'bell\npopup:Claude Code · proj|m' \
  "$(run "$d" '{"hook_event_name":"Notification","message":"m","cwd":"'"$d/proj"'","background_tasks":[{"status":"running"}]}')"

d="$(fresh)"
check "no cwd uses the hook's working directory" $'bell\npopup:Claude Code · proj|m' \
  "$(run "$d" '{"hook_event_name":"Notification","message":"m"}')"

d="$(fresh)"
set +e
out="$(run "$d" '{"hook_event_name":"Stop"}' "/usr/bin:/bin")"; rc=$?
set -e
check "ntfyer missing: silent exit 0" "0|" "$rc|$out"

d="$(fresh)"
nojq="$(mktemp -d)"; cleanup_dirs+=("$nojq")
ln -s "$bin" "$nojq/ntfyer"
for t in bash cat env dirname; do ln -s "$(command -v "$t")" "$nojq/$t"; done
set +e
out="$(run "$d" '{"hook_event_name":"Stop"}' "$nojq")"; rc=$?
set -e
check "jq missing: silent exit 0" "0|" "$rc|$out"

d="$(fresh)"
check "garbage payload still exits 0" "0" "$(run "$d" 'not json' >/dev/null; echo $?)"

printf '%d passed, %d failed\n' "$pass" "$fail"
[[ "$fail" -eq 0 ]]
