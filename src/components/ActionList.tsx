import { useState } from "react";
import { describeAction, keyLabel } from "../lib/format";
import type { Action, Key } from "../lib/types";
import { Field, Modal } from "./ui";
import { useI18n } from "../lib/i18n";

const NAMED_KEYS: { id: Key; label: string }[] = [
  { id: "enter", label: "Enter" }, { id: "escape", label: "Escape" },
  { id: "tab", label: "Tab" }, { id: "space", label: "Space" },
  { id: "backspace", label: "Backspace" }, { id: "delete", label: "Delete" },
  { id: "up", label: "↑" }, { id: "down", label: "↓" },
  { id: "left", label: "←" }, { id: "right", label: "→" },
  { id: "ctrl", label: "Ctrl" }, { id: "shift", label: "Shift" }, { id: "alt", label: "Alt" },
];

const ACTION_KINDS: { id: Action["type"]; key: "actionFocus" | "actionRestore" | "actionTypeText" | "actionPressKey" | "actionCombo" | "actionDelay" | "actionNotify" }[] = [
  { id: "focus_target", key: "actionFocus" },
  { id: "restore_target", key: "actionRestore" },
  { id: "type_text", key: "actionTypeText" },
  { id: "press_key", key: "actionPressKey" },
  { id: "key_combination", key: "actionCombo" },
  { id: "delay", key: "actionDelay" },
  { id: "notify", key: "actionNotify" },
];

export function ActionList({
  actions, onChange, readOnly = false,
}: { actions: Action[]; onChange?: (next: Action[]) => void; readOnly?: boolean }) {
  const { t, lang } = useI18n();
  const [editing, setEditing] = useState<number | null>(null);

  const move = (i: number, dir: -1 | 1) => {
    const next = [...actions];
    const j = i + dir;
    if (j < 0 || j >= next.length) return;
    [next[i], next[j]] = [next[j], next[i]];
    onChange?.(next);
  };

  return (
    <div>
      {actions.map((a, i) => (
        <div className="action-row" key={i}>
          <span className="action-idx">{i + 1}</span>
          <span className="action-desc" title={describeAction(a, lang)}>{describeAction(a, lang)}</span>
          {!readOnly && (
            <span className="action-controls">
              <button className="icon-btn" title={t("edit")} onClick={() => setEditing(i)}>✎</button>
              <button className="icon-btn" title="Move up" disabled={i === 0} onClick={() => move(i, -1)}>↑</button>
              <button className="icon-btn" title="Move down" disabled={i === actions.length - 1} onClick={() => move(i, 1)}>↓</button>
              <button className="icon-btn" title={t("clear")} onClick={() => onChange?.(actions.filter((_, j) => j !== i))}>✕</button>
            </span>
          )}
        </div>
      ))}
      {!readOnly && (
        <AddAction onAdd={(a) => onChange?.([...actions, a])} />
      )}
      {editing !== null && (
        <EditActionModal
          action={actions[editing]}
          onClose={() => setEditing(null)}
          onSave={(a) => {
            const next = [...actions];
            next[editing] = a;
            onChange?.(next);
            setEditing(null);
          }}
        />
      )}
    </div>
  );
}

function AddAction({ onAdd }: { onAdd: (a: Action) => void }) {
  const { t } = useI18n();
  const [open, setOpen] = useState(false);
  return (
    <>
      <div className="row">
        <select
          className="input"
          value=""
          onChange={(e) => {
            const kind = e.target.value as Action["type"];
            if (!kind) return;
            onAdd(defaultAction(kind));
            e.target.value = "";
          }}
        >
          <option value="">{t("addAction")}</option>
          {ACTION_KINDS.map((k) => (
            <option key={k.id} value={k.id}>{t(k.key)}</option>
          ))}
        </select>
        <button className="btn small" style={{ flex: "none" }} onClick={() => setOpen(true)}>{t("configure")}</button>
      </div>
      {open && (
        <EditActionModal
          onClose={() => setOpen(false)}
          onSave={(a) => {
            onAdd(a);
            setOpen(false);
          }}
        />
      )}
    </>
  );
}

function defaultAction(kind: Action["type"]): Action {
  switch (kind) {
    case "focus_target": return { type: "focus_target" };
    case "restore_target": return { type: "restore_target" };
    case "type_text": return { type: "type_text", text: "continue" };
    case "press_key": return { type: "press_key", key: "enter", count: 1, interval_ms: 0 };
    case "key_combination": return { type: "key_combination", keys: ["ctrl", { letter: "c" }] };
    case "delay": return { type: "delay", milliseconds: 1000 };
    case "notify": return { type: "notify", message: "Done" };
  }
}

