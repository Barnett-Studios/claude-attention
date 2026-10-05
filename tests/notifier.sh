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

echo '{"icon":false}' > "$HOME/.claude/attention.json"
CLAUDE_ATTENTION_DEFAULT_ICON="$generic" "$nt" build >/dev/null
expect "icon false means the generic icon" '[[ ! -e "$(icns)" && "$(bundle_id)" == "$generic_id" ]]'

rm -rf "$(app)"
"$nt" build >/dev/null
expect "a deleted bundle is rebuilt whole" '[[ -x "$(bin)" && -f "$(plist)" ]] && codesign --verify "$(app)" 2>/dev/null'

# --- lock recovery ---------------------------------------------------------------------------
mkdir -p "$dir/.build.lock"; echo 999999 > "$dir/.build.lock/pid"; rm -rf "$(app)"
"$nt" build >/dev/null 2>&1 || true
expect "a lock left by a dead process is recovered" '[[ -x "$(bin)" && ! -e "$dir/.build.lock" ]]'

mkdir -p "$dir/.build.lock"; echo $$ > "$dir/.build.lock/pid"; rm -rf "$(app)"
expect "a lock held by a live process is respected" '! "$nt" build >/dev/null 2>&1'
rm -rf "$dir/.build.lock"; "$nt" build >/dev/null

# --- notify never compiles inside a hook --------------------------------------------------------
fake="$d/fakebin"; mkdir -p "$fake"
printf '#!/bin/sh\ntouch "%s/swiftc-ran"\nsleep 4\nexit 1\n' "$d" > "$fake/swiftc"; chmod +x "$fake/swiftc"
rm -rf "$dir/build"
start=$(date +%s)
expect "notify with no compiled binary fails fast" '! PATH="$fake:$PATH" "$nt" notify t m >/dev/null 2>&1'
expect "...within 2 seconds" '(( $(date +%s) - start <= 2 ))'
for _ in 1 2 3 4 5 6 7 8 9 10; do [[ -e "$d/swiftc-ran" ]] && break; perl -e 'select(undef,undef,undef,0.3)'; done
expect "...and starts the compile in the background" '[[ -e "$d/swiftc-ran" ]]'
# wait out the background build (the fake compile sleeps 4s) before building for real
for _ in $(seq 1 40); do [[ -e "$dir/.build.lock" ]] || break; perl -e 'select(undef,undef,undef,0.25)'; done
"$nt" build >/dev/null

# --- missing Command Line Tools -----------------------------------------------------------------
printf '#!/bin/sh\nexit 2\n' > "$fake/xcode-select"; chmod +x "$fake/xcode-select"
rm -f "$d/swiftc-ran"; rm -rf "$dir/build"
expect "build without Command Line Tools fails" '! PATH="$fake:$PATH" "$nt" build >/dev/null 2>&1'
expect "...without invoking the swiftc shim" '[[ ! -e "$d/swiftc-ran" ]]'
"$nt" build >/dev/null

# --- relative icon paths resolve against ~/.claude ------------------------------------------------
cp "$d/a.png" "$HOME/.claude/rel.png"
echo '{"icon":"rel.png"}' > "$HOME/.claude/attention.json"
(cd "$d" && "$nt" build >/dev/null)
rel_id="$(bundle_id)"
expect "relative icon path resolves against ~/.claude" '[[ -e "$(icns)" ]]'
(cd / && "$nt" build >/dev/null)
expect "...the same from any working directory" '[[ "$(bundle_id)" == "$rel_id" ]]'

expect "notify refuses empty title" '! "$nt" notify "" "m" >/dev/null 2>&1'

printf '%d passed, %d failed\n' "$pass" "$fail"
[[ "$fail" -eq 0 ]]
