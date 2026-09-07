import { useEffect, useMemo, useState } from "react";
import { api, errorCode, errorMessage } from "../lib/api";
import { formatCountdown, formatSchedule, formatTime } from "../lib/format";
import { useI18n } from "../lib/i18n";
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

/** Local datetime-local value (with seconds) for `now + 1h`. */
function defaultAtValue(): string {
  const d = new Date(Date.now() + 3600_000);
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}T${p(d.getHours())}:${p(d.getMinutes())}:${p(d.getSeconds())}`;
}

export function Dashboard({
  tasks, onEdit, onChanged,
}: { tasks: ScheduledTask[]; onEdit: (t: ScheduledTask) => void; onChanged: () => void }) {
  const { t, lang } = useI18n();
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
    return formatExactTimeLocal(d, lang);
  }, [atValue, lang]);

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
      setError(t("selectTargetFirst"));
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
          <h1>{t("dashboardTitle")}</h1>
          <div className="page-sub">{t("dashboardSub")}</div>
        </div>
      </div>

      <Card title={t("quickAutomation")}>
        {error && <div className="error-banner">{error}</div>}
        <Field label={t("target")}>
          <div className="row">
            <div className="input" style={{ display: "flex", alignItems: "center", gap: 8, minHeight: 33 }}>
              {picked ? (
                <>
                  <WindowIcon processName={target?.process_name ?? ""} size={18} />
                  <span style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{picked}</span>
                </>
              ) : (
                <span className="muted">{t("noWindowSelected")}</span>
              )}
            </div>
            <button className="btn" style={{ flex: "none" }} onClick={() => setPickerOpen(true)}>{t("select")}</button>
          </div>
        </Field>

        <Field label={t("schedule")}>
          <div className="quick-presets" style={{ marginBottom: 8 }}>
            {(["after", "at", "every"] as ScheduleMode[]).map((mode) => (
              <button key={mode} className={`chip${schedMode === mode ? " selected" : ""}`} onClick={() => setSchedMode(mode)}>
                {mode === "after" ? t("schedAfter") : mode === "at" ? t("schedAt") : t("schedEvery")}
              </button>
            ))}
          </div>

          {schedMode === "after" && (
            <>
              <div className="quick-presets" style={{ marginBottom: 8 }}>
                {TIME_PRESETS.map((p, i) => (
                  <button key={p.label} className={`chip${timeIdx === i ? " selected" : ""}`} onClick={() => setTimeIdx(i)}>
                    {p.label}
                  </button>
                ))}
                <button className={`chip${timeIdx === TIME_PRESETS.length ? " selected" : ""}`} onClick={() => setTimeIdx(TIME_PRESETS.length)}>
                  {t("custom")}
                </button>
              </div>
              {timeIdx === TIME_PRESETS.length && (
                <div className="row" style={{ marginTop: 8 }}>
                  <input className="input" type="number" min={0} value={h} onChange={(e) => setH(Number(e.target.value))} aria-label={t("hours")} />
                  <span className="muted" style={{ flex: "none" }}>{t("hours")}</span>
                  <input className="input" type="number" min={0} max={59} value={m} onChange={(e) => setM(Number(e.target.value))} aria-label={t("minutes")} />
                  <span className="muted" style={{ flex: "none" }}>{t("minutes")}</span>
                  <input className="input" type="number" min={0} max={59} value={s} onChange={(e) => setS(Number(e.target.value))} aria-label={t("seconds")} />
                  <span className="muted" style={{ flex: "none" }}>{t("seconds")}</span>
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
                aria-label={t("schedAt")}
              />
              <div className="muted" style={{ marginTop: 5, fontSize: 12 }}>
                {t("firesAt")} {atPreview} {t("localTime")}
              </div>
            </>
          )}

          {schedMode === "every" && (
            <div className="row" style={{ marginTop: 8 }}>
              <input className="input" type="number" min={0} value={everyMin} onChange={(e) => setEveryMin(Number(e.target.value))} aria-label={t("minutes")} />
              <span className="muted" style={{ flex: "none" }}>{t("minutes")}</span>
              <input className="input" type="number" min={0} max={59} value={everySec} onChange={(e) => setEverySec(Number(e.target.value))} aria-label={t("seconds")} />
              <span className="muted" style={{ flex: "none" }}>{t("seconds")}</span>
            </div>
          )}
        </Field>

        <Field label={t("actions")}>
          <div className="quick-presets" style={{ marginBottom: 8 }}>
            <button className={`chip${actionMode === "preset" ? " selected" : ""}`} onClick={() => setActionMode("preset")}>
              {t("preset")}
            </button>
            <button className={`chip${actionMode === "custom" ? " selected" : ""}`} onClick={() => setActionMode("custom")}>
              {t("customFlow")}
            </button>
          </div>
          {actionMode === "preset" ? (
            <div className="quick-presets">
              {PRESETS.map((p) => (
                <button key={p.id} className={`chip${preset === p.id ? " selected" : ""}`} onClick={() => setPreset(p.id)}>
                  {presetLabel(p.id, lang)}
                </button>
              ))}
            </div>
          ) : (
            <ActionList actions={customActions} onChange={setCustomActions} />
          )}
        </Field>

        <div className="row" style={{ justifyContent: "flex-end" }}>
          <button className="btn primary" disabled={busy} onClick={startAutomation}>
            {busy ? t("starting") : t("startAutomation")}
          </button>
        </div>
      </Card>

      <Card title={`${t("automations")} (${tasks.length})`}>
        {tasks.length === 0 ? (
          <Empty icon="⏱" title={t("noAutomations")} hint={t("noAutomationsHint")} />
        ) : (
          tasks.map((task) => (
            <TaskRow key={task.id} task={task} now={now} onEdit={onEdit} onChanged={onChanged} />
          ))
        )}
      </Card>

      <WindowPicker
        open={pickerOpen}
        onClose={() => setPickerOpen(false)}
        onPick={(pickedTarget, c) => {
          setTarget(pickedTarget);
          setPicked(c.title);
          setPickerOpen(false);
        }}
      />
    </>
  );
}

function presetLabel(p: Preset, lang: "en" | "zh"): string {
  const labels: Record<Preset, [string, string]> = {
    continue_confirm: ["Continue + Confirm", "Continue + 确认"],
    continue: ["Continue", "Continue"],
    confirm_continue: ["Confirm + Continue", "确认 + Continue"],
    double_enter: ["Double Enter", "双击 Enter"],
    empty_submit: ["Empty Submit", "空提交"],
  };
  return labels[p][lang === "zh" ? 1 : 0];
}

function formatExactTimeLocal(d: Date, lang: "en" | "zh"): string {
  if (Number.isNaN(d.getTime())) return "—";
  if (lang === "zh") {
    const p2 = (n: number) => String(n).padStart(2, "0");
    return `${d.getFullYear()}年${d.getMonth() + 1}月${d.getDate()}日 ${p2(d.getHours())}:${p2(d.getMinutes())}:${p2(d.getSeconds())}`;
  }
  const day = d.getDate();
  const suffix =
    day % 10 === 1 && day !== 11 ? "st" :
    day % 10 === 2 && day !== 12 ? "nd" :
    day % 10 === 3 && day !== 13 ? "rd" : "th";
  const month = d.toLocaleString("en-US", { month: "short" });
  const time = d.toLocaleString("en-US", {
    hour: "numeric", minute: "2-digit", second: "2-digit", hour12: true,
  });
  return `${month} ${day}${suffix}, ${d.getFullYear()} ${time}`;
}

function TaskRow({
  task, now, onEdit, onChanged,
}: { task: ScheduledTask; now: number; onEdit: (t: ScheduledTask) => void; onChanged: () => void }) {
  const { t, locale } = useI18n();
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
            <span>{task.target.process_name ?? "—"}</span>
            <span>·</span>
            <span>{formatSchedule(task, locale === "zh-CN" ? "zh" : "en")}</span>
          </div>
        </div>
        <div className="task-actions">
          {!task.enabled && <span className="badge paused">{t("paused")}</span>}
          <button className="btn small" disabled={busy} onClick={() => act(() => api.setTaskEnabled(task.id, !task.enabled))}>
            {task.enabled ? t("pause") : t("resume")}
          </button>
          <button className="btn small" disabled={busy} onClick={() => act(() => api.runTaskNow(task.id))}>{t("runNow")}</button>
          <button className="btn small ghost" onClick={() => onEdit(task)}>{t("edit")}</button>
          <button className="btn small ghost" disabled={busy} onClick={() => act(() => api.deleteTask(task.id))}>✕</button>
        </div>
      </div>
      <div className="task-meta">
        <span>{t("nextRun")}</span>
        <span className="countdown">{task.enabled ? formatCountdown(task.next_run_at, now, locale === "zh-CN" ? "zh" : "en") : "—"}</span>
        <span>·</span>
        <span>{t("last")}</span>
        <span>{formatTime(task.last_run_at, locale)}</span>
      </div>
      {flash && <div className="error-banner" style={{ marginBottom: 0 }}>{flash}</div>}
    </div>
  );
}
