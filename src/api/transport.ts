import { invoke as desktopInvoke } from "@tauri-apps/api/core";

export interface ConnectionSettings {
  mode: "standalone" | "client" | "host";
  host: string;
  port: number;
  certificateFingerprint: string;
  dataDirectory: string;
}

let clientMode = false;
export function setClientMode(value: boolean) { clientMode = value; }
export function isClientMode() { return clientMode; }
export function getConnectionSettings() { return desktopInvoke<ConnectionSettings>("connection_settings"); }
export function saveConnectionSettings(host: string, port: number, certificatePath: string | null, mode: ConnectionSettings["mode"] = host ? "client" : "standalone", dataDirectory: string | null = null) {
  return desktopInvoke<void>("save_connection_settings", { host, port, certificatePath, mode, dataDirectory });
}

export interface ServerStatus {
  mode: ConnectionSettings["mode"];
  running: boolean;
  databaseReady: boolean;
  signedInConnections: number;
  addresses: string[];
  port: number;
  dataDirectory: string | null;
  uptimeSeconds: number | null;
  message: string;
}
export function getServerStatus() { return desktopInvoke<ServerStatus>("server_status"); }
export function startHostServer() { return desktopInvoke<void>("start_host_server"); }
export function stopHostServer(exit = false) { return desktopInvoke<void>(exit ? "stop_server_and_exit" : "stop_host_server", { confirmed: true }); }
export function exportServerCertificate(destination: string) { return desktopInvoke<void>("export_server_certificate", { destination }); }

export async function invoke<T>(command: string, args: Record<string, unknown> = {}): Promise<T> {
  try {
    return await (clientMode
      ? desktopInvoke<T>("lan_invoke", { command, args })
      : desktopInvoke<T>(command, args));
  } catch (error) {
    const code = (error as { code?: string } | null)?.code;
    if (clientMode && ["edit_conflict", "connection_lost", "authentication_required"].includes(code ?? "")) {
      window.dispatchEvent(new CustomEvent("oncoflow-connection-attention", { detail: error }));
    }
    throw error;
  }
}
