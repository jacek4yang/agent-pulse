import { useEffect, useMemo, useState } from "react";
import { api, errorMessage } from "../lib/api";
import type { TitleMatchMode, WindowCandidate, WindowTarget } from "../lib/types";
import { Modal } from "./ui";
import { useI18n } from "../lib/i18n";

/** Build the WindowTarget chosen from a live window. */
export function targetFromCandidate(c: WindowCandidate, mode: TitleMatchMode): WindowTarget {
  return {
    last_hwnd: c.hwnd,
    process_id: c.process_id,
    process_name: c.process_name,
    executable_path: c.executable_path,
    title: c.title,
    title_match_mode: mode,
  };
}

export function WindowPicker({
  open, onClose, onPick,
}: {
  open: boolean;
  onClose: () => void;
  onPick: (target: WindowTarget, candidate: WindowCandidate) => void;
}) {
  const [windows, setWindows] = useState<WindowCandidate[]>([]);
  const [filter, setFilter] = useState("");
  const [selected, setSelected] = useState<WindowCandidate | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const { t } = useI18n();

  useEffect(() => {
    if (!open) return;
    setLoading(true);
    api
      .listWindows()
      .then(setWindows)
      .catch((e) => setError(errorMessage(e)))
      .finally(() => setLoading(false));
  }, [open]);

  const filtered = useMemo(() => {
    const f = filter.toLowerCase();
    return windows
      .filter((w) => !f || w.title.toLowerCase().includes(f) || w.process_name.toLowerCase().includes(f))
      .sort((a, b) => a.process_name.localeCompare(b.process_name));
  }, [windows, filter]);

  if (!open) return null;

  return (
    <Modal title={t("selectTargetWindow")} onClose={onClose}>
      {error && <div className="error-banner">{error}</div>}
      <div className="row" style={{ marginBottom: 10, flex: "none" }}>
        <input
          className="input"
          placeholder={t("filterPlaceholder")}
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
          autoFocus
        />
        <button className="btn small" style={{ flex: "none" }} onClick={() => {
          setLoading(true);
          api.listWindows().then(setWindows).finally(() => setLoading(false));
        }}>
          {t("refresh")}
        </button>
      </div>
      <div style={{ maxHeight: "44vh", overflowY: "auto" }}>
        {loading && <div className="muted" style={{ padding: 12 }}>{t("loadingWindows")}</div>}
        {!loading && filtered.length === 0 && (
          <div className="muted" style={{ padding: 12 }}>{t("noVisibleWindows")}</div>
        )}
        {filtered.map((w) => (
          <div
            key={w.hwnd}
            className={`win-row${selected?.hwnd === w.hwnd ? " selected" : ""}`}
            onClick={() => setSelected(w)}
            onDoubleClick={() => onPick(targetFromCandidate(w, "contains"), w)}
          >
            <WindowIcon processName={w.process_name} />
            <div style={{ flex: 1, minWidth: 0 }}>
              <div className="win-title" title={w.title}>{w.title}</div>
              <div className="win-sub">{w.process_name} · PID {w.process_id}</div>
            </div>
          </div>
        ))}
      </div>
      <div className="row" style={{ marginTop: 14, flex: "none", justifyContent: "flex-end" }}>
        <button className="btn" onClick={onClose}>{t("cancel")}</button>
        <button
          className="btn primary"
          disabled={!selected}
          onClick={() => selected && onPick(targetFromCandidate(selected, "contains"), selected)}
        >
          {t("select")}
        </button>
      </div>
    </Modal>
  );
}

/** Deterministic letter-avatar "icon" per process (no external assets). */
export function WindowIcon({ processName, size = 22 }: { processName: string; size?: number }) {
  const letter = (processName || "?").replace(/\.exe$/i, "").charAt(0).toUpperCase();
  let hash = 0;
  for (let i = 0; i < processName.length; i++) hash = (hash * 31 + processName.charCodeAt(i)) | 0;
  const hue = Math.abs(hash) % 360;
  return (
    <div
      style={{
        width: size, height: size, borderRadius: 5, flexShrink: 0,
        background: `hsl(${hue} 45% 45%)`,
        color: "#fff", fontSize: size * 0.5, fontWeight: 650,
        display: "flex", alignItems: "center", justifyContent: "center",
      }}
      aria-hidden
    >
      {letter}
    </div>
  );
}
