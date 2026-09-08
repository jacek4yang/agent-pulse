import { createContext, useContext } from "react";
import type { Language } from "./types";

// ---------------------------------------------------------------------------
// Dictionaries. Keys are stable identifiers; en and zh must stay in sync.
// ---------------------------------------------------------------------------

export const STRINGS = {
  // app / nav
  appName: { en: "Agent Pulse", zh: "Agent Pulse" },
  navDashboard: { en: "Dashboard", zh: "仪表盘" },
  navHistory: { en: "History", zh: "历史" },
  navSettings: { en: "Settings", zh: "设置" },
  newAutomation: { en: "+ New Automation", zh: "+ 新建自动化" },
  sidebarHint: {
    en: "Scheduling runs in Rust — closing the window keeps it alive.",
    zh: "调度由 Rust 负责 —— 关闭窗口后仍会继续运行。",
  },

  // dashboard
  dashboardTitle: { en: "Dashboard", zh: "仪表盘" },
  dashboardSub: {
    en: "Schedule an automation or manage existing ones.",
    zh: "创建新的自动化，或管理已有的自动化。",
  },
  quickAutomation: { en: "Quick Automation", zh: "快速自动化" },
  target: { en: "Target", zh: "目标窗口" },
  noWindowSelected: { en: "No window selected", zh: "尚未选择窗口" },
  select: { en: "Select…", zh: "选择…" },
  schedule: { en: "Schedule", zh: "时间" },
  schedAfter: { en: "After", zh: "倒计时" },
  schedAt: { en: "At (exact time)", zh: "定时（精确时间）" },
  schedEvery: { en: "Every (recurring)", zh: "重复（循环）" },
  hours: { en: "hours", zh: "小时" },
  minutes: { en: "minutes", zh: "分" },
  seconds: { en: "seconds", zh: "秒" },
  custom: { en: "Custom", zh: "自定义" },
  firesAt: { en: "Fires at", zh: "将于以下时间执行" },
  localTime: { en: "(local time)", zh: "（本地时间）" },
  timezone: { en: "Timezone", zh: "时区" },
  timeInputHint: {
    en: "Type a time like 2026-09-08 13:26:05, or pick with the calendar.",
    zh: "可直接键盘输入，如 2026-09-08 13:26:05、Sep 8th 2026 1:26 AM。",
  },
  invalidTimeFormat: {
    en: "Unrecognized date/time — try 2026-09-08 13:26:05 or “Sep 8th 2026 1:26 AM”.",
    zh: "无法识别的时间格式 —— 请尝试 2026-09-08 13:26:05 或 Sep 8th 2026 1:26 AM。",
  },
  utcEquivalent: { en: "UTC", zh: "UTC" },
  localEquivalent: { en: "your local time", zh: "你的本地时间" },
  unknownZoneWarning: {
    en: "This zone will be validated by the scheduler when you start.",
    zh: "该时区将在启动时由调度器校验。",
  },
  actions: { en: "Actions", zh: "动作" },
  preset: { en: "Preset", zh: "预设" },
  customFlow: { en: "Custom flow", zh: "自定义流程" },
  startAutomation: { en: "Start Automation", zh: "启动自动化" },
  starting: { en: "Starting…", zh: "启动中…" },
  automations: { en: "Automations", zh: "自动化" },
  noAutomations: { en: "No automations yet", zh: "还没有自动化" },
  noAutomationsHint: {
    en: "Use Quick Automation above to schedule your first sequence.",
    zh: "使用上方的“快速自动化”创建第一个任务。",
  },
  selectTargetFirst: { en: "Select a target window first.", zh: "请先选择目标窗口。" },

  // task rows
  nextRun: { en: "Next run", zh: "下次执行" },
  last: { en: "Last", zh: "上次" },
  due: { en: "due", zh: "已到点" },
  pause: { en: "Pause", zh: "暂停" },
  resume: { en: "Resume", zh: "恢复" },
  runNow: { en: "Run Now", zh: "立即执行" },
  edit: { en: "Edit", zh: "编辑" },
  paused: { en: "paused", zh: "已暂停" },

  // schedule labels
  schedAfterLabel: { en: "After", zh: "倒计时" },
  schedAtLabel: { en: "At", zh: "定时" },
  schedEveryLabel: { en: "Every", zh: "每" },
  hUnit: { en: "h", zh: "小时" },
  mUnit: { en: "m", zh: "分" },
  sUnit: { en: "s", zh: "秒" },

  // editor
  editAutomation: { en: "Edit Automation", zh: "编辑自动化" },
  newAutomationTitle: { en: "New Automation", zh: "新建自动化" },
  editorSub: {
    en: "Actions run strictly in order against the verified target window.",
    zh: "动作将严格按顺序执行，且仅在验证后的目标窗口中输入。",
  },
  name: { en: "Name", zh: "名称" },
  namePlaceholder: { en: "Codex 5-hour Continue", zh: "Codex 5 小时 Continue" },
  anyTitle: { en: "(any title)", zh: "（任意标题）" },
  titleMatchMode: { en: "Title match mode", zh: "标题匹配方式" },
  matchExact: { en: "Exact", zh: "完全一致" },
  matchContains: { en: "Contains", zh: "包含" },
  matchRegex: { en: "Regex", zh: "正则" },
  matchAny: { en: "Any", zh: "任意" },
  misfirePolicy: { en: "Misfire policy", zh: "错过执行策略" },
  misfireRun: { en: "Run immediately when overdue", zh: "超时后立即执行" },
  misfireSkip: { en: "Skip missed occurrence", zh: "跳过错过的次数" },
  actionsCount: { en: "Actions", zh: "动作" },
  cancel: { en: "Cancel", zh: "取消" },
  save: { en: "Save", zh: "保存" },
  testTarget: { en: "Test Target", zh: "测试目标" },
  resolved: { en: "Resolved:", zh: "解析成功：" },

  // window picker
  selectTargetWindow: { en: "Select Target Window", zh: "选择目标窗口" },
  filterPlaceholder: { en: "Filter by title or process…", zh: "按标题或进程过滤…" },
  refresh: { en: "Refresh", zh: "刷新" },
  loadingWindows: { en: "Loading windows…", zh: "正在加载窗口…" },
  noVisibleWindows: { en: "No visible windows match.", zh: "没有匹配的可见窗口。" },

  // action editor
  addAction: { en: "Add action…", zh: "添加动作…" },
  configure: { en: "Configure…", zh: "详细配置…" },
  editAction: { en: "Edit Action", zh: "编辑动作" },
  addActionTitle: { en: "Add Action", zh: "添加动作" },
  saveAction: { en: "Save Action", zh: "保存动作" },
  actionFocus: { en: "Focus Target", zh: "聚焦目标窗口" },
  actionRestore: { en: "Restore Target", zh: "还原目标窗口" },
  actionTypeText: { en: "Type Text", zh: "输入文本" },
  actionPressKey: { en: "Press Key", zh: "按键" },
  actionCombo: { en: "Key Combination", zh: "组合键" },
  actionDelay: { en: "Delay", zh: "等待" },
  actionNotify: { en: "Notify", zh: "通知" },
  textToType: { en: "Text to type", zh: "要输入的文本" },
  key: { en: "Key", zh: "按键" },
  count: { en: "Count", zh: "次数" },
  intervalMs: { en: "Interval (ms)", zh: "间隔（毫秒）" },
  comboHint: { en: "Combination (e.g. Ctrl+C, Ctrl+Shift+P)", zh: "组合键（如 Ctrl+C、Ctrl+Shift+P）" },
  milliseconds: { en: "Milliseconds", zh: "毫秒" },
  message: { en: "Message", zh: "消息" },

  // history
  historyTitle: { en: "History", zh: "历史" },
  historySub: {
    en: "Bounded execution log with structured errors.",
    zh: "有上限的执行记录，包含结构化错误信息。",
  },
  clear: { en: "Clear", zh: "清空" },
  noExecutions: { en: "No executions yet", zh: "还没有执行记录" },
  noExecutionsHint: { en: "Runs will appear here with their outcome.", zh: "执行结果会显示在这里。" },

  // settings
  settingsTitle: { en: "Settings", zh: "设置" },
  saved: { en: "Saved ✓", zh: "已保存 ✓" },
  autoSaved: { en: "Changes are saved automatically.", zh: "更改会自动保存。" },
  appearance: { en: "Appearance", zh: "外观" },
  theme: { en: "Theme", zh: "主题" },
  langSystem: { en: "System", zh: "跟随系统" },
  langEn: { en: "English", zh: "English" },
  langZh: { en: "中文 (Simplified)", zh: "简体中文" },
  light: { en: "Light", zh: "浅色" },
  dark: { en: "Dark", zh: "深色" },
  notifications: { en: "Notifications", zh: "通知" },
  notifySuccess: { en: "Notify on success", zh: "成功时通知" },
  notifySuccessHint: {
    en: "Desktop notification when a sequence completes.",
    zh: "序列执行完成时弹出桌面通知。",
  },
  notifyFailure: { en: "Notify on failure", zh: "失败时通知" },
  notifyFailureHint: {
    en: "Desktop notification when a sequence aborts — including when no input was sent.",
    zh: "序列中止时弹出桌面通知（包括未发送任何输入的情况）。",
  },
  behavior: { en: "Behavior", zh: "行为" },
  abortOnFocusLoss: { en: "Abort on focus loss", zh: "失焦时中止" },
  abortOnFocusLossHint: {
    en: "Stop the sequence if the target stops being the foreground window (recommended).",
    zh: "当目标窗口不再处于前台时停止执行（推荐开启）。",
  },
  historyLimit: { en: "History limit", zh: "历史上限" },
  startup: { en: "Startup", zh: "启动" },
  startWithWindows: { en: "Start with Windows", zh: "开机自启" },
  startWithWindowsHint: {
    en: "Launch Agent Pulse when you sign in.",
    zh: "登录 Windows 时自动启动 Agent Pulse。",
  },
  startMinimized: { en: "Start minimized", zh: "启动时最小化" },
  startMinimizedHint: {
    en: "Begin with the main window hidden in the tray.",
    zh: "启动时主窗口隐藏到托盘。",
  },
  language: { en: "Language", zh: "语言" },
} as const;

export type StringKey = keyof typeof STRINGS;

export type UiLanguage = "en" | "zh";

/** Resolve the effective UI language from the stored setting. */
export function resolveLanguage(setting: Language): UiLanguage {
  if (setting === "en") return "en";
  if (setting === "zh") return "zh";
  return navigator.language.toLowerCase().startsWith("zh") ? "zh" : "en";
}

export function translate(lang: UiLanguage, key: StringKey): string {
  return STRINGS[key][lang];
}

/** Locale used for date/time previews per UI language. */
export function localeOf(lang: UiLanguage): string {
  return lang === "zh" ? "zh-CN" : "en-US";
}

export interface I18n {
  lang: UiLanguage;
  locale: string;
  t: (key: StringKey) => string;
}

export const I18nContext = createContext<I18n>({
  lang: "en",
  locale: "en-US",
  t: (key) => STRINGS[key].en,
});

export function useI18n(): I18n {
  return useContext(I18nContext);
}
