#!/bin/sh
# Copy a snapshot of condor-plugins into the crate that embeds it.
# Usage: scripts/sync-plugins.sh PATH_TO_CONDOR_PLUGINS_CHECKOUT
set -eu
src=$1
dest=$(dirname "$0")/../crates/condor_python/plugins
rm -rf "$dest"
mkdir -p "$dest"
cp "$src/schema.json" "$dest/"
cp -R "$src/python" "$dest/python"
cp -R "$src/tests" "$dest/tests"
rev=$(git -C "$src" rev-parse --verify -q HEAD 2>/dev/null || echo "uncommitted")
dirty=$(git -C "$src" status --porcelain 2>/dev/null | head -1)
{
  echo "Snapshot of https://github.com/mde-pach/condor-plugins"
  echo "revision: $rev${dirty:+ (with uncommitted changes)}"
  echo "Update with scripts/sync-plugins.sh; do not edit these files here."
} > "$dest/SOURCE"
echo "synced $rev into $dest"
