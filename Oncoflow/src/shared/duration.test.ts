import { describe, expect, it } from "vitest";

import {
  convertDurationValue,
  displayDuration,
  parseDuration,
  serializeDuration,
} from "./duration";

describe("duration values", () => {
  it("reads legacy rate phrases and fractional hours", () => {
    expect(parseDuration("in 90 min")).toMatchObject({ value: "90", unit: "minute" });
    expect(parseDuration("drip in 1/2 hr")).toMatchObject({ value: "0.5", unit: "hour" });
  });

  it("reads Access clock values as elapsed hours", () => {
    expect(parseDuration("04:00:00", { allowClock: true })).toMatchObject({ value: "4", unit: "hour" });
    expect(displayDuration("08:30:00", true)).toBe("8.5 hr");
  });

  it("converts and serializes minute and hour values", () => {
    expect(convertDurationValue("90", "minute", "hour")).toBe("1.5");
    expect(convertDurationValue("1.5", "hour", "minute")).toBe("90");
    expect(serializeDuration("2", "hour")).toBe("2 hr");
  });

  it("preserves an unrecognized legacy instruction for display", () => {
    expect(displayDuration("slowly push")).toBe("slowly push");
  });
});
