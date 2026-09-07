import { useEffect, useState } from "react";
import { api, errorCode, errorMessage } from "../lib/api";
import { formatCountdown, formatSchedule, formatTime } from "../lib/format";
import { PRESETS, type Preset, type ScheduledTask, type WindowTarget } from "../lib/types";
import { Card, Empty, Field } from "../components/ui";
import { WindowPicker, WindowIcon } from "../components/WindowPicker";

const TIME_PRESETS: { h: number; m: number; label: string }[] = [
  { h: 0, m: 30, label: "30m" },
  { h: 1, m: 0, label: "1h" },
  { h: 3, m: 0, label: "3h" },
  { h: 4, m: 0, label: "4h" },
  { h: 5, m: 0, label: "5h" },
  { h: 5, m: 5, label: "5h 5m" },
];

export function Dashboard({
  tasks, onEdit, onChanged,
}: { tasks: ScheduledTask[]; onEdit: (t: ScheduledTask) => void; onChanged: () => void }) {
  const [now, setNow] = useState(Date.now());
  useEffect(() => {
    const id = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(id);
  }, []);

  const [pickerOpen, setPickerOpen] = useState(false);
  const [target, setTarget] = useState<WindowTarget | null>(null);
  const [picked, setPicked] = useState<string | null>(null);
  const [timeIdx, setTimeIdx] = useState(5);
  const [customHours, setCustomHours] = useState(5);
  const [customMinutes, setCustomMinutes] = useState(5);
  const [preset, setPreset] = useState<Preset>("continue_confirm");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const startAutomation = async () => {
    if (!target) {
      setError("Select a target window first.");
      return;
    }
    setBusy(true);
    setError(null);
    const useCustom = timeIdx === TIME_PRESETS.length;
    const h = useCustom ? customHours : TIME_PRESETS[timeIdx].h;
    const m = useCustom ? customMinutes : TIME_PRESETS[timeIdx].m;
    try {
      await api.createQuickTask({
        target, preset, hours: h, minutes: m, confirmDelayMs: 1000,
      });
      onChanged();
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <>
      <div className="page-header">
        <div>
          <h1>Dashboard</h1>
          <div className="page-sub">Schedule an automation or manage existing ones.</div>
        </div>
      </div>

      <Card title="Quick Automation">
        {error && <div className="error-banner">{error}</div>}
        <Field label="Target">
          <div className="row">
            <div className="input" style={{ display: "flex", alignItems: "center", gap: 8, minHeight: 33 }}>
              {picked ? (
                <>
                  <WindowIcon processName={target?.process_name ?? ""} size={18} />
                  <span style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{picked}</span>
                </>
              ) : (
                <span className="muted">No window selected</span>
              )}
            </div>
            <button className="btn" style={{ flex: "none" }} onClick={() => setPickerOpen(true)}>Select…</button>
          </div>
        </Field>
        <Field label="Run after">
          <div className="quick-presets">
            {TIME_PRESETS.map((t, i) => (
              <button
                key={t.label}
                className={`chip${timeIdx === i ? " selected" : ""}`}
                onClick={() => setTimeIdx(i)}
              >
                {t.label}
              </button>
            ))}
            <button
              className={`chip${timeIdx === TIME_PRESETS.length ? " selected" : ""}`}
              onClick={() => setTimeIdx(TIME_PRESETS.length)}
            >
              Custom
            </button>
          </div>
          {timeIdx === TIME_PRESETS.length && (
            <div className="row" style={{ marginTop: 8 }}>
              <input className="input" type="number" min={0} value={customHours} onChange={(e) => setCustomHours(Number(e.target.value))} aria-label="Hours" />
              <span className="muted" style={{ flex: "none" }}>hours</span>
              <input className="input" type="number" min={0} value={customMinutes} onChange={(e) => setCustomMinutes(Number(e.target.value))} aria-label="Minutes" />
              <span className="muted" style={{ flex: "none" }}>minutes</span>
            </div>
          )}
        </Field>
        <Field label="Preset">
          <div className="quick-presets">
            {PRESETS.map((p) => (
              <button
                key={p.id}
                className={`chip${preset === p.id ? " selected" : ""}`}
                onClick={() => setPreset(p.id)}
              >
                {p.label}
              </button>
            ))}
          </div>
        </Field>
        <div className="row" style={{ justifyContent: "flex-end" }}>
          <button className="btn primary" disabled={busy} onClick={startAutomation}>
            {busy ? "Starting…" : "Start Automation"}
          </button>
        </div>
      </Card>

      <Card title={`Automations (${tasks.length})`}>
        {tasks.length === 0 ? (
          <Empty
            icon="⏱"
            title="No automations yet"
            hint="Use Quick Automation above to schedule your first sequence."
          />
        ) : (
          tasks.map((t) => (
            <TaskRow key={t.id} task={t} now={now} onEdit={onEdit} onChanged={onChanged} />
          ))
        )}
      </Card>

      <WindowPicker
        open={pickerOpen}
        onClose={() => setPickerOpen(false)}
        onPick={(t, c) => {
          setTarget(t);
          setPicked(c.title);
          setPickerOpen(false);
        }}
      />
    </>
  );
}

function TaskRow({
  task, now, onEdit, onChanged,
}: { task: ScheduledTask; now: number; onEdit: (t: ScheduledTask) => void; onChanged: () => void }) {
  const [busy, setBusy] = useState(false);
  const [flash, setFlash] = useState<string | null>(null);

  const act = async (fn: () => Promise<unknown>) => {
    setBusy(true);
    setFlash(null);
    try {
      await fn();
      onChanged();
    } catch (e) {
      setFlash(`${errorCode(e) ?? "Error"}: ${errorMessage(e)}`);
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="task-card" style={{ borderTop: "1px solid var(--border)", paddingTop: 10, marginTop: 10 }}>
      <div className="task-head">
        <div style={{ minWidth: 0 }}>
          <div className="task-name" style={{ opacity: task.enabled ? 1 : 0.55 }}>{task.name}</div>
          <div className="task-meta">
            <span>{task.target.process_name ?? "any process"}</span>
            <span>·</span>
            <span>{formatSchedule(task)}</span>
          </div>
        </div>
        <div className="task-actions">
          {!task.enabled && <span className="badge paused">paused</span>}
          <button className="btn small" disabled={busy} onClick={() => act(() => api.setTaskEnabled(task.id, !task.enabled))}>
            {task.enabled ? "Pause" : "Resume"}
          </button>
          <button className="btn small" disabled={busy} onClick={() => act(() => api.runTaskNow(task.id))}>Run Now</button>
          <button className="btn small ghost" onClick={() => onEdit(task)}>Edit</button>
          <button className="btn small ghost" disabled={busy} onClick={() => act(() => api.deleteTask(task.id))}>✕</button>
        </div>
      </div>
      <div className="task-meta">
        <span>Next run</span>
        <span className="countdown">{task.enabled ? formatCountdown(task.next_run_at, now) : "—"}</span>
        <span>·</span>
        <span>Last</span>
        <span>{formatTime(task.last_run_at)}</span>
      </div>
      {flash && <div className="error-banner" style={{ marginBottom: 0 }}>{flash}</div>}
    </div>
  );
}
