#!/bin/bash
# Fails when the Android app's permissions, hardware features or CSP differ from what the Play
# Data Safety draft declares (.hcc-nocapsnap/data-safety/declared.txt). W5a. Run from the repo root.
set -euo pipefail

MANIFEST=app/src-tauri/gen/android/app/src/main/AndroidManifest.xml
TAURI=app/src-tauri/tauri.conf.json
DECLARED=.hcc-nocapsnap/data-safety/declared.txt

found=$(
    {
        grep -o '<uses-permission android:name="[^"]*"' "$MANIFEST" | sed 's/.*name="\([^"]*\)"/permission \1/'
        grep -o '<uses-feature android:name="[^"]*"' "$MANIFEST" | sed 's/.*name="\([^"]*\)"/feature \1/'
        # The connect-src directive, as written.
        grep -o '"connect-src": "[^"]*"' "$TAURI" | sed 's/.*: "\(.*\)"/csp-connect \1/'
        # Any remote origin anywhere in the CSP block would be a new third party.
        if sed -n '/"csp": {/,/}/p' "$TAURI" | grep -qE 'https?://' &&
           [ "$(sed -n '/"csp": {/,/}/p' "$TAURI" | grep -oE 'https?://[^ "]+' | grep -vc 'ipc.localhost')" -gt 0 ]; then
            echo "csp-origins remote"
        else
            echo "csp-origins none"
        fi
    } | LC_ALL=C sort
)
declared=$(grep -v '^#' "$DECLARED" | grep -v '^$' | LC_ALL=C sort)

if [ "$found" != "$declared" ]; then
    echo "The app no longer matches the Data Safety draft:" >&2
    diff <(printf '%s\n' "$declared") <(printf '%s\n' "$found") >&2 || true
    echo "Update .hcc-nocapsnap/protocol/W5a-data-safety-draft.md, then $DECLARED, in this pull request." >&2
    exit 1
fi
echo "Data Safety declarations match the app ($(printf '%s\n' "$found" | wc -l) items)."
