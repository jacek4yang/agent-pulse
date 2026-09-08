// Keyboard-friendly datetime parsing + timezone helpers for exact-time
// scheduling. All conversions mirror the Rust backend (chrono-tz semantics).

export const COMMON_TIMEZONES: string[] = [
  "local",
  "utc",
  "UTC+8",
  "Asia/Shanghai",
  "Asia/Hong_Kong",
  "Asia/Taipei",
  "Asia/Singapore",
  "Asia/Tokyo",
  "Asia/Seoul",
  "Asia/Kolkata",
  "Asia/Dubai",
  "Europe/London",
  "Europe/Paris",
  "Europe/Berlin",
  "Europe/Moscow",
  "America/New_York",
  "America/Chicago",
  "America/Denver",
  "America/Los_Angeles",
  "America/Sao_Paulo",
  "Australia/Sydney",
  "Pacific/Auckland",
];

export type ParsedDateTime = {
  year: number;
  month: number; // 1-12
  day: number;
  hour: number;
  minute: number;
  second: number;
};

const MONTHS: Record<string, number> = {
  jan: 1, january: 1, fev: 2, feb: 2, february: 2, mar: 3, march: 3,
  apr: 4, april: 4, may: 5, jun: 6, june: 6, jul: 7, july: 7,
  aug: 8, august: 8, sep: 9, sept: 9, september: 9, oct: 10, october: 10,
  nov: 11, november: 11, dec: 12, december: 12,
};

/**
 * Parse a datetime typed freely at the keyboard. Supported shapes:
 *   2026-09-08 13:26:05        2026-09-08T13:26:05
 *   2026/09/08 13:26           2026-09-08 1:26:05 PM
 *   Sep 8th, 2026 1:26:05 AM   8 Sep 2026 13:26
 * Returns null when nothing matches.
 */
export function parseDateTimeText(input: string): ParsedDateTime | null {
  const text = input.trim().replace(/\s+/g, " ");
  if (!text) return null;

  // Split an optional AM/PM tail (also Chinese 上午/下午).
  const ampmMatch = text.match(/\s*(AM|PM|上午|下午)\s*$/i);
  const ampm = ampmMatch
    ? (ampmMatch[1].toUpperCase().startsWith("P") || ampmMatch[1] === "下午" ? "pm" : "am")
    : null;
  const body = (ampmMatch ? text.slice(0, ampmMatch.index) : text).trim().replace(/[T]/, " ");

  const twoDigitYear = (n: number) => (n < 100 ? 2000 + n : n);
  const to24 = (h: number): number => {
    if (!ampm) return h;
    if (ampm === "am") return h === 12 ? 0 : h;
    return h === 12 ? 12 : h + 12;
  };

  // ISO / slash lead: 2026-09-08 | 2026/9/8 | 26-9-8
  let m = body.match(/^(\d{2,4})[-/](\d{1,2})[-/](\d{1,2})(?:\s+(\d{1,2}):(\d{2})(?::(\d{2}))?)?$/);
  if (m) {
    const hour = to24(m[4] ? Number(m[4]) : 0);
    return {
      year: twoDigitYear(Number(m[1])),
      month: Number(m[2]),
      day: Number(m[3]),
      hour,
      minute: m[5] ? Number(m[5]) : 0,
      second: m[6] ? Number(m[6]) : 0,
    };
  }

  // "Sep 8th, 2026" or "Sep 8 2026" lead
  m = body.match(/^([A-Za-z]{3,9})\.?\s+(\d{1,2})(?:st|nd|rd|th)?,?\s+(\d{2,4})(?:\s+(\d{1,2}):(\d{2})(?::(\d{2}))?)?$/);
  if (m) {
    const month = MONTHS[m[1].toLowerCase()];
    if (!month) return null;
    return {
      year: twoDigitYear(Number(m[3])),
      month,
      day: Number(m[2]),
      hour: to24(m[4] ? Number(m[4]) : 0),
      minute: m[5] ? Number(m[5]) : 0,
      second: m[6] ? Number(m[6]) : 0,
    };
  }

  // "8 Sep 2026" lead
  m = body.match(/^(\d{1,2})(?:st|nd|rd|th)?\s+([A-Za-z]{3,9})\.?,?\s+(\d{2,4})(?:\s+(\d{1,2}):(\d{2})(?::(\d{2}))?)?$/);
  if (m) {
    const month = MONTHS[m[2].toLowerCase()];
    if (!month) return null;
    return {
      year: twoDigitYear(Number(m[3])),
      month,
      day: Number(m[1]),
      hour: to24(m[4] ? Number(m[4]) : 0),
      minute: m[5] ? Number(m[5]) : 0,
      second: m[6] ? Number(m[6]) : 0,
    };
  }

  return null;
}

