# Plan: NTFYER-1 — ntfyer Rust core (PR A)

## Executive summary
- **Goal:** replace the bash + jq implementation of claude-attention with `ntfyer`, a
  harness-agnostic Rust CLI and library for macOS and Linux. The shell plugin stays untouched in
  this PR; PR B swaps the Claude Code adapter over to it.
- **Parent plan:** `~/.claude/plans/kind-napping-globe.md` (approved 2026-10-06).
- **Complexity:** medium. Most behaviour is already specified by the 52 shell test cases.
- **Risk:** low to medium. The macOS bundle logic is the delicate part, and it is a direct port
  of code proven on a real machine.
- **Branch:** `ntfyer-core`. RED baseline commit `8254a40`.

## Critical implementation standards
- Fail-open: `ntfyer signal` always exits 0, and every channel failure degrades silently and is
  logged (HARNESS-COMPONENT-MODEL invariant #1).
- Observation only: ntfyer signals the human and never writes into an agent loop (invariant #2).
- No `unwrap`/`expect` on fallible input in `src/` (tests may use them). No dead code. Clippy
  `-D warnings`.
- No new dependencies beyond `Cargo.toml` (clap, serde, serde_json, sha2; dev: tempfile).
- Tests already exist and are committed. Executors write implementation bodies only.

## Current state
| Path | Role |
|---|---|
| `plugin/bin/attention.sh` | today's hook entrypoint (bash, jq) |
| `plugin/bin/notifier.sh` | today's macOS bundle build (bash) |
| `plugin/notifier/ClaudeAttention.swift` | Swift notifier (copied to `notifier/Ntfyer.swift`) |
| `tests/run.sh`, `tests/notifier.sh` | 21 + 31 behaviour cases, ported to `tests/*.rs` |

## Target architecture
```
ntfyer (bin, src/main.rs: clap CLI, exit 64 on usage)
  └─ signal::run(envelope, context) ──┬─ config::resolve(global, project)   ~/.config/ntfyer/config.json, <project>/.ntfyer.json
                                      ├─ debounce::admit (per project)        ~/.local/state/ntfyer/debounce/<sha12>.last
                                      ├─ log::append (200 lines)              ~/.local/state/ntfyer/log
                                      ├─ bell (/dev/tty)
                                      ├─ sound::resolve + sound::player       afplay | pw-play/paplay/aplay
                                      └─ popup
                                           ├─ macOS: macos_app::notify → Ntfyer-<icon sha12>.app (one bundle id per icon) → osascript fallback
                                           └─ Linux: notify-send → gdbus
```
Pure functions (local nodes) are kept separate from process and filesystem orchestration (author
nodes), so each local node is a single function body checked by its own test.

## Implementation phases
**Phase 0 (done, `8254a40`):** skeleton, doc contracts, RED tests, and the author-owned
cross-cutting code (`src/popup/macos_app.rs`, `src/signal.rs`, `src/doctor.rs`, `src/main.rs`).

**Phase 1 (local cascade):** 24 single-function nodes, below. They run **sequentially** in
manifest order, because some build on earlier ones: `parse-sound` and `parse-icon` use
`expand-home`, and `resolve-config` uses both. Each `accept` does two things:
- It checks with `git diff --quiet ntfyer-red -- tests Cargo.toml` that the tests and the
  dependencies are byte-identical to the tagged RED baseline. A node cannot pass by editing its
  verifier.
- It requires the **exact** number of passing tests for that function's filter.

**Phase 2 (author):** `integration-green` brings `tests/cli.rs` (30 cases) and
`tests/macos_build.rs` (9 cases, which build for real) to green, plus clippy and fmt. The cargo
CI matrix (macos-15, ubuntu-24.04) lands in this PR.

**Validation:**
```bash
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
```

## Testing strategy
- Unit tests per pure function, in `tests/<module>.rs`.
- CLI behaviour through the real binary in dry-run mode (`NTFYER_DRY_RUN=1`), with an isolated
  HOME, `env_clear`, and expectations that hold on any OS (the default-sound line is computed
  with `sound::resolve`).
- macOS bundle tests build for real and never register or post (`NTFYER_REGISTER=0`,
  `NTFYER_NO_FALLBACK=1`).
- Linux runs every non-macOS test in CI (ubuntu-24.04, in this PR).
- **Adapter policy is not core behaviour.** The old suite's "SubagentStop skipped" and "Stop with
  running background tasks skipped" cases now belong to the Claude Code adapter, which maps them
  to `quiet: true`. They get ported with the adapter in PR B. The core tests `quiet` itself.

## Risks
| Risk | Level | Mitigation |
|---|---|---|
| Local model edits beyond its function | M | `forbid: new_deps`, single file per node, a scoped `change` text, and the full suite run at Phase 2 |
| Linux sound and popup untested on a real desktop | M | Dry-run tests in CI, plus a manual check reported as unverified if no desktop is available |
| gdbus string parsing of titles that look like numbers | L | notify-send is primary and gdbus only the fallback; documented |
| Rename resets macOS permission (new bundle id prefix) | L | Expected: the user allows "ntfyer" once (noted in the README in PR B) |

## Rollout
PR A merges with nothing user-visible changing. PR B switches the adapter. Release v0.1.0
publishes to crates.io and Homebrew. Then the family registration and this machine's migration
follow, per the parent plan.

## Execution manifest
```json
{
  "execution-manifest": [
    {
      "id": "paths-from-lookup",
      "files": [
        "src/paths.rs"
      ],
      "change": "Implement Paths::from_lookup per its doc comment. Replace only the unimplemented!(\"delegated: paths-from-lookup\") body; do not change signatures, parameter names, other functions or tests, and add no #[allow] attributes.",
      "accept": "git diff --quiet ntfyer-red -- tests Cargo.toml && cargo test -q --test paths paths_ 2>&1 | grep -qE 'test result: ok\\. 4 passed'",
      "forbid": [
        "new_deps"
      ],
      "local": true,
      "kind": "edit"
    },
    {
      "id": "expand-home",
      "files": [
        "src/config.rs"
      ],
      "change": "Implement expand_home per its doc comment. Replace only the unimplemented!(\"delegated: expand-home\") body; do not change signatures, parameter names, other functions or tests, and add no #[allow] attributes.",
      "accept": "git diff --quiet ntfyer-red -- tests Cargo.toml && cargo test -q --test config expand_home 2>&1 | grep -qE 'test result: ok\\. 1 passed'",
      "forbid": [
        "new_deps"
      ],
      "local": true,
      "kind": "edit"
    },
    {
      "id": "parse-sound",
      "files": [
        "src/config.rs"
      ],
      "change": "Implement parse_sound per its doc comment (use expand_home for ~; a relative file path joins onto config_dir). Replace only the unimplemented!(\"delegated: parse-sound\") body; do not change signatures, parameter names, other functions or tests, and add no #[allow] attributes.",
      "accept": "git diff --quiet ntfyer-red -- tests Cargo.toml && cargo test -q --test config parse_sound 2>&1 | grep -qE 'test result: ok\\. 1 passed'",
      "forbid": [
        "new_deps"
      ],
      "local": true,
      "kind": "edit"
    },
    {
      "id": "parse-icon",
      "files": [
        "src/config.rs"
      ],
      "change": "Implement parse_icon per its doc comment (use expand_home; relative paths join onto config_dir). Replace only the unimplemented!(\"delegated: parse-icon\") body; do not change signatures, parameter names, other functions or tests, and add no #[allow] attributes.",
      "accept": "git diff --quiet ntfyer-red -- tests Cargo.toml && cargo test -q --test config parse_icon 2>&1 | grep -qE 'test result: ok\\. 1 passed'",
      "forbid": [
        "new_deps"
      ],
      "local": true,
      "kind": "edit"
    },
    {
      "id": "read-layer",
      "files": [
        "src/config.rs"
      ],
      "change": "Implement read_layer per its doc comment with std::fs and serde_json. Replace only the unimplemented!(\"delegated: read-layer\") body; do not change signatures, parameter names, other functions or tests, and add no #[allow] attributes.",
      "accept": "git diff --quiet ntfyer-red -- tests Cargo.toml && cargo test -q --test config read_layer 2>&1 | grep -qE 'test result: ok\\. 1 passed'",
      "forbid": [
        "new_deps"
      ],
      "local": true,
      "kind": "edit"
    },
    {
      "id": "resolve-config",
      "files": [
        "src/config.rs"
      ],
      "change": "Implement resolve per its doc comment, using parse_sound and parse_icon; a project-layer sound that parses to Sound::File is ignored. Replace only the unimplemented!(\"delegated: resolve-config\") body; do not change signatures, parameter names, other functions or tests, and add no #[allow] attributes.",
      "accept": "git diff --quiet ntfyer-red -- tests Cargo.toml && cargo test -q --test config resolve_ 2>&1 | grep -qE 'test result: ok\\. 8 passed'",
      "forbid": [
        "new_deps"
      ],
      "local": true,
      "kind": "edit"
    },
    {
      "id": "parse-envelope",
      "files": [
        "src/envelope.rs"
      ],
      "change": "Implement parse per its doc comment with serde_json::from_str::<Envelope>. Replace only the unimplemented!(\"delegated: parse-envelope\") body; do not change signatures, parameter names, other functions or tests, and add no #[allow] attributes.",
      "accept": "git diff --quiet ntfyer-red -- tests Cargo.toml && cargo test -q --test envelope parse_envelope 2>&1 | grep -qE 'test result: ok\\. 2 passed'",
      "forbid": [
        "new_deps"
      ],
      "local": true,
      "kind": "edit"
    },
    {
      "id": "default-message",
      "files": [
        "src/envelope.rs"
      ],
      "change": "Implement default_message per its doc comment. Replace only the unimplemented!(\"delegated: default-message\") body; do not change signatures, parameter names, other functions or tests, and add no #[allow] attributes.",
      "accept": "git diff --quiet ntfyer-red -- tests Cargo.toml && cargo test -q --test envelope default_message 2>&1 | grep -qE 'test result: ok\\. 1 passed'",
      "forbid": [
        "new_deps"
      ],
      "local": true,
      "kind": "edit"
    },
    {
      "id": "project-label",
      "files": [
        "src/envelope.rs"
      ],
      "change": "Implement project_label per its doc comment. Replace only the unimplemented!(\"delegated: project-label\") body; do not change signatures, parameter names, other functions or tests, and add no #[allow] attributes.",
      "accept": "git diff --quiet ntfyer-red -- tests Cargo.toml && cargo test -q --test envelope project_label 2>&1 | grep -qE 'test result: ok\\. 1 passed'",
      "forbid": [
        "new_deps"
      ],
      "local": true,
      "kind": "edit"
    },
    {
      "id": "stamp-path",
      "files": [
        "src/debounce.rs"
      ],
      "change": "Implement stamp_path per its doc comment using sha2::Sha256 over the project path's bytes (as_os_str().as_encoded_bytes()). Replace only the unimplemented!(\"delegated: stamp-path\") body; do not change signatures, parameter names, other functions or tests, and add no #[allow] attributes.",
      "accept": "git diff --quiet ntfyer-red -- tests Cargo.toml && cargo test -q --test debounce stamp_path 2>&1 | grep -qE 'test result: ok\\. 1 passed'",
      "forbid": [
        "new_deps"
      ],
      "local": true,
      "kind": "edit"
    },
    {
      "id": "debounce-admit",
      "files": [
        "src/debounce.rs"
      ],
      "change": "Implement admit per its doc comment with std::fs. Replace only the unimplemented!(\"delegated: debounce-admit\") body; do not change signatures, parameter names, other functions or tests, and add no #[allow] attributes.",
      "accept": "git diff --quiet ntfyer-red -- tests Cargo.toml && cargo test -q --test debounce admit_ 2>&1 | grep -qE 'test result: ok\\. 3 passed'",
      "forbid": [
        "new_deps"
      ],
      "local": true,
      "kind": "edit"
    },
    {
      "id": "log-sanitize",
      "files": [
        "src/log.rs"
      ],
      "change": "Implement sanitize per its doc comment. Replace only the unimplemented!(\"delegated: log-sanitize\") body; do not change signatures, parameter names, other functions or tests, and add no #[allow] attributes.",
      "accept": "git diff --quiet ntfyer-red -- tests Cargo.toml && cargo test -q --test log sanitize 2>&1 | grep -qE 'test result: ok\\. 1 passed'",
      "forbid": [
        "new_deps"
      ],
      "local": true,
      "kind": "edit"
    },
    {
      "id": "log-append",
      "files": [
        "src/log.rs"
      ],
      "change": "Implement append per its doc comment with std::fs (OpenOptions append; trim through '<name>.<pid>' temp file + rename). Replace only the unimplemented!(\"delegated: log-append\") body; do not change signatures, parameter names, other functions or tests, and add no #[allow] attributes.",
      "accept": "git diff --quiet ntfyer-red -- tests Cargo.toml && cargo test -q --test log append_ 2>&1 | grep -qE 'test result: ok\\. 3 passed'",
      "forbid": [
        "new_deps"
      ],
      "local": true,
      "kind": "edit"
    },
    {
      "id": "sound-resolve",
      "files": [
        "src/sound.rs"
      ],
      "change": "Implement resolve per its doc comment. Replace only the unimplemented!(\"delegated: sound-resolve\") body; do not change signatures, parameter names, other functions or tests, and add no #[allow] attributes.",
      "accept": "git diff --quiet ntfyer-red -- tests Cargo.toml && cargo test -q --test sound resolve_ 2>&1 | grep -qE 'test result: ok\\. 3 passed'",
      "forbid": [
        "new_deps"
      ],
      "local": true,
      "kind": "edit"
    },
    {
      "id": "sound-player",
      "files": [
        "src/sound.rs"
      ],
      "change": "Implement player per its doc comment. Replace only the unimplemented!(\"delegated: sound-player\") body; do not change signatures, parameter names, other functions or tests, and add no #[allow] attributes.",
      "accept": "git diff --quiet ntfyer-red -- tests Cargo.toml && cargo test -q --test sound player_ 2>&1 | grep -qE 'test result: ok\\. 2 passed'",
      "forbid": [
        "new_deps"
      ],
      "local": true,
      "kind": "edit"
    },
    {
      "id": "lock-acquire",
      "files": [
        "src/lock.rs"
      ],
      "change": "Implement acquire per its doc comment: use the existing open() helper, then File::try_lock(); return Some(LockGuard { _file }) on success. Replace only the unimplemented!(\"delegated: lock-acquire\") body; do not change signatures, parameter names, other functions or tests, and add no #[allow] attributes.",
      "accept": "git diff --quiet ntfyer-red -- tests Cargo.toml && cargo test -q --test lock acquire_ 2>&1 | grep -qE 'test result: ok\\. 4 passed'",
      "forbid": [
        "new_deps"
      ],
      "local": true,
      "kind": "edit"
    },
    {
      "id": "notify-send-args",
      "files": [
        "src/popup/linux.rs"
      ],
      "change": "Implement notify_send_args per its doc comment (use APP_NAME). Replace only the unimplemented!(\"delegated: notify-send-args\") body; do not change signatures, parameter names, other functions or tests, and add no #[allow] attributes.",
      "accept": "git diff --quiet ntfyer-red -- tests Cargo.toml && cargo test -q --test popup_linux notify_send_args 2>&1 | grep -qE 'test result: ok\\. 1 passed'",
      "forbid": [
        "new_deps"
      ],
      "local": true,
      "kind": "edit"
    },
    {
      "id": "gdbus-args",
      "files": [
        "src/popup/linux.rs"
      ],
      "change": "Implement gdbus_args per its doc comment (use APP_NAME). Replace only the unimplemented!(\"delegated: gdbus-args\") body; do not change signatures, parameter names, other functions or tests, and add no #[allow] attributes.",
      "accept": "git diff --quiet ntfyer-red -- tests Cargo.toml && cargo test -q --test popup_linux gdbus_args 2>&1 | grep -qE 'test result: ok\\. 1 passed'",
      "forbid": [
        "new_deps"
      ],
      "local": true,
      "kind": "edit"
    },
    {
      "id": "icon-digest",
      "files": [
        "src/popup/macos.rs"
      ],
      "change": "Implement icon_digest per its doc comment using sha2::Sha256. Replace only the unimplemented!(\"delegated: icon-digest\") body; do not change signatures, parameter names, other functions or tests, and add no #[allow] attributes.",
      "accept": "git diff --quiet ntfyer-red -- tests Cargo.toml && cargo test -q --test popup_macos icon_digest 2>&1 | grep -qE 'test result: ok\\. 1 passed'",
      "forbid": [
        "new_deps"
      ],
      "local": true,
      "kind": "edit"
    },
    {
      "id": "bundle-identity",
      "files": [
        "src/popup/macos.rs"
      ],
      "change": "Implement identity per its doc comment using ID_PREFIX. Replace only the unimplemented!(\"delegated: bundle-identity\") body; do not change signatures, parameter names, other functions or tests, and add no #[allow] attributes.",
      "accept": "git diff --quiet ntfyer-red -- tests Cargo.toml && cargo test -q --test popup_macos identity 2>&1 | grep -qE 'test result: ok\\. 1 passed'",
      "forbid": [
        "new_deps"
      ],
      "local": true,
      "kind": "edit"
    },
    {
      "id": "source-digest",
      "files": [
        "src/popup/macos.rs"
      ],
      "change": "Implement source_digest per its doc comment (sha256 over SWIFT_SOURCE bytes then env!(\"CARGO_PKG_VERSION\") bytes). Replace only the unimplemented!(\"delegated: source-digest\") body; do not change signatures, parameter names, other functions or tests, and add no #[allow] attributes.",
      "accept": "git diff --quiet ntfyer-red -- tests Cargo.toml && cargo test -q --test popup_macos source_digest 2>&1 | grep -qE 'test result: ok\\. 1 passed'",
      "forbid": [
        "new_deps"
      ],
      "local": true,
      "kind": "edit"
    },
    {
      "id": "info-plist",
      "files": [
        "src/popup/macos.rs"
      ],
      "change": "Implement info_plist per its doc comment as a format! string with the XML header and DOCTYPE. Replace only the unimplemented!(\"delegated: info-plist\") body; do not change signatures, parameter names, other functions or tests, and add no #[allow] attributes.",
      "accept": "git diff --quiet ntfyer-red -- tests Cargo.toml && cargo test -q --test popup_macos info_plist 2>&1 | grep -qE 'test result: ok\\. 1 passed'",
      "forbid": [
        "new_deps"
      ],
      "local": true,
      "kind": "edit"
    },
    {
      "id": "icon-source",
      "files": [
        "src/popup/macos.rs"
      ],
      "change": "Implement icon_source per its doc comment. Replace only the unimplemented!(\"delegated: icon-source\") body; do not change signatures, parameter names, other functions or tests, and add no #[allow] attributes.",
      "accept": "git diff --quiet ntfyer-red -- tests Cargo.toml && cargo test -q --test popup_macos icon_source 2>&1 | grep -qE 'test result: ok\\. 1 passed'",
      "forbid": [
        "new_deps"
      ],
      "local": true,
      "kind": "edit"
    },
    {
      "id": "osascript-args",
      "files": [
        "src/popup/macos.rs"
      ],
      "change": "Implement osascript_args per its doc comment. Replace only the unimplemented!(\"delegated: osascript-args\") body; do not change signatures, parameter names, other functions or tests, and add no #[allow] attributes.",
      "accept": "git diff --quiet ntfyer-red -- tests Cargo.toml && cargo test -q --test popup_macos osascript_args 2>&1 | grep -qE 'test result: ok\\. 1 passed'",
      "forbid": [
        "new_deps"
      ],
      "local": true,
      "kind": "edit"
    },
    {
      "id": "integration-green",
      "files": [
        "src/signal.rs",
        "src/popup/macos_app.rs",
        "src/main.rs",
        "src/doctor.rs"
      ],
      "change": "Author-owned: once every local node has landed, make tests/cli.rs and tests/macos_build.rs pass, fixing the cross-cutting code if needed; clippy -D warnings clean.",
      "accept": "cargo test -q && cargo clippy -q --all-targets -- -D warnings",
      "local": false
    }
  ]
}
```

## Multi-agent review
Four independent reviewers ran in parallel: rust-pro (architecture), security-auditor,
test-automator, and a fresh `code-validate-plan` run (verdict: pass with changes). Every finding
was weighed on evidence. Accepted and applied before tagging the RED baseline:

| Finding | Source | Change |
|---|---|---|
| `is_none_or` above the declared MSRV | rust-pro, validator | MSRV raised to 1.89 (needed for `File::try_lock` anyway) |
| pid-dir lock: race between `mkdir` and the pid write, and a takeover race | security, rust-pro | replaced with `flock` (`File::try_lock`), which the kernel releases on death, so it cannot go stale |
| Freshness not re-checked after taking the lock | rust-pro | re-checked; a concurrent winner's bundle is never swapped out |
| A project `.ntfyer.json` can make the machine play any file, and `-x/y` reached the player as an option | security | project layer may not name a sound file; file paths are always absolute (relative paths join the config dir) |
| gdbus could read a title or body starting with `-` as an option | security | `--` before the positional arguments |
| Log let ESC and other control sequences through | security | every control character plus U+2028/2029 is replaced |
| `signal` exited 64 on an unknown flag | validator | `signal` exits 0 on usage errors (fail-open); test added |
| Accept could pass by editing tests, and needed only 1 passing test | test-automator | accept now diffs `tests/` against the `ntfyer-red` tag and requires the exact count |
| `paths` accept had no filter | test-automator | filter `paths_` (the `project_config` test renamed) |
| Default-sound expectation computed by the code under test | test-automator | hardcoded per OS in the test |
| Missing cases: sound off alone, bell off alone, null project override, notifier empty title | test-automator | added |
| One 25-step macOS test where the first failure hid the rest | test-automator | split into 7 independent tests |
| Timing bound 3 s (flaky under load) | test-automator | 3.5 s against a 4 s fake compile |
| Missing CONTRACT.md; no cargo CI in PR A | validator | both added in Phase 0 |
| Doctor degraded and build-on-Linux untested | validator | tests added |

Accepted as design and not changed: same-user attacks on `$HOME` directories, trusting PATH for
tools, a project file being able to silence ntfyer (documented), and detached builds piling up
while one is already running (each exits on the lock).

**Status:** Approved for execution. **Date:** 2026-10-06.
