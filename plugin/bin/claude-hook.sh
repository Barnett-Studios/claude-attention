#!/usr/bin/env bash
# claude-hook.sh — the Claude Code adapter for ntfyer. Maps a Claude Code hook payload (stdin) to
# an ntfyer envelope and runs `ntfyer signal --json`. This is the only Claude-specific code: which
# events stay quiet is adapter policy, the signal itself is ntfyer's.
#
# quiet when: a subagent event, a payload carrying agent_id/agent_type or a subagent transcript, or
# a Stop while background tasks are still running (the session wakes again on its own).
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
command -v jq >/dev/null 2>&1 || exit 0

payload="$(cat 2>/dev/null || true)"
envelope="$(printf '%s' "$payload" | jq -c --arg pwd "$PWD" '
  (.cwd // $pwd) as $project
  | (.hook_event_name // "") as $event
  | {
      message: (.message // null),
      project: $project,
      event: (if $event == "" then null else ($event | ascii_downcase) end),
      session: (.session_id // null),
      title: ("Claude Code · " + ($project | split("/") | map(select(. != "")) | last // "/")),
      quiet: (
        ($event | startswith("Subagent"))
        or ((.agent_id // .agent_type // "") != "")
        or ((.transcript_path // .agent_transcript_path // "") | test("/subagents/"))
        or ($event == "Stop" and ([.background_tasks // [] | .[] | select(.status == "running")] | length) > 0)
      )
    }' 2>/dev/null)" || envelope=""
# an unreadable payload still signals, as a plain "needs your attention" for the hook's directory
[[ -n "$envelope" ]] || envelope="$(jq -cn --arg pwd "$PWD" '{project: $pwd, title: ("Claude Code · " + ($pwd | split("/") | map(select(. != "")) | last // "/"))}')"

printf '%s' "$envelope" | "$ntfyer" signal --json
exit 0
