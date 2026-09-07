import { useEffect, useMemo, useState } from "react";
import { api, errorCode, errorMessage } from "../lib/api";
import { formatCountdown, formatSchedule, formatTime } from "../lib/format";
import { PRESETS, type Action, type Preset, type ScheduledTask, type Schedule, type WindowTarget } from "../lib/types";
import { Card, Empty, Field } from "../components/ui";
import { WindowPicker, WindowIcon } from "../components/WindowPicker";
import { ActionList } from "../components/ActionList";

const TIME_PRESETS: { h: number; m: number; s: number; label: string }[] = [
  { h: 0, m: 30, s: 0, label: "30m" },
  { h: 1, m: 0, s: 0, label: "1h" },
  { h: 3, m: 0, s: 0, label: "3h" },
  { h: 4, m: 0, s: 0, label: "4h" },
  { h: 5, m: 0, s: 0, label: "5h" },
  { h: 5, m: 5, s: 0, label: "5h 5m" },
];

type ScheduleMode = "after" | "at" | "every";
type ActionMode = "preset" | "custom";

/** "Sep 8th, 2026 1:26:05 AM" style preview of the picked local time. */
function formatExactTime(d: Date): string {
  if (Number.isNaN(d.getTime())) return "—";
  const day = d.getDate();
  const suffix =
    day % 10 === 1 && day !== 11 ? "st" :
    day % 10 === 2 && day !== 12 ? "nd" :
    day % 10 === 3 && day !== 13 ? "rd" : "th";
  const month = d.toLocaleString(undefined, { month: "short" });
  const time = d.toLocaleString(undefined, {
    hour: "numeric", minute: "2-digit", second: "2-digit", hour12: true,
  });
  return `${month} ${day}${suffix}, ${d.getFullYear()} ${time}`;
}

