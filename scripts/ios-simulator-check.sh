#!/usr/bin/env bash
# Runs target/ios/sim/AcidRecall.app (scripts/ios-app.sh sim) in an iPad Simulator, then saves
# a screenshot, the app's output and any crash report into OUT_DIR (docs/ipad.md). Exits 1 when
# the app is no longer running after SECONDS (default 30): it crashed or quit.
#
#   scripts/ios-simulator-check.sh OUT_DIR [SECONDS]
#
# The Simulator is the first available iPad 11-inch (an iPad Air first), else any iPad.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
out=${1:?usage: scripts/ios-simulator-check.sh OUT_DIR [SECONDS]}
wait=${2:-30}
mkdir -p "$out"
out=$(cd "$out" && pwd)
app=$root/target/ios/sim/AcidRecall.app
[ -d "$app" ] || { echo "no $app: scripts/ios-app.sh sim" >&2; exit 1; }
bundle_id=$(/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "$app/Info.plist")

udid=$(xcrun simctl list devices available -j | python3 -c '
import json, sys
devices = json.load(sys.stdin)["devices"]
ipads = [d for runtime, ds in devices.items() if "iOS" in runtime
         for d in ds if d["name"].startswith("iPad")]
air = [d for d in ipads if "Air" in d["name"] and "11" in d["name"]]
eleven = [d for d in ipads if "11" in d["name"]]
pick = air or eleven or ipads
print(pick[-1]["udid"] + " " + pick[-1]["name"] if pick else "")
')
[ -n "$udid" ] || { echo "no iPad Simulator" >&2; xcrun simctl list devices available >&2; exit 1; }
name=${udid#* }
udid=${udid%% *}
echo "Simulator: $name ($udid)"

xcrun simctl boot "$udid" 2>/dev/null || true
xcrun simctl bootstatus "$udid" -b
xcrun simctl install "$udid" "$app"
xcrun simctl launch --terminate-running-process \
    --stdout="$out/stdout.log" --stderr="$out/stderr.log" "$udid" "$bundle_id"
sleep "$wait"
xcrun simctl io "$udid" screenshot "$out/ipad.png"

status=0
if xcrun simctl spawn "$udid" launchctl list | grep -q "$bundle_id"; then
    echo "The app is running after ${wait} s."
else
    echo "The app is not running after ${wait} s." >&2
    status=1
fi
for f in ~/Library/Logs/DiagnosticReports/AcidRecall*; do
    [ -e "$f" ] && cp "$f" "$out/"
done
echo "--- stderr ---"
cat "$out/stderr.log" 2>/dev/null || true
xcrun simctl shutdown "$udid" || true
exit $status
