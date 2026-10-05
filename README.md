# claude-attention

A Claude Code plugin that gets your attention when Claude finishes a turn or needs input:
a macOS notification popup with **your choice of icon**, a **configurable chime**, and the
terminal bell — each switchable globally or per project.

macOS only for now (Apple Silicon and Intel). On other platforms only the terminal bell fires.

## Install

```
/plugin marketplace add <this repo's git URL or local path>
/plugin install claude-attention@claude-attention
```

Requirements: `jq`, and the Xcode Command Line Tools (`xcode-select --install`) — the popup comes
from a tiny app compiled on your machine at session start, under
`~/Library/Application Support/claude-attention/`.

### First run: allow notifications

The popup is posted by a small app called **Claude Attention**. The first time it fires, macOS may
deny it without asking. Open **System Settings → Notifications → Claude Attention** and turn on
*Allow notifications* (Banners or Alerts). Until then the plugin falls back to a plain
AppleScript notification (Script Editor's icon).

## Configure

Create `~/.claude/attention.json` (global) and/or `<project>/.claude/attention.json` (overrides the
global file key by key). Every key is optional; a missing file or key keeps the default.

```json
{
  "enabled": true,
  "bell": true,
  "sound": "Glass",
  "popup": true,
  "icon": "claude"
}
```

| Key | Values | Default |
|---|---|---|
| `enabled` | `false` silences everything | `true` |
| `bell` | terminal bell on/off | `true` |
| `sound` | `false`, `true` (Glass), a macOS sound name (`Ping`, `Hero`, `Submarine`, … from `/System/Library/Sounds`), or a path to any audio file | `true` |
| `popup` | notification popup on/off | `true` |
| `icon` | `"claude"` (the installed Claude desktop app's icon), `false` (generic), or a path to an image (`.icns`, `.png`, `.jpg`, …) | `"claude"` |

`icon` is read from the **global** file only: macOS takes a notification's icon from the app that
posts it, so there is one icon per machine. Notification Center also keeps the first icon it sees
for an app forever, so **each icon gets its own app identity**: after you change `icon`, the next
popup builds a new *Claude Attention* app (the old one is removed), and macOS asks you to allow
notifications for it once — or, if it doesn't ask, allow it under System Settings → Notifications.
Until you do, popups fall back to the plain AppleScript notification. An outdated *Claude Attention*
entry may stay listed in Notifications settings; it is harmless.

A configured icon path that doesn't exist falls back to the generic icon and is logged to
`~/.claude/attention.log`.

The `"claude"` icon is read from your local `/Applications/Claude.app`; this plugin does not ship
it. Without the desktop app installed, `"claude"` falls back to the generic icon.

`CLAUDE_ATTENTION=off` in the environment silences everything for that session.

## Behaviour

- Fires on the `Stop` and `Notification` hooks.
- Skips subagent events, and `Stop` while background tasks are still running (the session will
  wake again on its own).
- Debounced: one signal per 8 seconds, so a manual call plus a hook never doubles up.
- Fail-open: it never blocks or breaks a session.
- Each decision is logged to `~/.claude/attention.log` (last 200 lines).

You can also signal from your own scripts:

```bash
printf '{"message":"Need a decision"}' | <plugin>/bin/attention.sh
```

## Development

```bash
tests/run.sh        # hook behaviour (dry run, no popups)
tests/notifier.sh   # app build, icon handling, rebuild avoidance (needs swiftc)
```

## License

MIT
