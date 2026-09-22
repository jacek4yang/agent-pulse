import json
import select
import sys
from pathlib import Path

# This program is run inside the disposable xterm, never in a user's terminal.
ready, _, _ = select.select([sys.stdin], [], [], 30)
if not ready:
    raise SystemExit('No input reached the isolated terminal')
line = sys.stdin.readline().rstrip('\n')
extra, _, _ = select.select([sys.stdin], [], [], 2)
result = {'text': line, 'extra_submission': bool(extra)}
Path(sys.argv[1]).write_text(json.dumps(result), encoding='utf-8')
if line != 'pulse probe 123' or extra:
    raise SystemExit(f'Unexpected terminal input: {result}')
