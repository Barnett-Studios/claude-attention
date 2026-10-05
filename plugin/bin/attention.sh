#!/usr/bin/env bash
# attention.sh — terminal bell + chime + macOS notification when Claude Code needs the user.
# Wired to the Stop and Notification hooks by hooks/hooks.json; can also be called directly with
# {"message": "..."} on stdin. Reads the hook JSON on stdin when present.
#
# Fail-open by design: a notifier must never break the session, so this exits 0 on every path and
# deliberately runs without `set -e` (any single failed channel is ignored, the rest still fire).
#
# Config (each key optional, a missing file or key keeps the default):
#   ~/.claude/attention.json, then <project>/.claude/attention.json overriding it key by key
#   { "enabled": true, "bell": true, "sound": true | false | "<System sound>" | "/path/file",
#     "popup": true, "icon": "claude" | "/path/image" }      (icon: global file only, see notifier.sh)
#   CLAUDE_ATTENTION=off in the environment silences everything.
# ATTENTION_DRY_RUN=1 prints each channel it would fire instead of firing it (used by tests/run.sh).
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
input="$(cat 2>/dev/null || true)"
j() { printf '%s' "$input" | jq -r "$1" 2>/dev/null || true; }
event="$(j '.hook_event_name // empty')"
msg="$(j '.message // empty')"
cwd="$(j '.cwd // empty')"
cwd="${cwd:-$PWD}"
project="$(basename "$cwd")"
tpath="$(j '.transcript_path // .agent_transcript_path // empty')"
agent="$(j '.agent_id // .agent_type // empty')"
# background work still in flight (subagents, shells, monitors): the session will wake again on
# its own, so a Stop now is a pause, not a wait for the user
running="$(j '[.background_tasks // [] | .[] | select(.status == "running")] | length')"
[[ "$running" =~ ^[0-9]+$ ]] || running=0
dry="${ATTENTION_DRY_RUN:-}"

# cfg <key> → the effective value as a string ("true" when unset); malformed files are ignored
cfg() {
  local v="true" f r
  for f in "$HOME/.claude/attention.json" "$cwd/.claude/attention.json"; do
    [[ -f "$f" ]] || continue
    r="$(jq -r --arg k "$1" 'if type == "object" and has($k) then .[$k] | tostring else empty end' "$f" 2>/dev/null)"
    [[ -n "$r" ]] && v="$r"
  done
  printf '%s' "$v"
}
on() { [[ "$(cfg "$1")" != "false" ]]; }

verdict="signal"
case "$event" in SubagentStop|SubagentStart) verdict="skip:subagent-event" ;; esac
[[ -n "$agent" ]] && verdict="skip:agent-field"
case "$tpath" in */subagents/*) verdict="skip:subagent-transcript" ;; esac
[[ "$event" == "Stop" && "$running" -gt 0 ]] && verdict="skip:${running}-tasks-running"
[[ "${CLAUDE_ATTENTION:-}" == "off" ]] && verdict="skip:env-off"
[[ "$verdict" == "signal" ]] && ! on enabled && verdict="skip:disabled"

# audit trail (last 200 lines) so a stray or missing signal can be traced to its payload
log="$HOME/.claude/attention.log"
if [[ -d "$HOME/.claude" ]]; then
  printf '%s event=%s verdict=%s project=%s sid=%s\n' "$(date +%FT%T)" "${event:-manual}" "$verdict" \
    "$project" "$(j '.session_id // "-"')" >> "$log" 2>/dev/null
  { tail -n 200 "$log" > "$log.tmp" && mv "$log.tmp" "$log"; } 2>/dev/null
fi
[[ "$verdict" == "signal" ]] || exit 0

# debounce: one signal per burst, so a manual call followed by the Stop/Notification hook never
# doubles up
WINDOW=8
stamp="${TMPDIR:-/tmp}/claude-attention.last"
now=$(date +%s)
last=$(cat "$stamp" 2>/dev/null || echo 0)
[[ "$last" =~ ^[0-9]+$ ]] || last=0
(( now - last < WINDOW )) && exit 0
printf '%s' "$now" > "$stamp" 2>/dev/null

if [[ -z "$msg" ]]; then
  case "$event" in
    Stop) msg="Finished and waiting for you." ;;
    *)    msg="Needs your attention." ;;
  esac
fi
title="Claude Code · $project"

# sound_file → the file the "sound" setting names, or nothing: true → Glass, a bare name → that
# macOS system sound, anything else → a path
sound_file() {
  local s; s="$(cfg sound)"
  case "$s" in
    false) return ;;
    true)  s="Glass" ;;
  esac
  [[ "$s" == */* ]] || s="/System/Library/Sounds/$s.aiff"
  [[ -f "$s" ]] && printf '%s' "$s"
}

# 1. terminal bell (best effort: hooks usually have no tty)
if on bell; then
  if [[ -n "$dry" ]]; then echo bell
  else { printf '\a' > /dev/tty; } 2>/dev/null || printf '\a' 2>/dev/null; fi
fi

# 2. chime
snd="$(sound_file)"
if [[ -n "$snd" ]]; then
  if [[ -n "$dry" ]]; then echo "sound:$snd"
  elif command -v afplay >/dev/null 2>&1; then afplay "$snd" >/dev/null 2>&1 & fi
fi

# 3. notification popup, through the notifier app (its icon) when built, else osascript (whose
# icon is always Script Editor's)
if on popup; then
  if [[ -n "$dry" ]]; then echo "popup:$title|$msg"; exit 0; fi
  "$here/notifier.sh" notify "$title" "$msg" "claude-attention-$project" >/dev/null 2>&1 || {
    t=${title//\\/\\\\}; t=${t//\"/\\\"}
    m=${msg//\\/\\\\}; m=${m//\"/\\\"}
    osascript -e "display notification \"$m\" with title \"$t\"" >/dev/null 2>&1
  }
fi
exit 0
