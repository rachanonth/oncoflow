import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { AuthState } from "../types/auth";
import type { ConnectionSettings } from "../api/transport";
import { clearUsernameHistory, readUsernameHistory, rememberSuccessfulUsername, usernameHistoryKey } from "./usernameHistory";

const settings: ConnectionSettings = { mode: "standalone", host: "", port: 9443, certificateFingerprint: "", dataDirectory: "C:\\OncoFlow\\data" };
const success = (username: string): AuthState => ({ authenticated: true, needsBootstrap: false, currentUser: { id: 1, username, displayName: "Not stored", role: "admin", userType: "pharmacist" } });

describe("remembered usernames", () => {
  let values: Map<string, string>;
  beforeEach(() => {
    values = new Map();
    vi.stubGlobal("window", { localStorage: {
      getItem: (key: string) => values.get(key) ?? null,
      setItem: (key: string, value: string) => values.set(key, value),
      removeItem: (key: string) => values.delete(key),
    } });
  });
  afterEach(() => vi.unstubAllGlobals());

  it("persists only successful usernames, deduplicates and orders by most recent login", () => {
    const key = usernameHistoryKey(settings);
    rememberSuccessfulUsername(key, success("user.one"));
    rememberSuccessfulUsername(key, success("user.two"));
    rememberSuccessfulUsername(key, success("USER.ONE"));
    rememberSuccessfulUsername(key, { ...success("failed.user"), authenticated: false });
    rememberSuccessfulUsername(key, { authenticated: true, needsBootstrap: false, currentUser: null });
    expect(readUsernameHistory(key)).toEqual(["USER.ONE", "user.two"]);
    expect(values.get(key)).toBe('["USER.ONE","user.two"]');
  });

  it("bounds history, isolates connections and clears only the selected connection", () => {
    const key = usernameHistoryKey(settings);
    const lan = usernameHistoryKey({ ...settings, mode: "client", host: "192.168.1.10", certificateFingerprint: "cert-a" });
    for (let index = 0; index < 25; index++) rememberSuccessfulUsername(key, success(`user.${index}`));
    expect(readUsernameHistory(key)).toHaveLength(20);
    expect(readUsernameHistory(key)[0]).toBe("user.24");
    expect(readUsernameHistory(lan)).toEqual([]);
    rememberSuccessfulUsername(lan, success("lan.user"));
    expect(clearUsernameHistory(key)).toBe(true);
    expect(readUsernameHistory(key)).toEqual([]);
    expect(readUsernameHistory(lan)).toEqual(["lan.user"]);
    expect(usernameHistoryKey({ ...settings, dataDirectory: "c:/oncoflow/data/", mode: "host" })).toBe(key);
    expect(usernameHistoryKey({ ...settings, dataDirectory: "C:/other" })).not.toBe(key);
    expect(usernameHistoryKey({ ...settings, mode: "client", host: "192.168.1.11", certificateFingerprint: "cert-a" })).not.toBe(lan);
    expect(usernameHistoryKey({ ...settings, mode: "client", host: "192.168.1.10", certificateFingerprint: "cert-b" })).not.toBe(lan);
  });

  it("ignores corrupt and invalid stored data and tolerates unavailable storage", () => {
    const key = usernameHistoryKey(settings);
    values.set(key, "invalid json");
    expect(readUsernameHistory(key)).toEqual([]);
    values.set(key, JSON.stringify([null, {}, 123, "", "too long ", "ab", "a\u0000bc", "x".repeat(65), "valid.user", "VALID.USER"]));
    expect(readUsernameHistory(key)).toEqual(["valid.user"]);
    vi.stubGlobal("window", { get localStorage() { throw new Error("Storage unavailable"); } });
    expect(readUsernameHistory(key)).toEqual([]);
    expect(() => rememberSuccessfulUsername(key, success("valid.user"))).not.toThrow();
    expect(clearUsernameHistory(key)).toBe(false);
    expect(readUsernameHistory(null)).toEqual([]);
  });
});
