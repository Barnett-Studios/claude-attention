#!/usr/bin/env bash
# Behaviour tests for bin/attention.sh. Uses ATTENTION_DRY_RUN=1, which prints each channel it would
# fire (bell / sound / popup) instead of firing it, and an isolated HOME / TMPDIR per case.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
script="$here/../plugin/bin/attention.sh"
pass=0 fail=0

# run <case-dir> <stdin-json> → channels fired, one per line
run() {
  local dir="$1" input="$2"
  HOME="$dir/home" TMPDIR="$dir/tmp" ATTENTION_DRY_RUN=1 bash "$script" <<<"$input"
}
check() {
  local name="$1" want="$2" got="$3"
  if [[ "$got" == "$want" ]]; then pass=$((pass + 1)); else
    fail=$((fail + 1)); printf 'FAIL %s\n  want: %q\n  got:  %q\n' "$name" "$want" "$got"; fi
}
fresh() {
  local d; d="$(mktemp -d)"; mkdir -p "$d/home/.claude" "$d/tmp" "$d/proj/.claude"; printf '%s' "$d"
}
cleanup_dirs=()
trap 'rm -rf "${cleanup_dirs[@]}"' EXIT

glass=/System/Library/Sounds/Glass.aiff
all=$'bell\nsound:'"$glass"$'\npopup:Claude Code · proj|hello'

d="$(fresh)"; cleanup_dirs+=("$d")
check "defaults fire every channel" "$all" "$(run "$d" "{\"message\":\"hello\",\"cwd\":\"$d/proj\"}")"

d="$(fresh)"; cleanup_dirs+=("$d")
echo '{"enabled":false}' > "$d/home/.claude/attention.json"
check "global enabled=false silences all" "" "$(run "$d" "{\"message\":\"hello\",\"cwd\":\"$d/proj\"}")"

d="$(fresh)"; cleanup_dirs+=("$d")
echo '{"enabled":false}' > "$d/home/.claude/attention.json"
echo '{"enabled":true}' > "$d/proj/.claude/attention.json"
check "project overrides global" "$all" "$(run "$d" "{\"message\":\"hello\",\"cwd\":\"$d/proj\"}")"

d="$(fresh)"; cleanup_dirs+=("$d")
echo '{"sound":false,"bell":false}' > "$d/proj/.claude/attention.json"
check "per-channel switches" "popup:Claude Code · proj|hello" "$(run "$d" "{\"message\":\"hello\",\"cwd\":\"$d/proj\"}")"

d="$(fresh)"; cleanup_dirs+=("$d")
echo '{"popup":false}' > "$d/home/.claude/attention.json"
check "popup off keeps bell and sound" $'bell\nsound:'"$glass" "$(run "$d" "{\"message\":\"hello\",\"cwd\":\"$d/proj\"}")"

d="$(fresh)"; cleanup_dirs+=("$d")
check "env CLAUDE_ATTENTION=off silences all" "" \
  "$(CLAUDE_ATTENTION=off run "$d" "{\"message\":\"hello\",\"cwd\":\"$d/proj\"}")"

d="$(fresh)"; cleanup_dirs+=("$d")
check "subagent stop is skipped" "" "$(run "$d" "{\"hook_event_name\":\"SubagentStop\",\"cwd\":\"$d/proj\"}")"

d="$(fresh)"; cleanup_dirs+=("$d")
check "stop with running background tasks is skipped" "" \
  "$(run "$d" "{\"hook_event_name\":\"Stop\",\"cwd\":\"$d/proj\",\"background_tasks\":[{\"status\":\"running\"}]}")"

d="$(fresh)"; cleanup_dirs+=("$d")
check "stop without message gets default text" $'bell\nsound:'"$glass"$'\npopup:Claude Code · proj|Finished and waiting for you.' \
  "$(run "$d" "{\"hook_event_name\":\"Stop\",\"cwd\":\"$d/proj\"}")"

d="$(fresh)"; cleanup_dirs+=("$d")
run "$d" "{\"message\":\"first\",\"cwd\":\"$d/proj\"}" >/dev/null
check "second signal inside debounce window is silent" "" "$(run "$d" "{\"message\":\"again\",\"cwd\":\"$d/proj\"}")"

