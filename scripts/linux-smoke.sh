#!/usr/bin/env bash
set -euo pipefail
result="$PWD/linux-enter-probe.json"
openbox > /tmp/agent-pulse-openbox.log 2>&1 &
wm_pid=$!
xterm -T AgentPulseIsolatedProbe -e python3 scripts/terminal_probe.py "$result" &
terminal_pid=$!
trap 'kill "$terminal_pid" "$wm_pid" 2>/dev/null || true' EXIT
for attempt in {1..50}; do
  if xdotool search --name '^AgentPulseIsolatedProbe$' >/dev/null 2>&1; then break; fi
  sleep 0.1
done
AGENT_PULSE_ISOLATED_DESKTOP_TEST=1 cargo run --manifest-path src-tauri/Cargo.toml --example desktop_probe --locked
wait "$terminal_pid"
python3 - "$result" <<'PY'
import json, sys
from pathlib import Path
result = json.loads(Path(sys.argv[1]).read_text())
assert result == {'text': 'pulse probe 123', 'extra_submission': False}, result
print(result)
PY
