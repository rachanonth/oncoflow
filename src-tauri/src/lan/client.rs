use super::{argument, encode, error, protocol::*, RpcResult};
use rustls::{
    pki_types::{CertificateDer, ServerName},
    ClientConfig, ClientConnection, RootCertStore, StreamOwned,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    fs,
    net::{IpAddr, SocketAddr, TcpStream},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};
use tauri::State;

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Settings {
    #[serde(default)]
    hosting: bool,
    #[serde(default)]
    data_directory: Option<String>,
    #[serde(default)]
    host: String,
    #[serde(default = "default_port")]
    port: u16,
    #[serde(default)]
    certificate: Vec<u8>,
}
fn default_port() -> u16 {
    7443
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConnectionSettings {
    mode: &'static str,
    host: String,
    port: u16,
    certificate_fingerprint: String,
    data_directory: String,
}

pub(crate) struct ConnectionState {
    settings: Settings,
    path: PathBuf,
    stream: Mutex<Option<StreamOwned<ClientConnection, TcpStream>>>,
}

impl ConnectionState {
    pub(crate) fn load(directory: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let path = directory.join("connection.json");
        let settings = match fs::read(&path) {
            Ok(data) => serde_json::from_slice(&data)?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Settings {
                port: default_port(),
                ..Settings::default()
            },
            Err(e) => return Err(e.into()),
        };
        Ok(Self {
            settings,
            path,
            stream: Mutex::new(None),
        })
    }

    pub(crate) fn is_client(&self) -> bool {
        self.settings.hosting || !self.settings.host.is_empty()
    }

    pub(crate) fn is_host(&self) -> bool {
        self.settings.hosting
    }
    pub(crate) fn port(&self) -> u16 {
        self.settings.port
    }
    pub(crate) fn data_directory(&self) -> PathBuf {
        self.settings
            .data_directory
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(|| self.path.parent().unwrap().to_path_buf())
    }
    fn effective_settings(&self) -> Result<Settings, Value> {
        let mut settings = self.settings.clone();
        if settings.hosting {
            settings.host = "127.0.0.1".into();
            settings.certificate = fs::read(self.data_directory().join("server-certificate.der"))
                .map_err(|_| {
                error(
                    "server_stopped",
                    "Start the server on this PC before signing in.",
                )
            })?;
        }
        Ok(settings)
    }
    pub(crate) fn disconnect(&self) {
        if let Ok(mut stream) = self.stream.lock() {
            *stream = None;
        }
    }
    pub(crate) fn probe(&self) -> RpcResult {
        let settings = self.effective_settings()?;
        let mut stream = connect(&settings)?;
        write_frame(
            &mut stream,
            &Request {
                version: PROTOCOL_VERSION,
                command: "health_status".into(),
                args: json!({}),
            },
        )
        .map_err(|_| error("server_unavailable", "The server did not respond."))?;
        read_frame::<Response>(&mut stream)
            .map_err(|_| error("server_unavailable", "The server did not respond."))?
            .result
    }

    fn call(&self, command: &str, args: Value) -> RpcResult {
        let mut guard = self
            .stream
            .lock()
            .map_err(|_| error("connection_error", "The server connection is unavailable."))?;
        // A reload discards the old session before connecting, including on failure.
        if command == "get_startup_status" {
            *guard = None;
        }
        // Never replay requests after a disconnect: a write may already have committed.
        if guard.is_none() {
            if !matches!(
                command,
                "get_startup_status" | "get_auth_state" | "login" | "health_status"
            ) {
                return Err(error(
                    "authentication_required",
                    "Reconnect and sign in before continuing.",
                ));
            }
            *guard = Some(connect(&self.effective_settings()?)?);
        }
        let stream = guard.as_mut().unwrap();
        let request = Request {
            version: PROTOCOL_VERSION,
            command: command.into(),
            args,
        };
        let response = write_frame(stream, &request).and_then(|_| read_frame::<Response>(stream));
        match response {
            Ok(response) => response.result,
            Err(_) => {
                *guard = None;
                Err(error("connection_lost", "The server connection was lost. A submitted change may have completed. Reload and check the record before trying again."))
            }
        }
    }

    fn invoke(&self, command: &str, args: Value) -> RpcResult {
        if !self.is_client() {
            return Err(error(
                "configuration_error",
                "This desktop is not configured as a LAN client.",
            ));
        }
        match command {
            "list_system_printers"
            | "list_system_label_fonts"
            | "validate_printer_queue"
            | "print_test_label" => {
                self.call("get_current_user", json!({}))?;
                match command {
                    "list_system_label_fonts" => encode(
                        crate::hardware::fonts::list_system_label_fonts().map_err(|_| {
                            error("font_unavailable", "Could not list this PC's label fonts.")
                        })?,
                    ),
                    "list_system_printers" => {
                        encode(crate::hardware::spooler::list_printers().map_err(|_| {
                            error("printer_error", "Could not list this PC's printers.")
                        })?)
                    }
                    "validate_printer_queue" => {
                        let printers = crate::hardware::spooler::list_printers().map_err(|_| {
                            error("printer_error", "Could not list this PC's printers.")
                        })?;
                        let name: Option<String> = argument(&args, "spoolerName")?;
                        Ok(
                            json!({"configuredQueue":name,"available":name.as_ref().is_some_and(|n| printers.contains(n)),"installedQueueCount":printers.len(),"physicalOutputConfirmed":false}),
                        )
                    }
                    _ => {
                        let config = argument(&args, "config")?;
                        let bytes =
                            crate::hardware::renderer::render_test_label(&config).map_err(|e| {
                                serde_json::to_value(crate::hardware::commands::CommandError::from(
                                    e,
                                ))
                                .unwrap()
                            })?;
                        encode(
                            crate::hardware::spooler::submit_raw(
                                &config.spooler_name,
                                "OncoFlow test label",
                                &bytes,
                            )
                            .map_err(|_| {
                                error("printer_error", "This PC's printer did not accept the job.")
                            })?,
                        )
                    }
                }
            }
            "print_preparation_label" | "print_order_preparation_labels" => {
                let config: crate::hardware::LabelPrinterConfig = argument(&args, "config")?;
                let mut args = args;
                args["renderLocally"] = json!(true);
                let prepared = self.call("prepare_client_print", args)?;
                let rendered = (|| {
                    let outputs: Vec<crate::output::PreparationOutput> =
                        argument(&prepared, "outputs")?;
                    crate::hardware::renderer::render_preparation_labels(&outputs, &config).map_err(
                        |e| {
                            serde_json::to_value(crate::hardware::commands::CommandError::from(e))
                                .unwrap()
                        },
                    )
                })();
                let bytes = match rendered {
                    Ok(bytes) => bytes,
                    Err(render_error) => {
                        // No spooler submission occurred, so releasing this pending print is safe.
                        self.call("cancel_client_print", json!({}))?;
                        return Err(render_error);
                    }
                };
                let job = crate::hardware::spooler::submit_raw(&config.spooler_name, "OncoFlow preparation labels", &bytes)
                    .map_err(|_| error("printer_error", "The print submission failed. Check this PC's queue before reconnecting or retrying."))?;
                let outputs = self.call("complete_client_print", json!({})).map_err(|_| error("print_audit_error", "Windows accepted the print job, but the server acknowledgement failed. Check the printer queue before retrying."))?;
                if command == "print_preparation_label" {
                    Ok(json!({"output":outputs[0], "job":job}))
                } else {
                    Ok(json!({"outputs":outputs, "job":job}))
                }
            }
            _ => self.call(command, args),
        }
    }
}

fn connect(settings: &Settings) -> Result<StreamOwned<ClientConnection, TcpStream>, Value> {
    let address: IpAddr = settings
        .host
        .parse()
        .map_err(|_| error("configuration_error", "Enter the server's LAN IP address."))?;
    let mut roots = RootCertStore::empty();
    roots
        .add(CertificateDer::from(settings.certificate.clone()))
        .map_err(|_| {
            error(
                "certificate_error",
                "Select the server's public certificate file.",
            )
        })?;
    let config =
        ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_safe_default_protocol_versions()
            .map_err(|_| error("certificate_error", "TLS configuration failed."))?
            .with_root_certificates(roots)
            .with_no_client_auth();
    let connection =
        ClientConnection::new(Arc::new(config), ServerName::try_from(SERVER_NAME).unwrap())
            .map_err(|_| error("certificate_error", "TLS configuration failed."))?;
    let socket = TcpStream::connect_timeout(
        &SocketAddr::new(address, settings.port),
        Duration::from_secs(5),
    )
    .map_err(|_| {
        error(
            "server_unavailable",
            "Cannot reach the server. Check its address, power, network, and firewall.",
        )
    })?;
    socket
        .set_read_timeout(Some(Duration::from_secs(30)))
        .map_err(|_| error("connection_error", "Could not configure the connection."))?;
    socket
        .set_write_timeout(Some(Duration::from_secs(30)))
        .map_err(|_| error("connection_error", "Could not configure the connection."))?;
    let _ = socket.set_nodelay(true);
    let mut stream = StreamOwned::new(connection, socket);
    while stream.conn.is_handshaking() {
        stream.conn.complete_io(&mut stream.sock).map_err(|_| error("certificate_error", "The server's identity could not be verified. Check the certificate and both PCs' clocks."))?;
    }
    Ok(stream)
}

#[tauri::command]
pub(crate) fn connection_settings(connection: State<'_, ConnectionState>) -> ConnectionSettings {
    ConnectionSettings {
        mode: if connection.is_host() {
            "host"
        } else if connection.is_client() {
            "client"
        } else {
            "standalone"
        },
        host: connection.settings.host.clone(),
        port: connection.settings.port,
        data_directory: connection.data_directory().display().to_string(),
        certificate_fingerprint: if connection.settings.certificate.is_empty() {
            String::new()
        } else {
            format!("{:x}", Sha256::digest(&connection.settings.certificate))
        },
    }
}

#[tauri::command]
pub(crate) async fn save_connection_settings(
    connection: State<'_, ConnectionState>,
    host: String,
    port: u16,
    certificate_path: Option<String>,
    mode: Option<String>,
    data_directory: Option<String>,
) -> Result<(), Value> {
    let hosting = mode.as_deref() == Some("host");
    if let Some(mode) = &mode {
        if !["host", "client", "standalone"].contains(&mode.as_str()) {
            return Err(error("validation", "Choose a connection mode."));
        }
    }
    if hosting {
        if port == 0 {
            return Err(error("validation", "Choose a port from 1 to 65535."));
        }
        let directory = data_directory
            .filter(|s| !s.trim().is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| connection.data_directory());
        if !directory.is_absolute() || directory.to_string_lossy().starts_with(r"\\") {
            return Err(error(
                "validation",
                "Choose a folder on this PC's local disk.",
            ));
        }
        return persist_settings(
            &connection.path,
            &Settings {
                hosting: true,
                data_directory: Some(directory.display().to_string()),
                host: "127.0.0.1".into(),
                port,
                certificate: Vec::new(),
            },
        );
    }
    let host = host.trim().to_owned();
    let certificate = if host.is_empty() {
        Vec::new()
    } else {
        let address: IpAddr = host
            .parse()
            .map_err(|_| error("validation", "Enter a valid LAN IP address."))?;
        if address.is_unspecified() || address.is_multicast() || port == 0 {
            return Err(error(
                "validation",
                "Enter the server's LAN IP address and port.",
            ));
        }
        match certificate_path.filter(|p| !p.is_empty()) {
            Some(path) => {
                if fs::metadata(&path)
                    .map_err(|_| error("certificate_error", "Cannot open the public certificate."))?
                    .len()
                    > 65536
                {
                    return Err(error("certificate_error", "Invalid certificate file."));
                }
                fs::read(path).map_err(|_| {
                    error("certificate_error", "Cannot read the public certificate.")
                })?
            }
            None if !connection.settings.certificate.is_empty() => {
                connection.settings.certificate.clone()
            }
            _ => {
                return Err(error(
                    "certificate_error",
                    "Select server-certificate.der copied from the server PC.",
                ))
            }
        }
    };
    let settings = Settings {
        hosting: false,
        data_directory: None,
        host,
        port,
        certificate,
    };
    if !settings.host.is_empty() {
        let candidate = settings.clone();
        tauri::async_runtime::spawn_blocking(move || connect(&candidate).map(|_| ()))
            .await
            .map_err(|_| error("connection_error", "Connection test failed."))??;
    }
    persist_settings(&connection.path, &settings)
}

fn persist_settings(path: &Path, settings: &Settings) -> Result<(), Value> {
    let data = serde_json::to_vec_pretty(settings)
        .map_err(|_| error("configuration_error", "Could not save the connection."))?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|_| {
            error(
                "configuration_error",
                "Could not create the settings folder.",
            )
        })?;
    }
    let temporary = path.with_extension("json.pending");
    fs::write(&temporary, data)
        .map_err(|_| error("configuration_error", "Could not save the connection."))?;
    fs::rename(&temporary, path).map_err(|_| {
        error(
            "configuration_error",
            "Could not replace the connection settings.",
        )
    })?;
    // Apply only after restart so an open clinical form can never change databases.
    Ok(())
}

