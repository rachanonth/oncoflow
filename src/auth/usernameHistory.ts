import { createContext } from "react";
import type { ConnectionSettings } from "../api/transport";
import type { AuthState } from "../types/auth";

export const UsernameHistoryScope = createContext<string | null>(null);
const LIMIT = 20;

export function usernameHistoryKey(settings: ConnectionSettings): string {
  const identity = settings.mode === "client"
    ? ["client", settings.host.trim().toLowerCase(), settings.port, settings.certificateFingerprint]
    : ["local", settings.dataDirectory.replace(/\\/g, "/").replace(/\/+$/, "").toLowerCase()];
  return `oncoflow.username-history.v1:${encodeURIComponent(JSON.stringify(identity))}`;
}

function cleanUsernames(values: unknown): string[] {
  if (!Array.isArray(values)) return [];
  const seen = new Set<string>();
  return values.filter((value): value is string => {
    if (typeof value !== "string" || value !== value.trim() || [...value].length < 3 || [...value].length > 64 || /[\s\p{Cc}]/u.test(value)) return false;
    // Match SQLite's ASCII NOCASE username comparison.
    const identity = value.replace(/[A-Z]/g, (letter) => letter.toLowerCase());
    if (seen.has(identity)) return false;
    seen.add(identity);
    return true;
  }).slice(0, LIMIT);
}

export function readUsernameHistory(key: string | null): string[] {
  if (!key) return [];
  try { return cleanUsernames(JSON.parse(window.localStorage.getItem(key) ?? "[]")); }
  catch { return []; }
}

export function rememberSuccessfulUsername(key: string | null, state: AuthState): void {
  if (!key || !state.authenticated || !state.currentUser) return;
  const usernames = cleanUsernames([state.currentUser.username, ...readUsernameHistory(key)]);
  try { window.localStorage.setItem(key, JSON.stringify(usernames)); }
  catch { /* Optional convenience storage must never block authentication. */ }
}

export function clearUsernameHistory(key: string | null): boolean {
  if (!key) return true;
  try { window.localStorage.removeItem(key); return true; }
  catch { return false; }
}
