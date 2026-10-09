#!/usr/bin/env bash
# Cuts a GitHub release for one plugin and prints the entry to submit to the plugin store.
#
#   scripts/release.sh plugins/images-tools
#
# Requires bun, the plugin's own toolchain (a Rust wasm32 target for plugins with a backend) and an authenticated `gh`.
set -euo pipefail

dir="${1:-}"
if [[ -z "$dir" || ! -f "$dir/package.json" ]]; then
  echo "usage: scripts/release.sh <plugin-dir>" >&2
  exit 2
fi

dir="$(cd "$dir" && pwd)"
root="$(git -C "$dir" rev-parse --show-toplevel)"

git -C "$root" fetch --quiet origin main
if [[ -n "$(git -C "$root" status --porcelain)" ]]; then
  echo "error: the working tree is dirty; a release must be built from committed code" >&2
  exit 1
fi
if ! git -C "$root" merge-base --is-ancestor HEAD origin/main; then
  echo "error: HEAD is not on origin/main; promote develop into main before releasing" >&2
  exit 1
fi
name="$(basename "$dir")"
version="$(node -p "require('$dir/package.json').version")"
tag="$(node -p "require('$dir/package.json').jensen.tag || '$version'")"
repo="$(node -p "require('$dir/package.json').jensen.repo")"

if [[ ! -f "$dir/README.md" ]]; then
  echo "error: $name has no README.md; every published plugin must document itself" >&2
  exit 1
fi

echo "==> installing and building $name"
(cd "$dir" && bun install --silent && bun run --silent build)

echo "==> publishing $name"
(cd "$dir" && bunx jensen-plugin publish --no-build)

if gh release view "$tag" --repo "$repo" >/dev/null 2>&1; then
  echo "==> release $tag exists, replacing its assets"
  gh release upload "$tag" "$dir"/release/* --repo "$repo" --clobber
else
  echo "==> creating release $tag"
  gh release create "$tag" "$dir"/release/* \
    --repo "$repo" \
    --target "$(git -C "$dir" rev-parse HEAD)" \
    --title "$name v$version" \
    --notes "See plugins/$name/README.md" \
    --latest=false
fi

echo
echo "==> release is live:"
echo "    https://github.com/$repo/releases/tag/$tag"
echo
echo "==> submit the printed entry to the store:"
echo "    https://github.com/jensen-org/plugins-store  (add entries/$(node -p "require('$dir/package.json').jensen.id").json)"