/** Local datetime-local value (with seconds) for `now + 1h`. */
function defaultAtValue(): string {
  const d = new Date(Date.now() + 3600_000);
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}T${p(d.getHours())}:${p(d.getMinutes())}:${p(d.getSeconds())}`;
}

export function Dashboard({
  tasks, onEdit, onChanged,
}: { tasks: ScheduledTask[]; onEdit: (t: ScheduledTask) => void; onChanged: () => void }) {
  const [now, setNow] = useState(Date.now());
  useEffect(() => {
    const id = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(id);
  }, []);

  // target
  const [pickerOpen, setPickerOpen] = useState(false);
  const [target, setTarget] = useState<WindowTarget | null>(null);
  const [picked, setPicked] = useState<string | null>(null);

  // schedule
  const [schedMode, setSchedMode] = useState<ScheduleMode>("after");
  const [timeIdx, setTimeIdx] = useState(5);
  const [h, setH] = useState(5);
  const [m, setM] = useState(5);
  const [s, setS] = useState(0);
  const [atValue, setAtValue] = useState(defaultAtValue);
  const [everyMin, setEveryMin] = useState(30);
  const [everySec, setEverySec] = useState(0);

  // actions
  const [actionMode, setActionMode] = useState<ActionMode>("preset");
  const [preset, setPreset] = useState<Preset>("continue_confirm");
  const [customActions, setCustomActions] = useState<Action[]>([
    { type: "focus_target" },
    { type: "type_text", text: "continue" },
    { type: "press_key", key: "enter", count: 1, interval_ms: 0 },
  ]);

  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const atPreview = useMemo(() => {
    const d = new Date(atValue);
    return formatExactTime(d);
  }, [atValue]);

  const buildSchedule = (): Schedule => {
    switch (schedMode) {
      case "after": {
        const useCustom = timeIdx === TIME_PRESETS.length;
        const hours = useCustom ? h : TIME_PRESETS[timeIdx].h;
        const minutes = useCustom ? m : TIME_PRESETS[timeIdx].m;
        const seconds = useCustom ? s : TIME_PRESETS[timeIdx].s;
        return { kind: "after", hours, minutes, seconds };
      }
      case "at": {
        const d = new Date(atValue);
        return {
          kind: "at",
          year: d.getFullYear(),
          month: d.getMonth() + 1,
          day: d.getDate(),
          hour: d.getHours(),
          minute: d.getMinutes(),
          second: d.getSeconds(),
        };
      }
      case "every":
        return { kind: "every", interval_seconds: Math.max(1, everyMin * 60 + everySec) };
    }
  };

  const buildActions = (): Action[] => {
    if (actionMode === "custom") return customActions;
    const enter = (): Action => ({ type: "press_key", key: "enter", count: 1, interval_ms: 0 });
    switch (preset) {
      case "continue":
        return [{ type: "focus_target" }, { type: "type_text", text: "continue" }, enter()];
      case "continue_confirm":
        return [
          { type: "focus_target" },
          { type: "type_text", text: "continue" },
          enter(),
          { type: "delay", milliseconds: 1000 },
          enter(),
        ];
      case "confirm_continue":
        return [
          { type: "focus_target" },
          enter(),
          { type: "delay", milliseconds: 500 },
          { type: "type_text", text: "continue" },
          enter(),
        ];
      case "double_enter":
        return [{ type: "focus_target" }, enter(), { type: "delay", milliseconds: 500 }, enter()];
      case "empty_submit":
        return [{ type: "focus_target" }, enter()];
    }
  };

  const startAutomation = async () => {
    if (!target) {
      setError("Select a target window first.");
      return;
    }
    setBusy(true);
    setError(null);
    try {
      await api.createQuickTask({
        target, schedule: buildSchedule(), actions: buildActions(),
      });
      onChanged();
    } catch (e) {
      setError(`${errorCode(e) ?? "Error"}: ${errorMessage(e)}`);
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

        <Field label="Schedule">
          <div className="quick-presets" style={{ marginBottom: 8 }}>
            {(["after", "at", "every"] as ScheduleMode[]).map((mode) => (
              <button key={mode} className={`chip${schedMode === mode ? " selected" : ""}`} onClick={() => setSchedMode(mode)}>
                {mode === "after" ? "After" : mode === "at" ? "At (exact time)" : "Every (recurring)"}
              </button>
            ))}
          </div>

          {schedMode === "after" && (
            <>
              <div className="quick-presets" style={{ marginBottom: 8 }}>
                {TIME_PRESETS.map((t, i) => (
                  <button key={t.label} className={`chip${timeIdx === i ? " selected" : ""}`} onClick={() => setTimeIdx(i)}>
                    {t.label}
                  </button>
                ))}
                <button className={`chip${timeIdx === TIME_PRESETS.length ? " selected" : ""}`} onClick={() => setTimeIdx(TIME_PRESETS.length)}>
                  Custom
                </button>
              </div>
              {timeIdx === TIME_PRESETS.length && (
                <div className="row" style={{ marginTop: 8 }}>
                  <input className="input" type="number" min={0} value={h} onChange={(e) => setH(Number(e.target.value))} aria-label="Hours" />
                  <span className="muted" style={{ flex: "none" }}>hours</span>
                  <input className="input" type="number" min={0} max={59} value={m} onChange={(e) => setM(Number(e.target.value))} aria-label="Minutes" />
                  <span className="muted" style={{ flex: "none" }}>minutes</span>
                  <input className="input" type="number" min={0} max={59} value={s} onChange={(e) => setS(Number(e.target.value))} aria-label="Seconds" />
                  <span className="muted" style={{ flex: "none" }}>seconds</span>
                </div>
              )}
            </>
          )}

          {schedMode === "at" && (
            <>
              <input
                className="input"
                type="datetime-local"
                step={1}
                value={atValue}
                onChange={(e) => setAtValue(e.target.value)}
                aria-label="Exact date and time"
              />
              <div className="muted" style={{ marginTop: 5, fontSize: 12 }}>
                Fires at {atPreview} (local time)
              </div>
            </>
          )}

          {schedMode === "every" && (
            <div className="row" style={{ marginTop: 8 }}>
              <input className="input" type="number" min={0} value={everyMin} onChange={(e) => setEveryMin(Number(e.target.value))} aria-label="Minutes" />
              <span className="muted" style={{ flex: "none" }}>minutes</span>
              <input className="input" type="number" min={0} max={59} value={everySec} onChange={(e) => setEverySec(Number(e.target.value))} aria-label="Seconds" />
              <span className="muted" style={{ flex: "none" }}>seconds</span>
            </div>
          )}
        </Field>

        <Field label="Actions">
          <div className="quick-presets" style={{ marginBottom: 8 }}>
            <button className={`chip${actionMode === "preset" ? " selected" : ""}`} onClick={() => setActionMode("preset")}>
              Preset
            </button>
            <button className={`chip${actionMode === "custom" ? " selected" : ""}`} onClick={() => setActionMode("custom")}>
              Custom flow
            </button>
          </div>
          {actionMode === "preset" ? (
            <div className="quick-presets">
              {PRESETS.map((p) => (
                <button key={p.id} className={`chip${preset === p.id ? " selected" : ""}`} onClick={() => setPreset(p.id)}>
                  {p.label}
                </button>
              ))}
            </div>
          ) : (
            <ActionList actions={customActions} onChange={setCustomActions} />
          )}
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
