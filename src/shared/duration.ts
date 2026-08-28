export type DurationUnit = "minute" | "hour";

export interface ParsedDuration {
  original: string;
  value: string | null;
  unit: DurationUnit;
}

export function parseDuration(
  rawValue: string | null,
  options: { allowClock?: boolean; defaultUnit?: DurationUnit } = {},
): ParsedDuration {
  const original = rawValue?.trim() ?? "";
  const defaultUnit = options.defaultUnit ?? "minute";

  if (options.allowClock) {
    const clock = /^(\d+):([0-5]\d)(?::([0-5]\d))?$/.exec(original);
    if (clock) {
      const seconds = Number(clock[1]) * 3600 + Number(clock[2]) * 60 + Number(clock[3] ?? 0);
      return {
        original,
        value: compactNumber(seconds / 3600),
        unit: "hour",
      };
    }
  }

  const text = /^(?:(?:over|in|drip\s+in)\s+)?(-?(?:\d+(?:\.\d*)?|\.\d+)|\d+\/\d+)\s*(minutes?|mins?|hours?|hrs?)$/i.exec(original);
  if (!text) return { original, value: null, unit: defaultUnit };

  return {
    original,
    value: fractionToDecimal(text[1]),
    unit: text[2].toLowerCase().startsWith("h") ? "hour" : "minute",
  };
}

export function convertDurationValue(
  value: string,
  from: DurationUnit,
  to: DurationUnit,
): string {
  if (from === to || !value.trim()) return value;
  const numeric = Number(value);
  if (!Number.isFinite(numeric)) return value;
  return compactNumber(from === "minute" ? numeric / 60 : numeric * 60);
}

export function serializeDuration(value: string, unit: DurationUnit): string {
  const trimmed = value.trim();
  return trimmed ? `${trimmed} ${unit === "minute" ? "min" : "hr"}` : "";
}

export function displayDuration(rawValue: string | null, allowClock = false): string | null {
  const parsed = parseDuration(rawValue, { allowClock, defaultUnit: "hour" });
  if (!parsed.original) return null;
  if (parsed.value === null) return parsed.original;
  return serializeDuration(parsed.value, parsed.unit);
}

function fractionToDecimal(value: string): string {
  if (!value.includes("/")) return value;
  const [numerator, denominator] = value.split("/").map(Number);
  if (!Number.isFinite(numerator) || !Number.isFinite(denominator) || denominator === 0) {
    return value;
  }
  return compactNumber(numerator / denominator);
}

function compactNumber(value: number): string {
  return Number(value.toFixed(6)).toString();
}
