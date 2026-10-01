#!/usr/bin/env bash
# Publishes one platform's build to GitHub Release $TAG. The Android and Windows workflows both call it, in either
# order: the first creates the release (and, on a manual run, the tag at $TARGET_SHA); the second adds its files and
# its section of the notes.
#
#   TAG=v0.1.0 TARGET_SHA=<commit> GITHUB_REPOSITORY=owner/repo GH_TOKEN=… \
#     publish-release.sh PLATFORM ABOUT PRERELEASE(true|false) DIR
#
# DIR holds the files to attach, including one SHA256SUMS*.txt.
set -euo pipefail

platform="$1" about="$2" prerelease="$3" dir="$4"
: "${TAG:?}" "${TARGET_SHA:?}" "${GITHUB_REPOSITORY:?}"
repo=(--repo "$GITHUB_REPOSITORY")
sums="$(cat "$dir"/SHA256SUMS*.txt)"
notes="$(mktemp)"
# shellcheck disable=SC2016  # the backticks are a literal Markdown code fence
printf '### %s\n\n%s\n\nSHA-256:\n```\n%s\n```\n' "$platform" "$about" "$sums" > "$notes"

add_to_existing() {
  gh release upload "$TAG" "$dir"/* "${repo[@]}" --clobber
  body="$(gh release view "$TAG" "${repo[@]}" --json body --jq .body)"
  # Add this platform's notes once (a re-run replaces the files but doesn't repeat the section).
  if ! grep -qxF "### $platform" <<< "$body" && ! grep -qF "$(head -1 <<< "$sums")" <<< "$body"; then
    { printf '%s\n\n' "$body"; cat "$notes"; } > "$notes.all"
    gh release edit "$TAG" "${repo[@]}" --notes-file "$notes.all"
  fi
  echo "Added $platform files to release $TAG"
}

if gh release view "$TAG" "${repo[@]}" > /dev/null 2>&1; then
  add_to_existing
else
  flags=()
  if [ "$prerelease" = true ]; then flags=(--prerelease); fi
  if gh release create "$TAG" "$dir"/* "${repo[@]}" --target "$TARGET_SHA" --title "CapSnap $TAG" \
      --notes-file "$notes" "${flags[@]}"; then
    echo "Created release $TAG with the $platform files"
  elif gh release view "$TAG" "${repo[@]}" > /dev/null 2>&1; then
    # The other platform's workflow created it at the same moment.
    add_to_existing
  else
    exit 1
  fi
fi
