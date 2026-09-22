import json
import sys
import tkinter as tk
from pathlib import Path

# Only invoked on the disposable GitHub Windows runner, not a user's desktop.
root = tk.Tk()
root.title('AgentPulseIsolatedProbe')
entry = tk.Entry(root, width=40)
entry.pack(padx=30, pady=30)
entry.focus_set()
submissions = []
result_path = Path(sys.argv[1])

def finish():
    result_path.write_text(json.dumps({'submissions': submissions}), encoding='utf-8')
    root.destroy()

def submitted(_event):
    submissions.append(entry.get())
    entry.delete(0, tk.END)
    if len(submissions) == 1:
        root.after(2000, finish)
    return 'break'

entry.bind('<Return>', submitted)
root.after(60000, finish)
root.mainloop()