/** Is the timezone string usable (by Intl for previews, or a fixed offset)? */
export function isTimezonePreviewable(tz: string): boolean {
  if (tz === "local") return true;
  if (/^(?:utc|gmt)?[+-]\d{1,2}(:?\d{2})?$/i.test(tz.trim())) return true;
  try {
    new Intl.DateTimeFormat("en-US", { timeZone: tz });
    return true;
  } catch {
    return false;
  }
}

/** Offset (ms) of `tz` at the given instant. */
function tzOffsetMs(date: Date, tz: string): number {
  const dtf = new Intl.DateTimeFormat("en-US", {
    timeZone: tz,
    hour12: false,
    year: "numeric", month: "2-digit", day: "2-digit",
    hour: "2-digit", minute: "2-digit", second: "2-digit",
  });
  const map: Record<string, string> = {};
  for (const p of dtf.formatToParts(date)) map[p.type] = p.value;
  const asUtc = Date.UTC(
    Number(map.year), Number(map.month) - 1, Number(map.day),
    Number(map.hour) % 24, Number(map.minute), Number(map.second),
  );
  return asUtc - date.getTime();
}

function fixedOffsetMs(tz: string): number | null {
  const m = tz.trim().match(/^(?:utc|gmt)?([+-])(\d{1,2})(?::?(\d{2}))?$/i);
  if (!m) return null;
  const sign = m[1].toLowerCase() === "-" ? -1 : 1;
  return sign * (Number(m[2]) * 3600 + Number(m[3] ?? 0) * 60) * 1000;
}

/**
 * Convert a wall-clock time in `tz` to the equivalent UTC instant.
 * Mirrors the Rust `TzKind::with_ymd_and_hms` behavior for preview purposes.
 */
export function zonedWallClockToUtc(parsed: ParsedDateTime, tz: string): Date | null {
  const { year, month, day, hour, minute, second } = parsed;
  if (tz === "local" || tz === "") {
    return new Date(year, month - 1, day, hour, minute, second);
  }
  if (/^(?:utc|gmt)?[+-]\d{1,2}(:?\d{2})?$/i.test(tz.trim())) {
    const off = fixedOffsetMs(tz);
    return off === null ? null : new Date(Date.UTC(year, month - 1, day, hour, minute, second) - off);
  }
  try {
    // Two-pass conversion handles DST boundaries well enough for previews.
    const naive = Date.UTC(year, month - 1, day, hour, minute, second);
    const off1 = tzOffsetMs(new Date(naive), tz);
    let out = new Date(naive - off1);
    const off2 = tzOffsetMs(out, tz);
    if (off2 !== off1) out = new Date(naive - off2);
    return out;
  } catch {
    return null;
  }
}

/** Canonical local "YYYY-MM-DDTHH:MM:SS" string (for datetime-local inputs). */
export function toLocalInputValue(d: Date): string {
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}T${p(d.getHours())}:${p(d.getMinutes())}:${p(d.getSeconds())}`;
}

/** Format a UTC instant in an arbitrary zone, e.g. "09/08 01:26:05". */
export function formatInZone(d: Date, tz: string): string {
  try {
    return new Intl.DateTimeFormat("en-US", {
      timeZone: tz === "local" ? undefined : tz,
      month: "2-digit", day: "2-digit",
      hour: "2-digit", minute: "2-digit", second: "2-digit", hour12: false,
    }).format(d);
  } catch {
    return "—";
  }
}
