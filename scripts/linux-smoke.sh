#!/usr/bin/env bash
set -euo pipefail
result="$PWD/linux-enter-probe.json"
cargo build --manifest-path src-tauri/Cargo.toml --example desktop_probe --locked
openbox > /tmp/agent-pulse-openbox.log 2>&1 &
wm_pid=$!
trap 'cat /tmp/agent-pulse-openbox.log; kill "$wm_pid" 2>/dev/null || true' EXIT
wm_ready=false
for attempt in {1..300}; do
  if xprop -root _NET_SUPPORTING_WM_CHECK | grep -q 'window id #'; then
    wm_ready=true
    break
  fi
  kill -0 "$wm_pid"
  sleep 0.1
done
if [ "$wm_ready" != true ]; then echo 'Window manager did not initialize' >&2; exit 1; fi
xterm -T AgentPulseIsolatedProbe -e python3 scripts/terminal_probe.py "$result" &
terminal_pid=$!
trap 'cat /tmp/agent-pulse-openbox.log; xprop -root _NET_CLIENT_LIST; kill "$terminal_pid" "$wm_pid" 2>/dev/null || true' EXIT
client_ready=false
for attempt in {1..100}; do
  probe_id=$(xdotool search --name '^AgentPulseIsolatedProbe$' 2>/dev/null | head -n 1 || true)
  if [ -n "$probe_id" ]; then
    printf -v probe_hex '0x%x' "$probe_id"
    if xprop -root _NET_CLIENT_LIST | grep -qw "$probe_hex"; then
      client_ready=true
      break
    fi
  fi
  sleep 0.1
done
if [ "$client_ready" != true ]; then echo 'Probe is not managed by the window manager' >&2; exit 1; fi
xdotool search --name '^AgentPulseIsolatedProbe$' getwindowpid
AGENT_PULSE_ISOLATED_DESKTOP_TEST=1 src-tauri/target/debug/examples/desktop_probe
wait "$terminal_pid"
python3 - "$result" <<'PY'
import json, sys
from pathlib import Path
result = json.loads(Path(sys.argv[1]).read_text())
assert result == {'text': 'pulse probe 123', 'extra_submission': False}, result
print(result)
PY
