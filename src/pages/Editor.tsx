import { useState } from "react";
import { useI18n } from "../lib/i18n";
import { api, errorCode, errorMessage } from "../lib/api";
import { ActionList } from "../components/ActionList";
import { WindowIcon, WindowPicker } from "../components/WindowPicker";
import { Card, Field } from "../components/ui";
import type { Action, MisfirePolicy, ScheduledTask, Schedule, WindowTarget } from "../lib/types";

type ScheduleKind = Schedule["kind"];

/** Editor for an existing task (or null => new). */
export function Editor({
  task, onClose, onChanged,
}: { task: ScheduledTask | null; onClose: () => void; onChanged: () => void }) {
  const { t } = useI18n();
  const [name, setName] = useState(task?.name ?? "");
  const [target, setTarget] = useState<WindowTarget>(task?.target ?? { title_match_mode: "contains" });
  const [kind, setKind] = useState<ScheduleKind>(task?.schedule?.kind ?? "after");
  const [hours, setHours] = useState(task?.schedule?.kind === "after" ? task.schedule.hours : 5);
  const [minutes, setMinutes] = useState(task?.schedule?.kind === "after" ? task.schedule.minutes : 5);
  const [seconds, setSeconds] = useState(task?.schedule?.kind === "after" ? task.schedule.seconds : 0);
  const [atZone, setAtZone] = useState(
    task?.schedule?.kind === "at" ? task.schedule.timezone || "local" : "local",
  );
  const [atValue, setAtValue] = useState(() => {
    const d = new Date(Date.now() + 3600_000);
    const p = (n: number) => String(n).padStart(2, "0");
    return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}T${p(d.getHours())}:${p(d.getMinutes())}`;
  });
  const [everyMinutes, setEveryMinutes] = useState(
    task?.schedule?.kind === "every" ? Math.round(task.schedule.interval_seconds / 60) : 30,
  );
  const [actions, setActions] = useState<Action[]>(task?.actions ?? []);
  const [policy, setPolicy] = useState<MisfirePolicy>(task?.misfire_policy ?? "run_immediately");
  const [pickerOpen, setPickerOpen] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [flash, setFlash] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const buildSchedule = (): Schedule => {
    switch (kind) {
      case "after":
        return { kind, hours, minutes, seconds };
      case "at": {
        const d = new Date(atValue);
        return {
          kind,
          year: d.getFullYear(),
          month: d.getMonth() + 1,
          day: d.getDate(),
          hour: d.getHours(),
          minute: d.getMinutes(),
          second: 0,
          timezone: atZone,
        };
      }
      case "every":
        return { kind, interval_seconds: Math.max(1, Math.round(everyMinutes * 60)) };
    }
  };

  const save = async () => {
    setError(null);
    setBusy(true);
    try {
      if (task) {
        await api.updateTask({
          ...task, name, target, schedule: buildSchedule(), actions, misfire_policy: policy,
        });
      } else {
        await api.createTask({
          name, target, schedule: buildSchedule(), actions, misfirePolicy: policy,
        });
      }
      onChanged();
      onClose();
    } catch (e) {
      setError(`${errorCode(e) ?? "Error"}: ${errorMessage(e)}`);
    } finally {
      setBusy(false);
    }
  };

  const test = async () => {
    setError(null);
    setFlash(null);
    setBusy(true);
    try {
      const hit = await api.testWindowTarget(target);
      setFlash(`${t("resolved")} ${hit.title} (${hit.process_name}, PID ${hit.process_id})`);
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
          <h1>{task ? t("editAutomation") : t("newAutomationTitle")}</h1>
          <div className="page-sub">{t("editorSub")}</div>
        </div>
      </div>

      {error && <div className="error-banner">{error}</div>}
      {flash && (
        <div
          className="error-banner"
          style={{ color: "var(--success)", borderColor: "var(--success)", background: "var(--success-soft)" }}
        >
          {flash}
        </div>
      )}

      <Card>
        <Field label={t("name")}>
          <input className="input" value={name} onChange={(e) => setName(e.target.value)} placeholder={t("namePlaceholder")} />
        </Field>

        <Field label={t("target")}>
          <div className="row">
            <div className="input" style={{ display: "flex", alignItems: "center", gap: 8, minHeight: 33 }}>
              {target.process_name ? (
                <>
                  <WindowIcon processName={target.process_name} size={18} />
                  <span style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                    {target.title || t("anyTitle")} — {target.process_name} · PID {target.process_id}
                  </span>
                </>
              ) : (
                <span className="muted">No window selected</span>
              )}
            </div>
            <select
              className="input"
              style={{ flex: "none", width: 110 }}
              value={target.title_match_mode}
              onChange={(e) => setTarget({ ...target, title_match_mode: e.target.value as WindowTarget["title_match_mode"] })}
              aria-label={t("titleMatchMode")}
            >
              <option value="contains">{t("matchContains")}</option>
              <option value="exact">{t("matchExact")}</option>
              <option value="regex">{t("matchRegex")}</option>
              <option value="any">{t("matchAny")}</option>
            </select>
            <button className="btn" style={{ flex: "none" }} onClick={() => setPickerOpen(true)}>{t("select")}</button>
          </div>
        </Field>

        <Field label={t("schedule")}>
          <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
            <label className="row" style={{ gap: 8 }}>
              <input type="radio" checked={kind === "after"} onChange={() => setKind("after")} style={{ flex: "none" }} />
              <span style={{ flex: "none", fontSize: 12 }}>{t("schedAfter")}</span>
              <input className="input" type="number" min={0} value={hours} onChange={(e) => setHours(Number(e.target.value))} aria-label="Hours" />
              <span className="muted" style={{ flex: "none" }}>{t("hUnit")}</span>
              <input className="input" type="number" min={0} value={minutes} onChange={(e) => setMinutes(Number(e.target.value))} aria-label="Minutes" />
              <span className="muted" style={{ flex: "none" }}>{t("mUnit")}</span>
              <input className="input" type="number" min={0} value={seconds} onChange={(e) => setSeconds(Number(e.target.value))} aria-label="Seconds" />
              <span className="muted" style={{ flex: "none" }}>{t("sUnit")}</span>
            </label>
            <label className="row" style={{ gap: 8 }}>
              <input type="radio" checked={kind === "at"} onChange={() => setKind("at")} style={{ flex: "none" }} />
              <span style={{ flex: "none", fontSize: 12 }}>{t("schedAt")}</span>
              <input className="input" type="datetime-local" step={1} value={atValue} onChange={(e) => setAtValue(e.target.value)} disabled={kind !== "at"} />
            </label>
            <div className="row" style={{ gap: 8 }}>
              <span style={{ flex: "none", fontSize: 12 }}>{t("timezone")}</span>
              <input
                className="input"
                type="text"
                list="agent-pulse-timezones"
                value={atZone}
                onChange={(e) => setAtZone(e.target.value)}
                placeholder="local"
                aria-label={t("timezone")}
                disabled={kind !== "at"}
                style={{ fontFamily: "ui-monospace, Consolas, monospace" }}
              />
            </div>
            <label className="row" style={{ gap: 8 }}>
              <input type="radio" checked={kind === "every"} onChange={() => setKind("every")} style={{ flex: "none" }} />
              <span style={{ flex: "none", fontSize: 12 }}>{t("schedEvery")}</span>
              <input className="input" type="number" min={1} value={everyMinutes} onChange={(e) => setEveryMinutes(Number(e.target.value))} disabled={kind !== "every"} />
              <span className="muted" style={{ flex: "none" }}>{t("minutes")}</span>
            </label>
          </div>
        </Field>

        <Field label={t("misfirePolicy")}>
          <select className="input" value={policy} onChange={(e) => setPolicy(e.target.value as MisfirePolicy)}>
            <option value="run_immediately">{t("misfireRun")}</option>
            <option value="skip">{t("misfireSkip")}</option>
          </select>
        </Field>
      </Card>

      <Card title={`${t("actionsCount")} (${actions.length})`}>
        <ActionList actions={actions} onChange={setActions} />
      </Card>

      <div className="row" style={{ justifyContent: "flex-end", marginTop: 14 }}>
        <button className="btn" onClick={onClose}>{t("cancel")}</button>
        <button className="btn" disabled={busy} onClick={test}>{t("testTarget")}</button>
        <button className="btn primary" disabled={busy} onClick={save}>{t("save")}</button>
      </div>

      <WindowPicker
        open={pickerOpen}
        onClose={() => setPickerOpen(false)}
        onPick={(t) => {
          setTarget(t);
          setPickerOpen(false);
        }}
      />
    </>
  );
}
