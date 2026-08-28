import { describe, expect, it } from "vitest";

import { bangkokLocalDateTimeToUtc, currentBangkokDateTimeValue, displayDate, displayDateTime, displayLocalDateTime, displayTime } from "./dateTime";

describe("global Bangkok date and time display", () => {
  it("formats date-only values with an abbreviated month and two-digit Buddhist year", () => {
    expect(displayDate("2026-08-25")).toBe("25/08/2569");
    expect(displayDate("2024-02-29")).toBe("29/02/2567");
  });

  it("converts UTC timestamps to Bangkok time in 24-hour format", () => {
    expect(displayDateTime("2026-08-24T18:05:00Z")).toBe("25/08/2569 01:05");
    expect(displayDateTime("2026-08-24 18:05:00")).toBe("25/08/2569 01:05");
  });

  it("preserves order and treatment wall-clock time as Bangkok local time", () => {
    expect(displayLocalDateTime("2026-08-25T09:07")).toBe("25/08/2569 09:07");
  });

  it("formats time-only values in 24-hour hours and minutes", () => {
    expect(displayTime("9:07:30")).toBe("09:07");
    expect(displayTime("23:59")).toBe("23:59");
  });

  it("keeps fallbacks and unrecognized legacy values intact", () => {
    expect(displayDate(null)).toBe("—");
    expect(displayDateTime(null, "Not recorded")).toBe("Not recorded");
    expect(displayDate("legacy date")).toBe("legacy date");
    expect(displayDateTime("legacy timestamp")).toBe("legacy timestamp");
  });

  it("creates Bangkok form values and converts selected instants to UTC", () => {
    expect(currentBangkokDateTimeValue(new Date("2026-08-25T02:07:00Z"))).toBe("2026-08-25T09:07");
    expect(bangkokLocalDateTimeToUtc("2026-08-25T09:07")).toBe("2026-08-25T02:07:00.000Z");
  });
});
