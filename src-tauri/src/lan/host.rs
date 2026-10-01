//! Local desktop controls. These commands are never in the LAN API allowlist.
use super::{client::ConnectionState, error, server::RunningServer};
use serde::Serialize;
use serde_json::Value;
use std::{path::PathBuf, sync::Mutex};
use tauri::{Manager, State};

struct HostInner {
    server: Option<RunningServer>,
    last_error: Option<String>,
}
pub(crate) struct HostManager {
    enabled: bool,
    directory: PathBuf,
    port: u16,
    addresses: Vec<String>,
    inner: Mutex<HostInner>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ServerStatus {
    mode: &'static str,
    running: bool,
    database_ready: bool,
    signed_in_connections: usize,
    addresses: Vec<String>,
    port: u16,
    data_directory: Option<String>,
    uptime_seconds: Option<u64>,
    message: String,
}

impl HostManager {
    pub(crate) fn new(connection: &ConnectionState) -> Self {
        Self {
            enabled: connection.is_host(),
            directory: connection.data_directory(),
            port: connection.port(),
            addresses: if connection.is_host() {
                local_addresses()
            } else {
                Vec::new()
            },
            inner: Mutex::new(HostInner {
                server: None,
                last_error: None,
            }),
        }
    }
    pub(crate) fn running(&self) -> bool {
        self.inner
            .lock()
            .ok()
            .and_then(|v| v.server.as_ref().map(|s| s.running()))
            .unwrap_or(false)
    }
    pub(crate) fn start(&self) -> Result<(), Value> {
        if !self.enabled {
            return Err(error(
                "host_mode_required",
                "Choose Host server on this PC and restart OncoFlow first.",
            ));
        }
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| error("server_error", "Server controls are unavailable."))?;
        if inner.server.as_ref().is_some_and(|s| s.running()) {
            return Ok(());
        }
        inner.server = None;
        match RunningServer::start(&self.directory, ([0, 0, 0, 0], self.port).into()) {
            Ok(server) => {
                inner.server = Some(server);
                inner.last_error = None;
                Ok(())
            }
            Err(_) => {
                let message = "Server could not start. Check that the data folder is writable, the database and certificate are valid, and no other OncoFlow server uses this folder or port.";
                inner.last_error = Some(message.into());
                Err(error("server_start_failed", message))
            }
        }
    }
    pub(crate) fn stop(&self) -> Result<(), Value> {
        if !self.enabled {
            return Err(error(
                "host_mode_required",
                "This PC is not the server host.",
            ));
        }
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| error("server_error", "Server controls are unavailable."))?;
        if let Some(mut server) = inner.server.take() {
            server.stop();
        }
        inner.last_error = None;
        Ok(())
    }
}

#[tauri::command]
pub(crate) async fn server_status(app: tauri::AppHandle) -> Result<ServerStatus, Value> {
    tauri::async_runtime::spawn_blocking(move || {
        let connection = app.state::<ConnectionState>();
        let host = app.state::<HostManager>();
        let (running, count, uptime, last_error, port) = {
            let inner = host
                .inner
                .lock()
                .map_err(|_| error("server_error", "Server status is unavailable."))?;
            (
                inner.server.as_ref().is_some_and(|s| s.running()),
                inner.server.as_ref().map_or(0, |s| s.sessions()),
                inner.server.as_ref().map(|s| s.started.elapsed().as_secs()),
                inner.last_error.clone(),
                inner
                    .server
                    .as_ref()
                    .map_or(host.port, |s| s.address.port()),
            )
        };
        let health = if connection.is_client() && (!connection.is_host() || running) {
            Some(connection.probe())
        } else {
            None
        };
        let ready = health.as_ref().is_some_and(|r| r.is_ok());
        let message = last_error.unwrap_or_else(|| match health {
            Some(Ok(_)) => "Server online · database ready".into(),
            Some(Err(e)) => e["message"].as_str().unwrap_or("Server unavailable").into(),
            None if connection.is_host() => {
                "Server stopped. Start it to use the shared workspace.".into()
            }
            None => "Standalone mode · database on this PC".into(),
        });
        Ok(ServerStatus {
            mode: if connection.is_host() {
                "host"
            } else if connection.is_client() {
                "client"
            } else {
                "standalone"
            },
            running: if connection.is_host() { running } else { ready },
            database_ready: ready,
            signed_in_connections: count,
            addresses: host.addresses.clone(),
            port,
            data_directory: connection
                .is_host()
                .then(|| host.directory.display().to_string()),
            uptime_seconds: uptime,
            message,
        })
    })
    .await
    .map_err(|_| error("server_error", "Server status could not be read."))?
}

#[tauri::command]
pub(crate) async fn start_host_server(app: tauri::AppHandle) -> Result<(), Value> {
    tauri::async_runtime::spawn_blocking(move || app.state::<HostManager>().start())
        .await
        .map_err(|_| error("server_error", "Server startup failed."))?
}

#[tauri::command]
pub(crate) async fn stop_host_server(app: tauri::AppHandle, confirmed: bool) -> Result<(), Value> {
    if !confirmed {
        return Err(error(
            "confirmation_required",
            "Stopping the server disconnects every user. Confirm to continue.",
        ));
    }
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<HostManager>().stop()?;
        app.state::<ConnectionState>().disconnect();
        Ok(())
    })
    .await
    .map_err(|_| error("server_error", "Server shutdown failed."))?
}

#[tauri::command]
pub(crate) async fn stop_server_and_exit(
    app: tauri::AppHandle,
    confirmed: bool,
) -> Result<(), Value> {
    stop_host_server(app.clone(), confirmed).await?;
    app.exit(0);
    Ok(())
}

#[tauri::command]
pub(crate) fn export_server_certificate(
    host: State<'_, HostManager>,
    destination: String,
) -> Result<(), Value> {
    if !host.enabled {
        return Err(error(
            "host_mode_required",
            "Only the host PC can export its public certificate.",
        ));
    }
    let source = host.directory.join("server-certificate.der");
    let destination = PathBuf::from(destination);
    if destination.exists() {
        return Err(error(
            "destination_exists",
            "Choose a new filename; existing files are not overwritten.",
        ));
    }
    let certificate = std::fs::read(source).map_err(|_| {
        error(
            "certificate_unavailable",
            "Start the server once to create its public certificate.",
        )
    })?;
    use std::io::Write;
    std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(destination)
        .and_then(|mut f| f.write_all(&certificate))
        .map_err(|_| {
            error(
                "export_failed",
                "The public certificate could not be saved.",
            )
        })
}

fn local_addresses() -> Vec<String> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // Fixed read-only command; no user text is interpolated and no visible console.
        let output = std::process::Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", "Get-NetIPAddress -AddressFamily IPv4 | Where-Object { $_.AddressState -eq 'Preferred' -and $_.IPAddress -ne '127.0.0.1' } | Select-Object -ExpandProperty IPAddress"])
            .creation_flags(0x08000000).output();
        if let Ok(output) = output {
            let mut addresses: Vec<String> = String::from_utf8_lossy(&output.stdout)
                .lines()
                .filter_map(|s| s.trim().parse::<std::net::Ipv4Addr>().ok())
                .filter(|ip| !ip.is_loopback() && !ip.is_link_local())
                .map(|ip| ip.to_string())
                .collect();
            addresses.sort();
            addresses.dedup();
            return addresses;
        }
    }
    Vec::new()
}
