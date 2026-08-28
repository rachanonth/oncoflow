const LATEST_ORDER_CONTEXT_KEY = "oncoflow.order.latest-context.v1";

export interface LatestOrderContext {
  doctorId: string;
  wardId: string;
}

type PreferenceStorage = Pick<Storage, "getItem" | "setItem">;

const EMPTY_CONTEXT: LatestOrderContext = { doctorId: "", wardId: "" };

export function loadLatestOrderContext(storage = browserStorage()): LatestOrderContext {
  if (!storage) return { ...EMPTY_CONTEXT };
  try {
    const parsed = JSON.parse(storage.getItem(LATEST_ORDER_CONTEXT_KEY) ?? "null") as Partial<LatestOrderContext> | null;
    return {
      doctorId: validId(parsed?.doctorId),
      wardId: validId(parsed?.wardId),
    };
  } catch {
    return { ...EMPTY_CONTEXT };
  }
}

export function saveLatestOrderContext(context: LatestOrderContext, storage = browserStorage()): void {
  if (!storage) return;
  try {
    storage.setItem(LATEST_ORDER_CONTEXT_KEY, JSON.stringify({
      doctorId: validId(context.doctorId),
      wardId: validId(context.wardId),
    }));
  } catch {
    // Preferences must never prevent an order from being saved.
  }
}

function validId(value: unknown): string {
  return typeof value === "string" && /^\d+$/.test(value) ? value : "";
}

function browserStorage(): PreferenceStorage | undefined {
  try {
    return typeof window === "undefined" ? undefined : window.localStorage;
  } catch {
    return undefined;
  }
}
