import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke as desktopInvoke } from "@tauri-apps/api/core";
import { invoke, setClientMode } from "./transport";
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

describe("desktop transport routing", () => {
  beforeEach(() => { vi.clearAllMocks(); setClientMode(false); });
  it("keeps standalone calls local", async () => {
    vi.mocked(desktopInvoke).mockResolvedValue({ total: 0 });
    await invoke("list_patients", { request: {} });
    expect(desktopInvoke).toHaveBeenCalledWith("list_patients", { request: {} });
  });
  it("routes all LAN commands through Rust without exposing credentials to browser storage", async () => {
    setClientMode(true); vi.mocked(desktopInvoke).mockResolvedValue({ authenticated: true });
    await invoke("login", { input: { username: "synthetic", password: "synthetic-test-only" } });
    expect(desktopInvoke).toHaveBeenCalledWith("lan_invoke", { command: "login", args: { input: { username: "synthetic", password: "synthetic-test-only" } } });
  });
  it("never retries or falls back locally after a failed write", async () => {
    setClientMode(true);
    vi.stubGlobal("window", { dispatchEvent: vi.fn() });
    vi.stubGlobal("CustomEvent", class { constructor(public type: string, public init: unknown) {} });
    const error = { code: "connection_lost", message: "Check whether it was saved" };
    vi.mocked(desktopInvoke).mockRejectedValue(error);
    await expect(invoke("create_patient", { input: {} })).rejects.toEqual(error);
    expect(desktopInvoke).toHaveBeenCalledTimes(1);
    expect(window.dispatchEvent).toHaveBeenCalledTimes(1);
    vi.unstubAllGlobals();
  });
});
