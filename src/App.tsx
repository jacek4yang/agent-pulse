import { useCallback, useEffect, useMemo, useState } from "react";
import { api, subscribeToEvents } from "./lib/api";
import type { HistoryRecord, ScheduledTask, Settings } from "./lib/types";
import { DEFAULT_SETTINGS } from "./lib/types";
import { I18nContext, resolveLanguage, translate, type StringKey } from "./lib/i18n";
import { localeOf } from "./lib/i18n";
import { Dashboard } from "./pages/Dashboard";
import { Editor } from "./pages/Editor";
import { History } from "./pages/History";
import { SettingsPage } from "./pages/Settings";

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

  const i18n = useMemo(() => {
    const lang = resolveLanguage(settings.language);
    return {
      lang,
      locale: localeOf(lang),
      t: (key: StringKey) => translate(lang, key),
    };
  }, [settings.language]);

  if (editorOpen) {
    return (
      <I18nContext.Provider value={i18n}>
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
      </I18nContext.Provider>
    );
  }

  const t = i18n.t;

  return (
    <I18nContext.Provider value={i18n}>
      <div className="app">
        <nav className="sidebar">
          <div className="brand">
            <div className="brand-mark" aria-hidden />
            <span className="brand-name">{t("appName")}</span>
          </div>
          <button className={`nav-item${page === "dashboard" ? " active" : ""}`} onClick={() => setPage("dashboard")}>
            ◈ {t("navDashboard")}
            <span className="nav-count">{tasks.filter((task) => task.enabled).length}</span>
          </button>
          <button className={`nav-item${page === "history" ? " active" : ""}`} onClick={() => setPage("history")}>
            ☰ {t("navHistory")}
          </button>
          <button className={`nav-item${page === "settings" ? " active" : ""}`} onClick={() => setPage("settings")}>
            ⚙ {t("navSettings")}
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
              {t("newAutomation")}
            </button>
            <div style={{ marginTop: 10 }}>{t("sidebarHint")}</div>
          </div>
        </nav>
        <main className="main">
          <div className="main-inner">
            {page === "dashboard" && (
              <Dashboard
                tasks={tasks}
                onEdit={(task) => {
                  setEditing(task);
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
    </I18nContext.Provider>
  );
}