d="$(fresh)"; cleanup_dirs+=("$d")
echo 'not json' > "$d/home/.claude/attention.json"
check "malformed config fails open" "$all" "$(run "$d" "{\"message\":\"hello\",\"cwd\":\"$d/proj\"}")"

d="$(fresh)"; cleanup_dirs+=("$d")
check "empty stdin still signals" $'bell\nsound:'"$glass"$'\npopup:Claude Code · proj|Needs your attention.' \
  "$(cd "$d/proj" && HOME="$d/home" TMPDIR="$d/tmp" ATTENTION_DRY_RUN=1 bash "$script" </dev/null)"

d="$(fresh)"; cleanup_dirs+=("$d")
echo '{"sound":"Ping"}' > "$d/proj/.claude/attention.json"
check "sound by system name" $'bell\nsound:/System/Library/Sounds/Ping.aiff\npopup:Claude Code · proj|hello' \
  "$(run "$d" "{\"message\":\"hello\",\"cwd\":\"$d/proj\"}")"

d="$(fresh)"; cleanup_dirs+=("$d")
touch "$d/custom.wav"
echo "{\"sound\":\"$d/custom.wav\"}" > "$d/home/.claude/attention.json"
check "sound by file path" $'bell\nsound:'"$d/custom.wav"$'\npopup:Claude Code · proj|hello' \
  "$(run "$d" "{\"message\":\"hello\",\"cwd\":\"$d/proj\"}")"

d="$(fresh)"; cleanup_dirs+=("$d")
echo '{"sound":"NoSuchSound"}' > "$d/home/.claude/attention.json"
check "unknown sound plays nothing, other channels still fire" $'bell\npopup:Claude Code · proj|hello' \
  "$(run "$d" "{\"message\":\"hello\",\"cwd\":\"$d/proj\"}")"

d="$(fresh)"; cleanup_dirs+=("$d")
echo '{"sound":false}' > "$d/home/.claude/attention.json"
check "sound false plays nothing" $'bell\npopup:Claude Code · proj|hello' \
  "$(run "$d" "{\"message\":\"hello\",\"cwd\":\"$d/proj\"}")"

d="$(fresh)"; cleanup_dirs+=("$d")
echo '{"enabled":false}' > "$d/home/.claude/attention.json"
echo '{"enabled":null}' > "$d/proj/.claude/attention.json"
check "null in project config does not override global" "" "$(run "$d" "{\"message\":\"hello\",\"cwd\":\"$d/proj\"}")"

d="$(fresh)"; cleanup_dirs+=("$d")
echo '{"sound":null}' > "$d/home/.claude/attention.json"
check "null sound keeps the default" "$all" "$(run "$d" "{\"message\":\"hello\",\"cwd\":\"$d/proj\"}")"

d="$(fresh)"; cleanup_dirs+=("$d")
touch "$d/home/chime.wav"
echo '{"sound":"~/chime.wav"}' > "$d/home/.claude/attention.json"
check "sound path expands ~" $'bell\nsound:'"$d/home/chime.wav"$'\npopup:Claude Code · proj|hello' \
  "$(run "$d" "{\"message\":\"hello\",\"cwd\":\"$d/proj\"}")"

d="$(fresh)"; cleanup_dirs+=("$d"); mkdir -p "$d/other"
run "$d" "{\"message\":\"first\",\"cwd\":\"$d/proj\"}" >/dev/null
check "debounce is per project: another project still signals" $'bell\nsound:'"$glass"$'\npopup:Claude Code · other|hello' \
  "$(run "$d" "{\"message\":\"hello\",\"cwd\":\"$d/other\"}")"

d="$(fresh)"; cleanup_dirs+=("$d")
nl="$d/a"$'\n'"b"; mkdir -p "$nl"
run "$d" "$(jq -nc --arg c "$nl" '{message:"x",cwd:$c}')" >/dev/null
check "log stays one line per event despite a newline in the project name" "1" \
  "$(wc -l < "$d/home/.claude/attention.log" | tr -d ' ')"

printf '%d passed, %d failed\n' "$pass" "$fail"
[[ "$fail" -eq 0 ]]
