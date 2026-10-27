#!/bin/zsh
# SI Future HUD release build for teammates (run from anywhere: it works in
# the repository root): the same build as
# `pnpm run build:macos -- --bundle`, with this Mac's paths (home folder,
# Cargo registry, source folder) rewritten in the binaries so no user name or
# folder of the build machine ships. Low priority, one heavy job at a time.
# Fails unless the app carries a whole, valid ad-hoc signature (a downloaded
# copy with an incomplete one is reported "damaged" by macOS).
set -euo pipefail
cd "$(dirname "$0")/.."
export RUSTFLAGS="--remap-path-prefix=$HOME/.cargo=/cargo --remap-path-prefix=$PWD=/src --remap-path-prefix=$HOME=/home"
nice -n 15 pnpm run build:macos -- --bundle
app="target/release/bundle/macos/SI Future HUD.app"
dmg=$(ls -t target/release/bundle/dmg/*.dmg | head -1)

echo "== signature"
codesign --verify --deep --strict --verbose=2 "$app"
codesign -dv "$app" 2>&1 | grep -E 'Sealed Resources|Info.plist'
for helper in agent-hud-engine agent-hud-mcp; do
  codesign --verify --strict "$app/Contents/Resources/$helper"
done

echo "== a downloaded copy (quarantined), as a teammate gets it"
check=$(mktemp -d)
ditto "$app" "$check/SI Future HUD.app"
xattr -w com.apple.quarantine "0081;$(printf %x $(date +%s));Safari;" "$check/SI Future HUD.app"
codesign --verify --deep --strict "$check/SI Future HUD.app" && echo "signature still valid after download"
spctl -a -vv "$check/SI Future HUD.app" 2>&1 | head -3 || true
rm -rf "$check"

echo "== build machine's user name left in the bundle"
leaks=$(LC_ALL=C grep -rl "$USER" "$app/Contents" 2>/dev/null | wc -l | tr -d ' ' || true)
echo "$leaks"
ls -la "$dmg"
