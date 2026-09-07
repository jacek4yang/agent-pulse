import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  Action,
  HistoryRecord,
  MisfirePolicy,
  Preset,
  ScheduledTask,
  Settings,
  Schedule,
  WindowCandidate,
  WindowTarget,
} from "./types";

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  return invoke<T>(command, args);
}

export const api = {
  listTasks: () => call<ScheduledTask[]>("list_tasks"),
  createTask: (input: {
    name: string;
    target: WindowTarget;
    schedule: Schedule;
    actions: Action[];
    misfirePolicy: MisfirePolicy;
  }) =>
    call<ScheduledTask>("create_task", {
      name: input.name,
      target: input.target,
      schedule: input.schedule,
      actions: input.actions,
      misfirePolicy: input.misfirePolicy,
    }),
  updateTask: (task: ScheduledTask) => call<ScheduledTask>("update_task", { task }),
  deleteTask: (taskId: string) => call<void>("delete_task", { taskId }),
  setTaskEnabled: (taskId: string, enabled: boolean) =>
    call<ScheduledTask>("set_task_enabled", { taskId, enabled }),
  runTaskNow: (taskId: string) => call<HistoryRecord>("run_task_now", { taskId }),
  listWindows: () => call<WindowCandidate[]>("list_windows"),
  testWindowTarget: (target: WindowTarget) =>
    call<WindowCandidate>("test_window_target", { target }),
  getHistory: () => call<HistoryRecord[]>("get_history"),
  clearHistory: () => call<void>("clear_history"),
  getSettings: () => call<Settings>("get_settings"),
  updateSettings: (settings: Settings) => call<void>("update_settings", { settings }),
  createQuickTask: (input: {
    target: WindowTarget;
    preset: Preset;
    hours: number;
    minutes: number;
    confirmDelayMs: number;
  }) =>
    call<ScheduledTask>("create_quick_task", {
      target: input.target,
      preset: input.preset,
      hours: input.hours,
      minutes: input.minutes,
      confirmDelayMs: input.confirmDelayMs,
    }),
};

/** Subscribe to all backend state events; returns a cleanup function. */
export async function subscribeToEvents(handlers: {
  onTaskStarted?: () => void;
  onTaskFinished?: () => void;
  onSchedulerUpdated?: () => void;
}): Promise<UnlistenFn[]> {
  const subs: Promise<UnlistenFn>[] = [
    listen("task-started", () => handlers.onTaskStarted?.()),
    listen("task-completed", () => handlers.onTaskFinished?.()),
    listen("task-failed", () => handlers.onTaskFinished?.()),
    listen("task-created", () => handlers.onSchedulerUpdated?.()),
    listen("task-updated", () => handlers.onSchedulerUpdated?.()),
    listen("scheduler-updated", () => handlers.onSchedulerUpdated?.()),
  ];
  return Promise.all(subs);
}

export function errorMessage(err: unknown): string {
  if (typeof err === "string") return err;
  if (err && typeof err === "object" && "message" in err) {
    return String((err as { message: unknown }).message);
  }
  return String(err);
}

export function errorCode(err: unknown): string | null {
  if (err && typeof err === "object" && "code" in err) {
    return String((err as { code: unknown }).code);
  }
  return null;
}
