#!/usr/bin/env bash
# Runs target/ios/sim/AcidRecall.app (scripts/ios-app.sh sim) in an iPad Simulator, then saves
# screenshots, the app's output and any crash report into OUT_DIR (docs/ipad.md), and prints
# them too, so the CI log alone tells what happened (the screenshots as base64 JPEG between
# BEGIN and END lines). Exits 1 when the app is no longer running after SECONDS (default 30):
# it crashed or quit.
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

# A screenshot of the Simulator now, as OUT_DIR/NAME.png and, in the log, a small JPEG.
shot() {
    xcrun simctl io "$udid" screenshot "$out/$1.png" >/dev/null 2>&1 || return 0
    sips -s format jpeg -s formatOptions 55 -Z 900 "$out/$1.png" --out "$out/$1.jpg" >/dev/null
    echo "BEGIN $1.jpg"
    base64 -i "$out/$1.jpg" | fold -w 120
    echo "END $1.jpg"
}

xcrun simctl boot "$udid" 2>/dev/null || true
xcrun simctl bootstatus "$udid" -b >/dev/null
xcrun simctl install "$udid" "$app"
start=$(date "+%Y-%m-%d %H:%M:%S")
launched=$(SIMCTL_CHILD_RUST_BACKTRACE=1 SIMCTL_CHILD_ORB_GPU_INFO=1 \
    xcrun simctl launch --terminate-running-process \
    --stdout="$out/stdout.log" --stderr="$out/stderr.log" "$udid" "$bundle_id")
echo "$launched"
pid=${launched##*: }
sleep 6
shot early
sleep "$wait"
shot late

status=0
if kill -0 "$pid" 2>/dev/null; then
    echo "The app is running after $((wait + 6)) s."
else
    echo "The app is not running after $((wait + 6)) s." >&2
    status=1
fi

echo "--- stdout ---"
cat "$out/stdout.log" 2>/dev/null || true
echo "--- stderr ---"
cat "$out/stderr.log" 2>/dev/null || true
echo "--- the app's crash logs (Library/Logs/OpenRB-338) ---"
data=$(xcrun simctl get_app_container "$udid" "$bundle_id" data 2>/dev/null || true)
if [ -n "$data" ] && [ -d "$data/Library/Logs/OpenRB-338" ]; then
    for f in "$data/Library/Logs/OpenRB-338"/*; do
        [ -e "$f" ] || continue
        echo "== $f"
        head -80 "$f"
        cp "$f" "$out/"
    done
fi
echo "--- crash reports ---"
for f in ~/Library/Logs/DiagnosticReports/AcidRecall*; do
    [ -e "$f" ] || continue
    echo "== $f"
    head -120 "$f"
    cp "$f" "$out/"
done
echo "--- system log of the app ---"
xcrun simctl spawn "$udid" log show --start "$start" --style compact \
    --predicate 'process == "AcidRecall"' 2>/dev/null | tail -150 | tee "$out/system.log" || true

xcrun simctl shutdown "$udid" || true
exit $status
