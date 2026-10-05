#!/usr/bin/env bash
# Build tests for plugin/bin/notifier.sh: the app bundle, the configurable icon (one app identity per
# icon), and rebuild avoidance. Builds for real (needs swiftc) into an isolated HOME; never posts.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
nt="$here/../plugin/bin/notifier.sh"
pass=0 fail=0
ok() { pass=$((pass + 1)); }
bad() { fail=$((fail + 1)); printf 'FAIL %s\n' "$1"; }
expect() { if eval "$2"; then ok; else bad "$1"; fi; }

d="$(mktemp -d)"
trap 'rm -rf "$d"' EXIT
mkdir -p "$d/home/.claude"
generic=/System/Library/CoreServices/CoreTypes.bundle/Contents/Resources/GenericApplicationIcon.icns
sips -s format png "$generic" --out "$d/a.png" >/dev/null
sips -s format png -r 90 "$generic" --out "$d/b.png" >/dev/null
# never register the throwaway bundles with LaunchServices: a stale registration of a deleted copy
# confuses Notification Center about which bundle a notification came from
export HOME="$d/home" CLAUDE_ATTENTION_DEFAULT_ICON="$d/none.icns" CLAUDE_ATTENTION_REGISTER=0
dir="$HOME/Library/Application Support/claude-attention"

apps() { find "$dir" -maxdepth 1 -name 'ClaudeAttention-*.app' 2>/dev/null; }
app() { apps | head -1; }
bin() { printf '%s' "$(app)/Contents/MacOS/ClaudeAttention"; }
plist() { printf '%s' "$(app)/Contents/Info.plist"; }
bundle_id() { /usr/libexec/PlistBuddy -c "Print :CFBundleIdentifier" "$(plist)" 2>/dev/null || true; }
icns() { printf '%s' "$(app)/Contents/Resources/AppIcon.icns"; }
sum() { shasum "$1" | cut -d' ' -f1; }

"$nt" build >/dev/null
expect "build creates exactly one app" '[[ $(apps | wc -l) -eq 1 ]]'
expect "build creates the executable" '[[ -x "$(bin)" ]]'
expect "bundle identifier is namespaced" '[[ "$(bundle_id)" == dev.claude-attention.notifier.* ]]'
expect "bundle is signed" 'codesign --verify "$(app)" 2>/dev/null'
expect "no icon when the default icon is absent" '[[ ! -e "$(icns)" ]]'
expect "usage error without arguments" '[[ $("$(bin)" >/dev/null 2>&1; echo $?) -eq 2 ]]'
generic_id="$(bundle_id)"

echo "{\"icon\":\"$d/a.png\"}" > "$HOME/.claude/attention.json"
out="$("$nt" build)"
expect "icon change does not recompile" '[[ "$out" == "icon updated" ]]'
expect "png icon is converted to icns" '[[ "$(file -b "$(icns)")" == *"Mac OS X icon"* ]]'
expect "icon change gets a new bundle identity" '[[ "$(bundle_id)" != "$generic_id" ]]'
expect "the old bundle is removed" '[[ $(apps | wc -l) -eq 1 ]]'
expect "new bundle is signed" 'codesign --verify "$(app)" 2>/dev/null'
first="$(sum "$(icns)")"; first_id="$(bundle_id)"

out="$("$nt" build)"
expect "unchanged build does nothing" '[[ -z "$out" && "$(sum "$(icns)")" == "$first" ]]'

echo "{\"icon\":\"$d/b.png\"}" > "$HOME/.claude/attention.json"
"$nt" build >/dev/null
expect "a different icon replaces the old one" '[[ "$(sum "$(icns)")" != "$first" ]]'
expect "a different icon gets a different identity" '[[ "$(bundle_id)" != "$first_id" ]]'

echo "{\"icon\":\"$d/a.png\"}" > "$HOME/.claude/attention.json"
"$nt" build >/dev/null
expect "returning to an icon returns to its identity" '[[ "$(bundle_id)" == "$first_id" ]]'

echo "{\"icon\":\"$generic\"}" > "$HOME/.claude/attention.json"
"$nt" build >/dev/null
expect "icns icon is used as-is" '[[ "$(sum "$(icns)")" == "$(sum "$generic")" ]]'

echo '{"icon":"claude"}' > "$HOME/.claude/attention.json"
CLAUDE_ATTENTION_DEFAULT_ICON="$generic" "$nt" build >/dev/null
expect "\"claude\" uses the default icon" '[[ "$(sum "$(icns)")" == "$(sum "$generic")" ]]'

echo "{\"icon\":\"$d/missing.png\"}" > "$HOME/.claude/attention.json"
"$nt" build >/dev/null
expect "missing icon file falls back to no icon" '[[ ! -e "$(icns)" && "$(bundle_id)" == "$generic_id" ]]'
expect "missing icon file is logged" 'grep -q "icon not found: $d/missing.png" "$HOME/.claude/attention.log"'

rm -rf "$(app)"
"$nt" build >/dev/null
expect "a deleted bundle is rebuilt whole" '[[ -x "$(bin)" && -f "$(plist)" ]] && codesign --verify "$(app)" 2>/dev/null'

expect "notify refuses empty title" '! "$nt" notify "" "m" >/dev/null 2>&1'

printf '%d passed, %d failed\n' "$pass" "$fail"
[[ "$fail" -eq 0 ]]