function EditActionModal({
  action, onClose, onSave,
}: { action?: Action; onClose: () => void; onSave: (a: Action) => void }) {
  const { t } = useI18n();
  const [kind, setKind] = useState<Action["type"]>(action?.type ?? "type_text");
  const [text, setText] = useState(action?.type === "type_text" ? action.text : "continue");
  const [key, setKey] = useState<Key>(action?.type === "press_key" ? action.key : "enter");
  const [count, setCount] = useState(action?.type === "press_key" ? action.count : 1);
  const [intervalMs, setIntervalMs] = useState(action?.type === "press_key" ? action.interval_ms : 0);
  const [combo, setCombo] = useState<string>(
    action?.type === "key_combination" ? action.keys.map(keyLabel).join("+") : "Ctrl+C",
  );
  const [ms, setMs] = useState(action?.type === "delay" ? action.milliseconds : 1000);
  const [message, setMessage] = useState(action?.type === "notify" ? action.message : "Done");

  const save = () => {
    switch (kind) {
      case "focus_target": return onSave({ type: "focus_target" });
      case "restore_target": return onSave({ type: "restore_target" });
      case "type_text": return onSave({ type: "type_text", text });
      case "press_key": return onSave({ type: "press_key", key, count: Math.max(1, count), interval_ms: intervalMs });
      case "key_combination": return onSave({
        type: "key_combination",
        keys: parseCombo(combo),
      });
      case "delay": return onSave({ type: "delay", milliseconds: Math.max(0, ms) });
      case "notify": return onSave({ type: "notify", message });
    }
  };

  return (
    <Modal title={action ? t("editAction") : t("addActionTitle")} onClose={onClose}>
      <Field label={t("actions")}>
        <select className="input" value={kind} onChange={(e) => setKind(e.target.value as Action["type"])}>
          {ACTION_KINDS.map((k) => <option key={k.id} value={k.id}>{t(k.key)}</option>)}
        </select>
      </Field>
      {kind === "type_text" && (
        <Field label={t("textToType")}>
          <input className="input" value={text} onChange={(e) => setText(e.target.value)} />
        </Field>
      )}
      {kind === "press_key" && (
        <>
          <Field label={t("key")}>
            <select className="input" value={typeof key === "string" ? key : ""} onChange={(e) => setKey(e.target.value as Key)}>
              {NAMED_KEYS.map((k) => <option key={k.id as string} value={k.id as string}>{k.label}</option>)}
            </select>
          </Field>
          <div className="row">
            <Field label={t("count")}><input className="input" type="number" min={1} value={count} onChange={(e) => setCount(Number(e.target.value))} /></Field>
            <Field label={t("intervalMs")}><input className="input" type="number" min={0} value={intervalMs} onChange={(e) => setIntervalMs(Number(e.target.value))} /></Field>
          </div>
        </>
      )}
      {kind === "key_combination" && (
        <Field label={t("comboHint")}>
          <input className="input" value={combo} onChange={(e) => setCombo(e.target.value)} />
        </Field>
      )}
      {kind === "delay" && (
        <Field label={t("milliseconds")}><input className="input" type="number" min={0} value={ms} onChange={(e) => setMs(Number(e.target.value))} /></Field>
      )}
      {kind === "notify" && (
        <Field label={t("message")}><input className="input" value={message} onChange={(e) => setMessage(e.target.value)} /></Field>
      )}
      <div className="row" style={{ justifyContent: "flex-end" }}>
        <button className="btn" onClick={onClose}>{t("cancel")}</button>
        <button className="btn primary" onClick={save}>{t("saveAction")}</button>
      </div>
    </Modal>
  );
}

function parseCombo(spec: string): Key[] {
  return spec
    .split("+")
    .map((p) => p.trim())
    .filter(Boolean)
    .map((token): Key => {
      const t = token.toLowerCase();
      if (["ctrl", "shift", "alt", "enter", "escape", "tab", "space", "up", "down", "left", "right", "delete", "backspace", "home", "end", "page_up", "page_down"].includes(t)) {
        return t as Key;
      }
      if (/^f\d{1,2}$/.test(t)) return { function: Number(t.slice(1)) };
      if (/^\d$/.test(t)) return { digit: Number(t) };
      return { letter: token.charAt(0).toLowerCase() };
    });
}
