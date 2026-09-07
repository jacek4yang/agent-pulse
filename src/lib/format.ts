import type { Action, Key, ScheduledTask } from "./types";

/** Human label for an action, shown in the editor and summaries. */
export function describeAction(a: Action): string {
  switch (a.type) {
    case "focus_target":
      return "Focus Target";
    case "restore_target":
      return "Restore Target";
    case "type_text":
      return `Type Text ${JSON.stringify(a.text)}`;
    case "press_key":
      return `Press Key ${keyLabel(a.key)}${a.count > 1 ? ` ×${a.count}` : ""}`;
    case "key_combination":
      return `Combo ${a.keys.map(keyLabel).join("+")}`;
    case "delay":
      return `Wait ${a.milliseconds} ms`;
    case "notify":
      return `Notify ${JSON.stringify(a.message)}`;
  }
}

export function keyLabel(k: Key): string {
  if (typeof k === "string") {
    return k.replace(/_/g, "").replace(/^./, (c) => c.toUpperCase());
  }
  if ("letter" in k) return k.letter.toUpperCase();
  if ("digit" in k) return String(k.digit);
  return `F${k.function}`;
}

/** Compact countdown like `4h 52m 11s`. */
export function formatCountdown(targetIso: string | null, now: number): string {
  if (!targetIso) return "—";
  const ms = new Date(targetIso).getTime() - now;
  if (ms <= 0) return "due";
  const s = Math.floor(ms / 1000);
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const sec = s % 60;
  if (h > 0) return `${h}h ${m}m ${sec}s`;
  if (m > 0) return `${m}m ${sec}s`;
  return `${sec}s`;
}

export function formatSchedule(t: ScheduledTask): string {
  switch (t.schedule.kind) {
    case "after": {
      const parts: string[] = [];
      if (t.schedule.hours) parts.push(`${t.schedule.hours}h`);
      if (t.schedule.minutes) parts.push(`${t.schedule.minutes}m`);
      if (t.schedule.seconds) parts.push(`${t.schedule.seconds}s`);
      return `After ${parts.join(" ") || "0s"}`;
    }
    case "at": {
      const p2 = (n: number) => String(n).padStart(2, "0");
      return `At ${t.schedule.year}-${p2(t.schedule.month)}-${p2(t.schedule.day)} ${p2(t.schedule.hour)}:${p2(t.schedule.minute)}:${p2(t.schedule.second)}`;
    }
    case "every": {
      const s = t.schedule.interval_seconds;
      if (s % 3600 === 0) return `Every ${s / 3600}h`;
      if (s % 60 === 0) return `Every ${s / 60}m`;
      return `Every ${s}s`;
    }
  }
}

export function formatTime(iso: string | null): string {
  if (!iso) return "—";
  const d = new Date(iso);
  return d.toLocaleString(undefined, {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

export function formatDuration(startedIso: string, finishedIso: string): string {
  const ms = new Date(finishedIso).getTime() - new Date(startedIso).getTime();
  if (ms < 1000) return `${ms} ms`;
  return `${(ms / 1000).toFixed(1)} s`;
}
