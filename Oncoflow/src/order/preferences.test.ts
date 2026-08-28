import { describe, expect, it } from "vitest";

import { loadLatestOrderContext, saveLatestOrderContext } from "./preferences";

function memoryStorage(initial: string | null = null) {
  let value = initial;
  return {
    getItem: () => value,
    setItem: (_key: string, next: string) => { value = next; },
  };
}

describe("latest order context preferences", () => {
  it("remembers the last doctor and ward selections", () => {
    const storage = memoryStorage();

    saveLatestOrderContext({ doctorId: "12", wardId: "7" }, storage);

    expect(loadLatestOrderContext(storage)).toEqual({ doctorId: "12", wardId: "7" });
  });

  it("ignores malformed or stale preference data safely", () => {
    const storage = memoryStorage('{"doctorId":"not-an-id","wardId":4}');

    expect(loadLatestOrderContext(storage)).toEqual({ doctorId: "", wardId: "" });
  });
});
