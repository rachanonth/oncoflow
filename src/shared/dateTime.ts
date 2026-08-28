const BANGKOK_TIME_ZONE = "Asia/Bangkok";
const BANGKOK_OFFSET = "+07:00";
const bangkokParts = new Intl.DateTimeFormat("en-GB", {
  timeZone: BANGKOK_TIME_ZONE,
  year: "numeric",
  month: "2-digit",
  day: "2-digit",
  hour: "2-digit",
  minute: "2-digit",
  hourCycle: "h23",
});

export function displayDate(value: string | null, fallback = "—"): string {
  if (!value) return fallback;
  const match = /^(\d{4})-(\d{2})-(\d{2})/.exec(value.trim());
  if (!match) return value;
  const year = Number(match[1]);
  const month = Number(match[2]);
  const day = Number(match[3]);
  const probe = new Date(Date.UTC(year, month - 1, day));
  if (probe.getUTCFullYear() !== year || probe.getUTCMonth() !== month - 1 || probe.getUTCDate() !== day) return value;
  return formatDateParts(year, month, day);
}

/** Formats an instant in Bangkok time. Offset-less database timestamps are treated as UTC. */
export function displayDateTime(value: string | null, fallback = "—"): string {
  return formatDateTime(value, fallback, false);
}

/** Formats an order/treatment wall-clock value that is already expressed in Bangkok local time. */
export function displayLocalDateTime(value: string | null, fallback = "Not recorded"): string {
  return formatDateTime(value, fallback, true);
}

export function displayTime(value: string | null, fallback = "—"): string {
  if (!value) return fallback;
  const match = /^(\d{1,2}):(\d{2})(?::\d{2})?$/.exec(value.trim());
  if (!match) return value;
  const hour = Number(match[1]);
  const minute = Number(match[2]);
  if (hour > 23 || minute > 59) return value;
  return `${String(hour).padStart(2, "0")}:${match[2]}`;
}

function formatDateTime(value: string | null, fallback: string, localWallClock: boolean): string {
  if (!value) return fallback;
  const normalized = value.trim().replace(" ", "T");
  const hasOffset = /(?:Z|[+-]\d{2}:?\d{2})$/i.test(normalized);
  const date = new Date(hasOffset ? normalized : `${normalized}${localWallClock ? BANGKOK_OFFSET : "Z"}`);
  if (Number.isNaN(date.getTime())) return value;

  const parts = Object.fromEntries(
    bangkokParts.formatToParts(date)
      .filter((part) => part.type !== "literal")
      .map((part) => [part.type, part.value]),
  );
  return `${formatDateParts(Number(parts.year), Number(parts.month), Number(parts.day))} ${parts.hour}:${parts.minute}`;
}

function formatDateParts(year: number, month: number, day: number): string {
  return `${String(day).padStart(2, "0")}/${String(month).padStart(2, "0")}/${year + 543}`;
}

export function currentBangkokDateTimeValue(now = new Date()): string {
  const parts = Object.fromEntries(
    bangkokParts.formatToParts(now)
      .filter((part) => part.type !== "literal")
      .map((part) => [part.type, part.value]),
  );
  return `${parts.year}-${parts.month}-${parts.day}T${parts.hour}:${parts.minute}`;
}

export function bangkokLocalDateTimeToUtc(value: string): string {
  if (!/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}$/.test(value)) return value;
  const date = new Date(`${value}:00${BANGKOK_OFFSET}`);
  return Number.isNaN(date.getTime()) ? value : date.toISOString();
}
