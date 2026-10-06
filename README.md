# ntfyer

[![CI](https://github.com/Barnett-Studios/ntfyer/actions/workflows/ci.yml/badge.svg)](https://github.com/Barnett-Studios/ntfyer/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/ntfyer)](https://crates.io/crates/ntfyer)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

**Interaction plane · Active** — under development; the surface still moves.
See the [component map](https://github.com/Barnett-Studios) for how this fits the rest.

**Get a human's attention when an agent — or any script — needs them.**

`ntfyer signal` fires a desktop popup, a chime and the terminal bell, on macOS and Linux. Any agent
harness, git hook or shell script can call it: it takes flags or a small JSON envelope on stdin,
and it is **fail-open by construction** — it always exits 0, and a missing backend only means a
channel does not fire. It signals the human and never writes anything back into the caller.

> Part of the Barnett Studios agentic-harness toolkit → cxpak · commitward · **ntfyer** · …

## Install

```sh
brew tap Barnett-Studios/tap && brew install ntfyer     # or: cargo install ntfyer
ntfyer doctor                                           # what this machine can do
```

- **macOS** needs the Xcode Command Line Tools (`xcode-select --install`): the popup comes from a
  tiny notifier app ntfyer compiles on first use. The first popup asks you to allow
  notifications for **ntfyer** (or allow it under System Settings → Notifications).
- **Linux** uses `notify-send` (libnotify) or `gdbus`, and `pw-play` / `paplay` / `aplay` for the
  chime.

## Use

```sh
ntfyer signal --message "Build finished"                  # title defaults to the directory name
ntfyer signal --title "CI" --message "Red" --project ~/src/app
printf '{"message":"Needs input","event":"notification"}' | ntfyer signal --json
```

The envelope (all fields optional): `message`, `title`, `project`, `event`, `session`, `quiet`.
Full contract — commands, exit codes, guarantees: [CONTRACT.md](CONTRACT.md).

### Claude Code

```
/plugin marketplace add Barnett-Studios/ntfyer
/plugin install ntfyer@ntfyer
```

The plugin is a thin adapter ([`plugin/bin/claude-hook.sh`](plugin/bin/claude-hook.sh)): it maps
the `Stop` and `Notification` hook payloads to an envelope and calls `ntfyer signal --json`. It
stays quiet for subagents and for a `Stop` while background tasks are still running. Needs the
`ntfyer` binary and `jq`.

**Coming from `claude-attention`?** That plugin is now this one. Move over with:

```
/plugin uninstall claude-attention@claude-attention
/plugin marketplace remove claude-attention
/plugin marketplace add Barnett-Studios/ntfyer
/plugin install ntfyer@ntfyer
```

and move `~/.claude/attention.json` to `~/.config/ntfyer/config.json` (same keys). macOS asks once
to allow notifications for **ntfyer**.

### Git hooks and scripts

See [`examples/git-hooks/post-merge`](examples/git-hooks/post-merge). Any long-running command
works the same way: `make release; ntfyer signal --message "release done"`.

## Configure

`~/.config/ntfyer/config.json` (or `$XDG_CONFIG_HOME/ntfyer/config.json`), overridden key by key by
`<project>/.ntfyer.json`. Every key is optional; `null` or a malformed file keeps the defaults.

```json
{ "enabled": true, "bell": true, "sound": "Pop", "popup": true, "icon": "claude" }
```

| Key | Values | Default |
|---|---|---|
| `enabled` | `false` silences everything | `true` |
| `bell` | terminal bell (only when there is a controlling terminal) | `true` |
| `sound` | `false`, `true` (OS default), a system sound name (macOS: `Pop`, `Hero`, …; Linux: freedesktop names such as `complete`), or a file path (`~/x.wav`, `./x.wav`) — files from the global config only | `true` |
| `popup` | desktop popup | `true` |
| `icon` | `"claude"` (the Claude desktop app's icon, if installed), `false`, or an image path — global config only | `"claude"` |

On macOS each icon gets its own notifier identity (Notification Center caches icons per app
forever), so changing `icon` asks you to allow notifications once more. `NTFYER=off` silences
everything. Decisions are logged to `~/.local/state/ntfyer/log`.

## License

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) at your option.
Unless you explicitly state otherwise, any contribution you intentionally submit for
inclusion in the work shall be dual-licensed as above, without any additional terms.

---

Built by [Barnett Studios](https://barnett-studios.com/) — part of the agentic-harness
toolkit: [cxpak](https://github.com/Barnett-Studios/cxpak) ·
[commitward](https://github.com/Barnett-Studios/commitward) ·
[cascadr](https://github.com/Barnett-Studios/cascadr) ·
[abproof](https://github.com/Barnett-Studios/abproof) ·
[cordon](https://github.com/Barnett-Studios/cordon) ·
[slicr](https://github.com/Barnett-Studios/slicr) · **ntfyer**.
