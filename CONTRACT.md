# ntfyer — contract

ntfyer gets a **human's** attention: popup, chime and terminal bell, on macOS and Linux. It never
writes anything back into the agent or script that called it.

## Front doors

| Door | Shape |
|---|---|
| CLI | `ntfyer signal \| build \| doctor \| config path` |
| Library | `ntfyer::signal::run(&Envelope, &Context, &mut dyn Write) -> String` (the verdict) |

There is no container image. A container cannot reach the host's notification centre, so an
image would have nothing to signal with.

## `ntfyer signal`

```
ntfyer signal [--message M] [--title T] [--project DIR] [--event NAME] [--json]
```

`--json` reads an envelope from stdin. Flags override envelope fields.

```json
{"message": "…", "title": "…", "project": "/path", "event": "stop", "session": "…", "quiet": false}
```

- Every field is optional. Input that is malformed or has a wrongly typed field counts as `{}`.
- `project` selects `<project>/.ntfyer.json` and the debounce key. It defaults to the working
  directory, and its last path component is the default title.
- `message` defaults to "Finished and waiting for you." when `event` is `stop`, and to "Needs
  your attention." otherwise.
- `quiet: true` logs the event without signalling. Harness adapters use it for events that must
  stay silent, such as a subagent finishing.

**Exit code: always 0.** This includes malformed input, unknown flags and a broken environment.
A notifier must never break its caller.

## Other commands

| Command | Exit |
|---|---|
| `ntfyer build` | 0 ok or up to date; 1 failed (reason on stderr). On macOS it builds the notifier app; on Linux it does nothing |
| `ntfyer doctor [--format text\|json]` | 0 healthy (a popup path works); 1 degraded. JSON: `{"schema_version":"1","status":"ok","body":{…}}` |
| `ntfyer config path` | 0 |
| any usage error outside `signal` | 64 |

## Configuration

| File | Scope |
|---|---|
| `$XDG_CONFIG_HOME/ntfyer/config.json` (default `~/.config/ntfyer/config.json`) | global |
| `<project>/.ntfyer.json` | project; overrides the global file key by key |

| Key | Values | Default |
|---|---|---|
| `enabled` | `false` silences everything | `true` |
| `bell` | terminal bell | `true` |
| `sound` | `false`; `true` (OS default); a system sound name; a file path (`~` allowed; relative paths are resolved against the global config directory). **Only the global file may name a sound file.** | `true` |
| `popup` | desktop popup | `true` |
| `icon` | `"claude"` (the Claude desktop app's icon where installed); `false`; an image path. **Global file only** | `"claude"` |

A missing file, a malformed file, a missing key and `null` all keep the default. A project file
can silence ntfyer, or pick a different system sound for that project. `NTFYER=off` silences
everything.

## Behaviour guarantees

- **Fail-open.** An absent backend, a missing tool or a broken config degrades a channel. It never
  fails the call.
- **Debounce.** At most one signal per project per 8 seconds.
- **Log.** Each decision is one line in `$XDG_STATE_HOME/ntfyer/log` (default
  `~/.local/state/ntfyer/log`), with control characters stripped. The log keeps the last 200 lines.
- **No compile inside `signal`.** On macOS, if the notifier binary is missing or stale, `signal`
  starts a detached `ntfyer build` and this popup falls back to `osascript`.
- **One app identity per icon (macOS).** Notification Center keeps the first icon it sees for a
  bundle id, so each icon gets its own bundle (`dev.ntfyer.notifier.<sha12>`). Changing the icon
  means allowing notifications for the new bundle once.

## Test seams

These environment variables exist for tests and are not a stable interface: `NTFYER_DRY_RUN=1`
(print channels instead of firing them), `NTFYER_NO_FALLBACK=1`, `NTFYER_REGISTER=0`,
`NTFYER_DEFAULT_ICON=<path>`.

## Versioning

While the version is 0.x, the minor number is the breaking position. The crate version, git tag and
Homebrew formula carry the same number.
