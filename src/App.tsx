import { useCallback, useEffect, useState } from "react";
import { api, subscribeToEvents } from "./lib/api";
import type { HistoryRecord, ScheduledTask, Settings } from "./lib/types";
import { Dashboard } from "./pages/Dashboard";
import { Editor } from "./pages/Editor";
import { History } from "./pages/History";
import { SettingsPage } from "./pages/Settings";
import { DEFAULT_SETTINGS } from "./lib/types";

type Page = "dashboard" | "history" | "settings";

export default function App() {
  const [page, setPage] = useState<Page>("dashboard");
  const [tasks, setTasks] = useState<ScheduledTask[]>([]);
  const [history, setHistory] = useState<HistoryRecord[]>([]);
  const [settings, setSettings] = useState<Settings>(DEFAULT_SETTINGS);
  const [editing, setEditing] = useState<ScheduledTask | null>(null);
  const [editorOpen, setEditorOpen] = useState(false);

  const refresh = useCallback(async () => {
    const [t, h, s] = await Promise.all([
      api.listTasks().catch(() => []),
      api.getHistory().catch(() => []),
      api.getSettings().catch(() => DEFAULT_SETTINGS),
    ]);
    setTasks(t);
    setHistory(h);
    setSettings(s);
  }, []);

  useEffect(() => {
    refresh();
    const unsubs = subscribeToEvents({
      onTaskStarted: refresh,
      onTaskFinished: refresh,
      onSchedulerUpdated: () => api.listTasks().then(setTasks).catch(() => {}),
    });
    return () => {
      unsubs.then((fns) => fns.forEach((u) => u()));
    };
  }, [refresh]);

  // Theme: system preference is default; explicit choice sets data-theme.
  useEffect(() => {
    if (settings.theme === "system") {
      document.documentElement.removeAttribute("data-theme");
    } else {
      document.documentElement.setAttribute("data-theme", settings.theme);
    }
  }, [settings.theme]);

  if (editorOpen) {
    return (
      <div className="app">
        <main className="main" style={{ paddingTop: 30 }}>
          <div className="main-inner">
            <Editor
              task={editing}
              onClose={() => setEditorOpen(false)}
              onChanged={refresh}
            />
          </div>
        </main>
      </div>
    );
  }

  return (
    <div className="app">
      <nav className="sidebar">
        <div className="brand">
          <div className="brand-mark" aria-hidden />
          <span className="brand-name">Agent Pulse</span>
        </div>
        <button className={`nav-item${page === "dashboard" ? " active" : ""}`} onClick={() => setPage("dashboard")}>
          ◈ Dashboard
          <span className="nav-count">{tasks.filter((t) => t.enabled).length}</span>
        </button>
        <button className={`nav-item${page === "history" ? " active" : ""}`} onClick={() => setPage("history")}>
          ☰ History
        </button>
        <button className={`nav-item${page === "settings" ? " active" : ""}`} onClick={() => setPage("settings")}>
          ⚙ Settings
        </button>
        <div className="sidebar-footer">
          <button
            className="btn small"
            style={{ width: "100%" }}
            onClick={() => {
              setEditing(null);
              setEditorOpen(true);
            }}
          >
            + New Automation
          </button>
          <div style={{ marginTop: 10 }}>Scheduling runs in Rust — closing the window keeps it alive.</div>
        </div>
      </nav>
      <main className="main">
        <div className="main-inner">
          {page === "dashboard" && (
            <Dashboard
              tasks={tasks}
              onEdit={(t) => {
                setEditing(t);
                setEditorOpen(true);
              }}
              onChanged={refresh}
            />
          )}
          {page === "history" && <History records={history} onChanged={refresh} />}
          {page === "settings" && <SettingsPage settings={settings} onChanged={refresh} />}
        </div>
      </main>
    </div>
  );
}