#[tauri::command]
pub(crate) async fn lan_invoke(app: tauri::AppHandle, command: String, args: Value) -> RpcResult {
    use tauri::Manager;
    tauri::async_runtime::spawn_blocking(move || {
        let connection = app.state::<ConnectionState>();
        if connection.is_host() && !app.state::<super::host::HostManager>().running() {
            return Err(error(
                "server_stopped",
                "Start the server on this PC before using the shared workspace.",
            ));
        }
        connection.invoke(&command, args)
    })
    .await
    .map_err(|_| error("connection_error", "The server request could not complete."))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    #[test]
    fn embedded_host_starts_stops_releases_the_database_and_preserves_identity() {
        let dir = tempfile::tempdir().unwrap();
        let mut server =
            super::super::server::RunningServer::start(dir.path(), "127.0.0.1:0".parse().unwrap())
                .unwrap();
        assert!(server.running());
        let certificate = fs::read(dir.path().join("server-certificate.der")).unwrap();
        let settings = Settings {
            host: "127.0.0.1".into(),
            port: server.address.port(),
            certificate: certificate.clone(),
            ..Settings::default()
        };
        let mut stream = connect(&settings).unwrap();
        // A frame arriving across an idle poll must not lose its partial prefix.
        let body = serde_json::to_vec(&Request {
            version: PROTOCOL_VERSION,
            command: "health_status".into(),
            args: json!({}),
        })
        .unwrap();
        let mut frame = (body.len() as u32).to_be_bytes().to_vec();
        frame.extend(body);
        stream.write_all(&frame[..2]).unwrap();
        stream.flush().unwrap();
        std::thread::sleep(Duration::from_millis(400));
        stream.write_all(&frame[2..]).unwrap();
        stream.flush().unwrap();
        assert!(read_frame::<Response>(&mut stream).unwrap().result.is_ok());
        write_frame(&mut stream, &Request { version: PROTOCOL_VERSION, command: "bootstrap_user".into(), args: json!({"input":{"username":"hosttester","displayName":"Synthetic Host","password":"Synthetic-host-only-42!"}}) }).unwrap();
        assert!(read_frame::<Response>(&mut stream).unwrap().result.is_ok());
        assert_eq!(server.sessions(), 1);
        let mut probe = connect(&settings).unwrap();
        write_frame(
            &mut probe,
            &Request {
                version: PROTOCOL_VERSION,
                command: "health_status".into(),
                args: json!({}),
            },
        )
        .unwrap();
        assert!(read_frame::<Response>(&mut probe).unwrap().result.is_ok());
        assert_eq!(
            server.sessions(),
            1,
            "health checks must not inflate the user count"
        );
        assert!(super::super::server::RunningServer::start(
            dir.path(),
            "127.0.0.1:0".parse().unwrap()
        )
        .is_err());
        let (stopped_tx, stopped_rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            server.stop();
            let _ = stopped_tx.send(server);
        });
        let server = stopped_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("idle connections must not hang shutdown");
        assert!(!server.running());
        assert_eq!(server.sessions(), 0);
        assert!(write_frame(
            &mut stream,
            &Request {
                version: PROTOCOL_VERSION,
                command: "health_status".into(),
                args: json!({})
            }
        )
        .and_then(|_| read_frame::<Response>(&mut stream))
        .is_err());
        let restarted =
            super::super::server::RunningServer::start(dir.path(), "127.0.0.1:0".parse().unwrap())
                .unwrap();
        assert!(restarted.running());
        assert_eq!(
            fs::read(dir.path().join("server-certificate.der")).unwrap(),
            certificate
        );
        let db = crate::db::Database::initialize(dir.path().join("oncoflow.db")).unwrap();
        let auth = crate::auth::AuthSession::default();
        assert!(
            !crate::auth::AuthService::new(&db, &auth)
                .state()
                .unwrap()
                .needs_bootstrap
        );
        drop(restarted);
        assert!(super::super::locking::lock_database(dir.path()).is_ok());
    }

    #[test]
    fn embedded_host_port_conflict_does_not_create_a_database() {
        let occupied = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let dir = tempfile::tempdir().unwrap();
        assert!(super::super::server::RunningServer::start(
            dir.path(),
            occupied.local_addr().unwrap()
        )
        .is_err());
        assert!(!dir.path().join("oncoflow.db").exists());
        assert!(super::super::locking::lock_database(dir.path()).is_ok());
    }

    #[test]
    fn host_settings_reopen_as_loopback_client_with_same_data_directory() {
        let dir = tempfile::tempdir().unwrap();
        let database_directory = dir.path().join("shared-data");
        let settings = Settings {
            hosting: true,
            data_directory: Some(database_directory.display().to_string()),
            host: "203.0.113.10".into(),
            port: 7443,
            ..Settings::default()
        };
        persist_settings(&dir.path().join("connection.json"), &settings).unwrap();
        persist_settings(&dir.path().join("connection.json"), &settings).unwrap();
        let loaded = ConnectionState::load(dir.path()).unwrap();
        assert!(loaded.is_host());
        assert!(loaded.is_client());
        assert_eq!(loaded.data_directory(), database_directory);
        assert!(
            loaded.effective_settings().is_err(),
            "missing host certificate must not fall back locally"
        );
        super::super::server::initialize_server(&database_directory).unwrap();
        assert_eq!(loaded.effective_settings().unwrap().host, "127.0.0.1");
        assert!(!dir.path().join("oncoflow.db").exists());
    }
    #[test]
    fn real_tls_clients_validate_certificate_and_share_the_server_database() {
        let dir = tempfile::tempdir().unwrap();
        super::super::server::initialize_server(dir.path()).unwrap();
        let (address, worker) = super::super::server::test_listener(dir.path(), 3);
        let settings = Settings {
            hosting: false,
            data_directory: None,
            host: address.ip().to_string(),
            port: address.port(),
            certificate: fs::read(dir.path().join("server-certificate.der")).unwrap(),
        };
        let mut first = connect(&settings).unwrap();
        let mut second = connect(&settings).unwrap();
        fn rpc(
            stream: &mut StreamOwned<ClientConnection, TcpStream>,
            command: &str,
            args: Value,
        ) -> RpcResult {
            write_frame(
                stream,
                &Request {
                    version: PROTOCOL_VERSION,
                    command: command.into(),
                    args,
                },
            )
            .unwrap();
            read_frame::<Response>(stream).unwrap().result
        }
        rpc(&mut first,"bootstrap_user",json!({"input":{"username":"tlstester","displayName":"TLS test","password":"Synthetic-tls-only-42!"}})).unwrap();
        rpc(
            &mut first,
            "create_patient",
            json!({"input":{"hn":"SYNTHETIC-TLS-001"}}),
        )
        .unwrap();
        assert_eq!(
            rpc(&mut second, "list_patients", json!({"request":{}})).unwrap_err()["code"],
            "authentication_required"
        );
        rpc(
            &mut second,
            "login",
            json!({"input":{"username":"tlstester","password":"Synthetic-tls-only-42!"}}),
        )
        .unwrap();
        assert_eq!(
            rpc(&mut second, "list_patients", json!({"request":{}})).unwrap()["total"],
            1
        );
        let other = tempfile::tempdir().unwrap();
        super::super::server::initialize_server(other.path()).unwrap();
        let wrong = Settings {
            certificate: fs::read(other.path().join("server-certificate.der")).unwrap(),
            ..settings
        };
        assert!(connect(&wrong).is_err());
        drop(first);
        drop(second);
        worker.join().unwrap();
    }
    #[test]
    fn client_configuration_does_not_create_a_database_and_never_falls_back() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("connection.json"),
            br#"{"host":"127.0.0.1","port":1,"certificate":[]}"#,
        )
        .unwrap();
        let state = ConnectionState::load(dir.path()).unwrap();
        assert!(state.is_client());
        assert!(state.call("get_startup_status", json!({})).is_err());
        assert!(!dir.path().join("oncoflow.db").exists());
        assert_eq!(
            state.call("create_patient", json!({})).unwrap_err()["code"],
            "authentication_required"
        );
    }
}
