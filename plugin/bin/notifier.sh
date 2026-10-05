#!/usr/bin/env bash
# notifier.sh — builds and drives ClaudeAttention-<id>.app, the tiny app that posts the popup. macOS
# takes a notification's icon from the app that posts it, and Notification Center keeps the first icon
# it saw for a bundle id for good (restarting its daemons does not clear it). So each icon gets its own
# app identity, derived from the icon's content: changing the icon builds a new bundle (which macOS
# asks to allow once) and removes the old one. One icon per machine, read from ~/.claude/attention.json.
#
#   notifier.sh build                          compile / re-icon only what changed; idempotent
#   notifier.sh notify <title> <message> [group]
#       exit 0 posted; non-zero means the caller should fall back (not built, not allowed, failed)
#
# "icon": "claude" (default) uses the installed Claude desktop app's icon, which is read from the
# local install and never shipped with this plugin; a path uses that image (.icns as-is, any other
# image format sips reads is converted). No usable icon → the generic app icon.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
src="$here/../notifier/ClaudeAttention.swift"
dir="$HOME/Library/Application Support/claude-attention"
cache="$dir/build"         # compiled binary, shared by every icon's bundle
stamp="$cache/src-digest"  # outside the bundles: anything inside one breaks its signature
id_prefix="dev.claude-attention.notifier"
default_icon="${CLAUDE_ATTENTION_DEFAULT_ICON:-/Applications/Claude.app/Contents/Resources/electron.icns}"
lsregister=/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister

# build scratch state, global so the EXIT trap still sees it after build() returns
lock="$dir/.build.lock"
tmp=""
cleanup() {
  if [[ -n "$tmp" ]]; then rm -rf "$tmp"; rmdir "$lock" 2>/dev/null || true; fi
}

die() { printf 'notifier: %s\n' "$*" >&2; exit 1; }

icon_source() {
  local v=""
  if [[ -f "$HOME/.claude/attention.json" ]]; then
    v="$(jq -r 'if type == "object" and has("icon") then .icon | tostring else empty end' \
      "$HOME/.claude/attention.json" 2>/dev/null || true)"
  fi
  case "$v" in ""|true|claude) v="$default_icon" ;; false) v="" ;; esac
  v="${v/#\~/$HOME}"
  [[ -n "$v" ]] || return 0
  if [[ -f "$v" ]]; then printf '%s' "$v"
  elif [[ "$v" != "$default_icon" ]]; then
    # a configured icon that is not there must not fail silently: say so where users look
    [[ -d "$HOME/.claude" ]] && printf '%s notifier: icon not found: %s (using the generic icon)\n' \
      "$(date +%FT%T)" "$v" >> "$HOME/.claude/attention.log"
  fi
}

digest() { if [[ -n "$1" ]]; then shasum "$1" | cut -d' ' -f1; else echo none; fi; }

# write_plist <app> <bundle-id> <with-icon:0|1>
write_plist() {
  local icon_key=""
  [[ "$3" == 1 ]] && icon_key="<key>CFBundleIconFile</key><string>AppIcon</string>"
  cat > "$1/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleIdentifier</key><string>$2</string>
<key>CFBundleName</key><string>Claude Attention</string>
<key>CFBundleDisplayName</key><string>Claude Attention</string>
<key>CFBundleExecutable</key><string>ClaudeAttention</string>
$icon_key
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleShortVersionString</key><string>1.0</string>
<key>CFBundleVersion</key><string>1</string>
<key>LSUIElement</key><true/>
</dict></plist>
PLIST
}

# app_for <icon-digest> → the bundle path for that icon
app_for() { printf '%s/ClaudeAttention-%s.app' "$dir" "${1:0:12}"; }

# build → sets the global $app to the bundle for the configured icon, building what is missing
app=""
build() {
  [[ -f "$src" ]] || die "missing source $src"
  local icon want_src want_icon have_src="" old
  icon="$(icon_source)"
  want_icon="$(digest "$icon")"
  app="$(app_for "$want_icon")"
  # the build recipe is part of the source: a changed notifier.sh rebuilds the binary and bundle
  want_src="$(cat "$src" "${BASH_SOURCE[0]}" | shasum | cut -d' ' -f1)"
  [[ -f "$stamp" ]] && have_src="$(cat "$stamp")"
  local fresh_bin=0 fresh_app=0
  [[ -x "$cache/ClaudeAttention" && "$have_src" == "$want_src" ]] || fresh_bin=1
  [[ -x "$app/Contents/MacOS/ClaudeAttention" && -f "$app/Contents/Info.plist" ]] || fresh_app=1
  (( fresh_bin || fresh_app )) || return 0

  mkdir -p "$cache"
  mkdir "$lock" 2>/dev/null || die "another build is running"
  tmp="$(mktemp -d)"
  trap cleanup EXIT

  if (( fresh_bin )); then
    command -v swiftc >/dev/null 2>&1 || die "swiftc not found (install Xcode Command Line Tools)"
    swiftc -O "$src" -o "$tmp/ClaudeAttention" || die "compile failed"
    mv "$tmp/ClaudeAttention" "$cache/ClaudeAttention"
    printf '%s' "$want_src" > "$stamp"
    echo compiled
  else
    echo "icon updated"
  fi

  # assemble the bundle off to the side, then swap it in whole
  local stage="$tmp/ClaudeAttention.app" with_icon=0
  mkdir -p "$stage/Contents/MacOS" "$stage/Contents/Resources"
  cp "$cache/ClaudeAttention" "$stage/Contents/MacOS/ClaudeAttention"
  if [[ -n "$icon" ]]; then
    if [[ "$icon" == *.icns ]]; then cp "$icon" "$stage/Contents/Resources/AppIcon.icns"
    else
      sips -z 1024 1024 -s format icns "$icon" --out "$stage/Contents/Resources/AppIcon.icns" \
        >/dev/null 2>&1 || die "cannot convert icon $icon"
    fi
    with_icon=1
  fi
  write_plist "$stage" "$id_prefix.${want_icon:0:12}" "$with_icon"
  codesign --force -s - "$stage" >/dev/null 2>&1 || die "codesign failed"
  rm -rf "$app"
  mv "$stage" "$app"

  # retire every other icon's bundle, so only the current one is registered and listed
  for old in "$dir"/ClaudeAttention-*.app; do
    [[ -d "$old" && "$old" != "$app" ]] || continue
    if [[ "${CLAUDE_ATTENTION_REGISTER:-1}" != 0 ]]; then "$lsregister" -u "$old" >/dev/null 2>&1 || true; fi
    rm -rf "$old"
  done
  if [[ "${CLAUDE_ATTENTION_REGISTER:-1}" != 0 ]]; then "$lsregister" -f "$app" >/dev/null 2>&1 || true; fi
}

notify() {
  [[ $# -ge 2 && -n "$1" ]] || die "usage: notifier.sh notify <title> <message> [group]"
  build >/dev/null
  local rc=0
  "$app/Contents/MacOS/ClaudeAttention" "$1" "$2" "${3:-}" || rc=$?
  if (( rc == 3 )); then
    # not allowed yet: a LaunchServices launch registers the app with Notification Center so the
    # user can allow it; this popup itself still goes through the caller's fallback
    open -g -n -a "$app" --args "$1" "$2" "${3:-}" >/dev/null 2>&1 || true
  fi
  return "$rc"
}

case "${1:-}" in
  build)  build ;;
  notify) shift; notify "$@" ;;
  *)      die "usage: notifier.sh build | notify <title> <message> [group]" ;;
esac
