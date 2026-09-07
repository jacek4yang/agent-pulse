// Mirror of the Rust domain model (src-tauri/src/model). Keep in sync —
// serde tags are the contract.

export type Theme = "system" | "light" | "dark";
export type Language = "system" | "en" | "zh";
export type MisfirePolicy = "run_immediately" | "skip";
export type TitleMatchMode = "exact" | "contains" | "regex" | "any";

export type Schedule =
  | { kind: "after"; hours: number; minutes: number; seconds: number }
  | { kind: "at"; year: number; month: number; day: number; hour: number; minute: number; second: number }
  | { kind: "every"; interval_seconds: number };

export type Key =
  | "enter" | "escape" | "tab" | "space" | "backspace" | "delete"
  | "home" | "end" | "page_up" | "page_down"
  | "up" | "down" | "left" | "right"
  | "ctrl" | "shift" | "alt"
  | { letter: string }
  | { digit: number }
  | { function: number };

export type Action =
  | { type: "focus_target" }
  | { type: "restore_target" }
  | { type: "type_text"; text: string }
  | { type: "press_key"; key: Key; count: number; interval_ms: number }
  | { type: "key_combination"; keys: Key[] }
  | { type: "delay"; milliseconds: number }
  | { type: "notify"; message: string };

export type WindowTarget = {
  last_hwnd?: number | null;
  process_id?: number | null;
  executable_path?: string | null;
  process_name?: string | null;
  title?: string | null;
  title_match_mode: TitleMatchMode;
};

export type WindowCandidate = {
  hwnd: number;
  title: string;
  process_id: number;
  process_name: string;
  executable_path: string | null;
  is_visible: boolean;
};

export type ScheduledTask = {
  id: string;
  name: string;
  enabled: boolean;
  target: WindowTarget;
  schedule: Schedule;
  actions: Action[];
  misfire_policy: MisfirePolicy;
  created_at: string;
  updated_at: string;
  last_run_at: string | null;
  next_run_at: string | null;
};

export type ExecutionOutcome =
  | { outcome: "success" }
  | { outcome: "failure"; error_code: string; error_message: string };

export type HistoryRecord = {
  id: string;
  task_id: string;
  task_name: string;
  scheduled_at: string;
  started_at: string;
  finished_at: string;
  target_description: string;
  outcome: ExecutionOutcome;
};

export type Settings = {
  theme: Theme;
  language: Language;
  notify_on_success: boolean;
  notify_on_failure: boolean;
  start_with_windows: boolean;
  start_minimized: boolean;
  default_misfire_policy: MisfirePolicy;
  history_limit: number;
  abort_on_focus_loss: boolean;
};

export type Preset =
  | "continue" | "continue_confirm" | "confirm_continue" | "double_enter" | "empty_submit";

export const PRESETS: { id: Preset; label: string }[] = [
  { id: "continue_confirm", label: "Continue + Confirm" },
  { id: "continue", label: "Continue" },
  { id: "confirm_continue", label: "Confirm + Continue" },
  { id: "double_enter", label: "Double Enter" },
  { id: "empty_submit", label: "Empty Submit" },
];

export type AppErrorShape = { code: string; message: string };

export const DEFAULT_SETTINGS: Settings = {
  theme: "system",
  language: "system",
  notify_on_success: true,
  notify_on_failure: true,
  start_with_windows: false,
  start_minimized: false,
  default_misfire_policy: "run_immediately",
  history_limit: 1000,
  abort_on_focus_loss: true,
};
