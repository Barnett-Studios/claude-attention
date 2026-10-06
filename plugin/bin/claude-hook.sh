#!/usr/bin/env bash
# claude-hook.sh — the Claude Code adapter for ntfyer. Maps a Claude Code hook payload (stdin) to
# an ntfyer envelope and runs `ntfyer signal --json`. This is the only Claude-specific code: which
# events stay quiet is adapter policy, the signal itself is ntfyer's.
#
#   claude-hook.sh            Stop / Notification hook: signal (or stay quiet)
#   claude-hook.sh --build    SessionStart hook: `ntfyer build`, silently (stdout reaches Claude)
#
# quiet when: a Subagent* event, a payload carrying agent_id (present only inside a subagent), a
# subagent transcript, or a Stop while background tasks are still running (the session wakes again
# on its own). agent_type alone is not a subagent: a main session started with --agent carries it.
#
# Fail-open: no ntfyer or no jq → exit 0 without a signal. Deliberately no `set -e`: a hook must
# never fail the session.
set -uo pipefail

ntfyer="$(command -v ntfyer || true)"
if [[ -z "$ntfyer" ]]; then
  # hooks often run with a minimal PATH; look where the documented installs put it
  for c in /opt/homebrew/bin/ntfyer /usr/local/bin/ntfyer "$HOME/.cargo/bin/ntfyer"; do
    if [[ -x "$c" ]]; then ntfyer="$c"; break; fi
  done
fi
[[ -n "$ntfyer" ]] || exit 0

if [[ "${1:-}" == "--build" ]]; then
  "$ntfyer" build >/dev/null 2>&1
  exit 0
fi

command -v jq >/dev/null 2>&1 || exit 0
payload="$(cat 2>/dev/null || true)"

# every field is type-checked: a malformed field is treated as absent, never as a reason to fall
# back to a loud generic signal
envelope="$(printf '%s' "$payload" | jq -c --arg pwd "$PWD" '
  if type != "object" then error("not an object") else . end
  | ((.cwd | strings) // $pwd) as $project
  | ((.hook_event_name | strings) // "") as $event
  | {
      message: ((.message | strings) // null),
      project: $project,
      event: (if $event == "" then null else ($event | ascii_downcase) end),
      session: ((.session_id | strings) // null),
      title: ("Claude Code · " + ($project | split("/") | map(select(. != "")) | last // "/")),
      quiet: (
        ($event | startswith("Subagent"))
        or (((.agent_id | strings) // "") != "")
        or (((.transcript_path | strings) // (.agent_transcript_path | strings) // "") | test("/subagents/"))
        or ($event == "Stop"
            and ([((.background_tasks | arrays) // [])[] | objects | select(.status == "running")] | length) > 0)
      )
    }' 2>/dev/null)" || envelope=""

if [[ -z "$envelope" ]]; then
  # unreadable payload: stay quiet if it looks like a subagent's, else a plain "needs your
  # attention" for the hook's directory
  case "$payload" in
    *Subagent* | *'"agent_id"'* | */subagents/*) quiet=true ;;
    *) quiet=false ;;
  esac
  envelope="$(jq -cn --arg pwd "$PWD" --argjson quiet "$quiet" \
    '{project: $pwd, quiet: $quiet, title: ("Claude Code · " + ($pwd | split("/") | map(select(. != "")) | last // "/"))}')"
fi

printf '%s' "$envelope" | "$ntfyer" signal --json
exit 0
