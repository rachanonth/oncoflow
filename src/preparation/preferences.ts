const WORKING_FORMULA_ARRANGEMENT_KEY = "oncoflow.preparation.working-formula-arrangement.v1";

export type WorkingFormulaArrangement = "order" | "drug";

type PreferenceStorage = Pick<Storage, "getItem" | "setItem">;

export function loadWorkingFormulaArrangement(storage = browserStorage()): WorkingFormulaArrangement {
  if (!storage) return "order";
  try {
    return storage.getItem(WORKING_FORMULA_ARRANGEMENT_KEY) === "drug" ? "drug" : "order";
  } catch {
    return "order";
  }
}

export function saveWorkingFormulaArrangement(arrangement: WorkingFormulaArrangement, storage = browserStorage()): void {
  if (!storage) return;
  try {
    storage.setItem(WORKING_FORMULA_ARRANGEMENT_KEY, arrangement);
  } catch {
    // A display preference must never interrupt preparation work.
  }
}

function browserStorage(): PreferenceStorage | undefined {
  try {
    return typeof window === "undefined" ? undefined : window.localStorage;
  } catch {
    return undefined;
  }
}
