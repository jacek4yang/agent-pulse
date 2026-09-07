import { useState } from "react";
import { api, errorMessage } from "../lib/api";
import { Card, Toggle } from "../components/ui";
import type { Settings, Theme } from "../lib/types";

export function SettingsPage({
  settings, onChanged,
}: { settings: Settings; onChanged: () => void }) {
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
          <h1>Settings</h1>
          <div className="page-sub">{saved ? "Saved ✓" : "Changes are saved automatically."}</div>
        </div>
      </div>
      {error && <div className="error-banner">{error}</div>}

      <Card title="Appearance">
        <div className="field">
          <span className="field-label">Theme</span>
          <select className="input" value={settings.theme} onChange={(e) => update({ theme: e.target.value as Theme })}>
            <option value="system">System</option>
            <option value="light">Light</option>
            <option value="dark">Dark</option>
          </select>
        </div>
      </Card>

      <Card title="Notifications">
        <Toggle
          label="Notify on success"
          hint="Desktop notification when a sequence completes."
          checked={settings.notify_on_success}
          onChange={(v) => update({ notify_on_success: v })}
        />
        <Toggle
          label="Notify on failure"
          hint="Desktop notification when a sequence aborts — including when no input was sent."
          checked={settings.notify_on_failure}
          onChange={(v) => update({ notify_on_failure: v })}
        />
      </Card>

      <Card title="Behavior">
        <Toggle
          label="Abort on focus loss"
          hint="Stop the sequence if the target stops being the foreground window (recommended)."
          checked={settings.abort_on_focus_loss}
          onChange={(v) => update({ abort_on_focus_loss: v })}
        />
        <div className="field" style={{ marginTop: 10 }}>
          <span className="field-label">History limit</span>
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

      <Card title="Startup">
        <Toggle
          label="Start with Windows"
          hint="Launch Agent Pulse when you sign in."
          checked={settings.start_with_windows}
          onChange={(v) => update({ start_with_windows: v })}
        />
        <Toggle
          label="Start minimized"
          hint="Begin with the main window hidden in the tray."
          checked={settings.start_minimized}
          onChange={(v) => update({ start_minimized: v })}
        />
      </Card>
    </>
  );
}
