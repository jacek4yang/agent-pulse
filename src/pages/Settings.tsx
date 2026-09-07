import { useState } from "react";
import { api, errorMessage } from "../lib/api";
import { Card, Toggle } from "../components/ui";
import { useI18n } from "../lib/i18n";
import type { Language, Settings, Theme } from "../lib/types";

export function SettingsPage({
  settings, onChanged,
}: { settings: Settings; onChanged: () => void }) {
  const { t } = useI18n();
  const [saved, setSaved] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const update = async (patch: Partial<Settings>) => {
    const next = { ...settings, ...patch };
    try {
      await api.updateSettings(next);
      setSaved(true);
      setTimeout(() => setSaved(false), 1500);
      onChanged();
    } catch (e) {
      setError(errorMessage(e));
    }
  };

  return (
    <>
      <div className="page-header">
        <div>
          <h1>{t("settingsTitle")}</h1>
          <div className="page-sub">{saved ? t("saved") : t("autoSaved")}</div>
        </div>
      </div>
      {error && <div className="error-banner">{error}</div>}

      <Card title={t("appearance")}>
        <div className="field">
          <span className="field-label">{t("language")}</span>
          <select className="input" value={settings.language} onChange={(e) => update({ language: e.target.value as Language })}>
            <option value="system">{t("langSystem")}</option>
            <option value="en">{t("langEn")}</option>
            <option value="zh">{t("langZh")}</option>
          </select>
        </div>
        <div className="field">
          <span className="field-label">{t("theme")}</span>
          <select className="input" value={settings.theme} onChange={(e) => update({ theme: e.target.value as Theme })}>
            <option value="system">{t("langSystem")}</option>
            <option value="light">{t("light")}</option>
            <option value="dark">{t("dark")}</option>
          </select>
        </div>
      </Card>

      <Card title={t("notifications")}>
        <Toggle
          label={t("notifySuccess")}
          hint={t("notifySuccessHint")}
          checked={settings.notify_on_success}
          onChange={(v) => update({ notify_on_success: v })}
        />
        <Toggle
          label={t("notifyFailure")}
          hint={t("notifyFailureHint")}
          checked={settings.notify_on_failure}
          onChange={(v) => update({ notify_on_failure: v })}
        />
      </Card>

      <Card title={t("behavior")}>
        <Toggle
          label={t("abortOnFocusLoss")}
          hint={t("abortOnFocusLossHint")}
          checked={settings.abort_on_focus_loss}
          onChange={(v) => update({ abort_on_focus_loss: v })}
        />
        <div className="field" style={{ marginTop: 10 }}>
          <span className="field-label">{t("historyLimit")}</span>
          <input
            className="input"
            type="number"
            min={10}
            max={10000}
            value={settings.history_limit}
            onChange={(e) => update({ history_limit: Number(e.target.value) })}
          />
        </div>
      </Card>

      <Card title={t("startup")}>
        <Toggle
          label={t("startWithWindows")}
          hint={t("startWithWindowsHint")}
          checked={settings.start_with_windows}
          onChange={(v) => update({ start_with_windows: v })}
        />
        <Toggle
          label={t("startMinimized")}
          hint={t("startMinimizedHint")}
          checked={settings.start_minimized}
          onChange={(v) => update({ start_minimized: v })}
        />
      </Card>
    </>
  );
}
