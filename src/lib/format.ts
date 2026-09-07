import type { Action, Key, ScheduledTask } from "./types";
import type { UiLanguage } from "./i18n";

const WORDS = {
  after: { en: "After", zh: "倒计时" },
  at: { en: "At", zh: "定时" },
  every: { en: "Every", zh: "每" },
  typeText: { en: "Type Text", zh: "输入文本" },
  pressKey: { en: "Press Key", zh: "按键" },
  combo: { en: "Combo", zh: "组合键" },
  wait: { en: "Wait", zh: "等待" },
  notify: { en: "Notify", zh: "通知" },
  focus: { en: "Focus Target", zh: "聚焦目标窗口" },
  restore: { en: "Restore Target", zh: "还原目标窗口" },
  due: { en: "due", zh: "已到点" },
} as const;

/** Human label for an action, shown in the editor and summaries. */
export function describeAction(a: Action, lang: UiLanguage): string {
  switch (a.type) {
    case "focus_target":
      return WORDS.focus[lang];
    case "restore_target":
      return WORDS.restore[lang];
    case "type_text":
      return `${WORDS.typeText[lang]} ${JSON.stringify(a.text)}`;
    case "press_key":
      return `${WORDS.pressKey[lang]} ${keyLabel(a.key)}${a.count > 1 ? ` ×${a.count}` : ""}`;
    case "key_combination":
      return `${WORDS.combo[lang]} ${a.keys.map(keyLabel).join("+")}`;
    case "delay":
      return `${WORDS.wait[lang]} ${a.milliseconds} ms`;
    case "notify":
      return `${WORDS.notify[lang]} ${JSON.stringify(a.message)}`;
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
export function formatCountdown(targetIso: string | null, now: number, lang: UiLanguage): string {
  if (!targetIso) return "—";
  const ms = new Date(targetIso).getTime() - now;
  if (ms <= 0) return WORDS.due[lang];
  const s = Math.floor(ms / 1000);
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const sec = s % 60;
  if (h > 0) return `${h}h ${m}m ${sec}s`;
  if (m > 0) return `${m}m ${sec}s`;
  return `${sec}s`;
}

export function formatSchedule(t: ScheduledTask, lang: UiLanguage): string {
  switch (t.schedule.kind) {
    case "after": {
      const parts: string[] = [];
      if (t.schedule.hours) parts.push(`${t.schedule.hours}h`);
      if (t.schedule.minutes) parts.push(`${t.schedule.minutes}m`);
      if (t.schedule.seconds) parts.push(`${t.schedule.seconds}s`);
      const span = parts.join(" ") || "0s";
      return lang === "zh" ? `${WORDS.after.zh} ${span}` : `${WORDS.after.en} ${span}`;
    }
    case "at": {
      const p2 = (n: number) => String(n).padStart(2, "0");
      const dt = `${t.schedule.year}-${p2(t.schedule.month)}-${p2(t.schedule.day)} ${p2(t.schedule.hour)}:${p2(t.schedule.minute)}:${p2(t.schedule.second)}`;
      return lang === "zh" ? `${WORDS.at.zh} ${dt}` : `${WORDS.at.en} ${dt}`;
    }
    case "every": {
      const s = t.schedule.interval_seconds;
      const span = s % 3600 === 0 ? `${s / 3600}h` : s % 60 === 0 ? `${s / 60}m` : `${s}s`;
      return lang === "zh" ? `${WORDS.every.zh} ${span}` : `${WORDS.every.en} ${span}`;
    }
  }
}

export function formatTime(iso: string | null, locale = "en-US"): string {
  if (!iso) return "—";
  const d = new Date(iso);
  return d.toLocaleString(locale, {
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

/** "Sep 8th, 2026 1:26:05 AM" (en) or "2026年9月8日 01:26:05" (zh). */
export function formatExactTime(d: Date, lang: UiLanguage): string {
  if (Number.isNaN(d.getTime())) return "—";
  if (lang === "zh") {
    const p2 = (n: number) => String(n).padStart(2, "0");
    return `${d.getFullYear()}年${d.getMonth() + 1}月${d.getDate()}日 ${p2(d.getHours())}:${p2(d.getMinutes())}:${p2(d.getSeconds())}`;
  }
  const day = d.getDate();
  const suffix =
    day % 10 === 1 && day !== 11 ? "st" :
    day % 10 === 2 && day !== 12 ? "nd" :
    day % 10 === 3 && day !== 13 ? "rd" : "th";
  const month = d.toLocaleString("en-US", { month: "short" });
  const time = d.toLocaleString("en-US", {
    hour: "numeric", minute: "2-digit", second: "2-digit", hour12: true,
  });
  return `${month} ${day}${suffix}, ${d.getFullYear()} ${time}`;
}
