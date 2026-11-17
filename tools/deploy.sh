#!/bin/sh
# Install the viewer on the Quest, optionally push a mesh to it, and start it.
# Usage: tools/deploy.sh [mesh.scm]
set -e
here=$(cd "$(dirname "$0")/.." && pwd)
adb install -r "$here/target/release/apk/scview.apk"
# The server's access code, so the headset need not be typed it (tools/serve.py makes it on first run).
if [ -f "$HOME/.scview/token" ]; then adb push "$HOME/.scview/token" /sdcard/Android/data/org.scview.viewer/files/token.txt; fi
if [ -n "$1" ]; then adb push "$1" /sdcard/Android/data/org.scview.viewer/files/sea.scm; fi
adb shell am force-stop org.scview.viewer
adb shell am start -n org.scview.viewer/android.app.NativeActivity
