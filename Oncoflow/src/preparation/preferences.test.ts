import { describe, expect, it } from "vitest";

import { loadWorkingFormulaArrangement, saveWorkingFormulaArrangement } from "./preferences";

function memoryStorage(initial: string | null = null) {
  let value = initial;
  return {
    getItem: () => value,
    setItem: (_key: string, next: string) => { value = next; },
  };
}

describe("working formula arrangement preference", () => {
  it("remembers the last selected arrangement", () => {
    const storage = memoryStorage();
    saveWorkingFormulaArrangement("drug", storage);
    expect(loadWorkingFormulaArrangement(storage)).toBe("drug");
  });

  it("falls back to the order view for invalid stored values", () => {
    expect(loadWorkingFormulaArrangement(memoryStorage("unknown"))).toBe("order");
  });
});
