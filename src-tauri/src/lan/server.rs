use super::{argument, encode, encode_result, error, protocol::*, RpcResult};
use crate::{
    auth::{AuthService, AuthSession},
    db::Database,
    output::{OutputService, PreparationOutput},
};
use rustls::{
    pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer},
    ServerConfig, ServerConnection, StreamOwned,
};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    fs::{self, OpenOptions},
    io::{self, Read, Write},
    net::{Shutdown, SocketAddr, TcpListener, TcpStream},
    path::Path,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
};
use std::{
    sync::Mutex,
    time::{Duration, Instant},
};

include!(concat!(env!("OUT_DIR"), "/lan_dispatch.rs"));

use super::locking::lock_database;
const SESSION_LIFETIME: Duration = Duration::from_secs(8 * 60 * 60);
const MAX_CONNECTIONS: usize = 16;

#[derive(Default)]
struct Control {
    stopping: AtomicBool,
    sockets: Mutex<HashMap<u64, TcpStream>>,
    sessions: Arc<AtomicUsize>,
}

pub(super) struct RunningServer {
    pub address: SocketAddr,
    pub started: Instant,
    control: Arc<Control>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl RunningServer {
    pub(super) fn start(
        directory: &Path,
        listen: SocketAddr,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        if !directory.is_absolute() || directory.to_string_lossy().starts_with(r"\\") {
            return Err("Choose an absolute data directory on a local disk".into());
        }
        let database_lock = lock_database(directory)?;
        let cert = directory.join("server-certificate.der");
        let key = directory.join("server-private-key.der");
        if !cert.exists() && !key.exists() {
            initialize_server(directory)?;
        }
        let tls = tls_config(directory)?;
        // Bind before database migration so an occupied port cannot modify a database.
        let listener = TcpListener::bind(listen)?;
        let address = listener.local_addr()?;
        listener.set_nonblocking(true)?;
        let database = Database::initialize(directory.join(crate::db::DATABASE_FILENAME))?;
        let server = Arc::new(Server {
            database,
            revision: Mutex::new(0),
            login_attempts: Mutex::new(Vec::new()),
        });
        let control = Arc::new(Control::default());
        let worker_control = control.clone();
        let thread = std::thread::spawn(move || {
            // Keep the OS database lock until every in-flight request has finished.
            let _database_lock = database_lock;
            let mut workers: Vec<std::thread::JoinHandle<()>> = Vec::new();
            let mut next_id = 0_u64;
            while !worker_control.stopping.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((socket, _)) => {
                        let mut sockets = worker_control.sockets.lock().unwrap();
                        if sockets.len() >= MAX_CONNECTIONS {
                            continue;
                        }
                        let Ok(tracked) = socket.try_clone() else {
                            continue;
                        };
                        next_id += 1;
                        let id = next_id;
                        sockets.insert(id, tracked);
                        drop(sockets);
                        let (tls, server, control) =
                            (tls.clone(), server.clone(), worker_control.clone());
                        workers.push(std::thread::spawn(move || {
                            struct Release {
                                control: Arc<Control>,
                                id: u64,
                            }
                            impl Drop for Release {
                                fn drop(&mut self) {
                                    if let Ok(mut sockets) = self.control.sockets.lock() {
                                        sockets.remove(&self.id);
                                    }
                                }
                            }
                            let _release = Release {
                                control: control.clone(),
                                id,
                            };
                            let _ = serve_connection(socket, tls, server, Some(control.clone()));
                        }));
                    }
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(25))
                    }
                    Err(_) => break,
                }
                let mut i = 0;
                while i < workers.len() {
                    if workers[i].is_finished() {
                        let _ = workers.swap_remove(i).join();
                    } else {
                        i += 1;
                    }
                }
            }
            drop(listener);
            if let Ok(sockets) = worker_control.sockets.lock() {
                for socket in sockets.values() {
                    let _ = socket.shutdown(Shutdown::Both);
                }
            }
            for worker in workers {
                let _ = worker.join();
            }
        });
        Ok(Self {
            address,
            started: Instant::now(),
            control,
            thread: Some(thread),
        })
    }

    pub(super) fn running(&self) -> bool {
        self.thread.as_ref().is_some_and(|t| !t.is_finished())
            && !self.control.stopping.load(Ordering::SeqCst)
    }
    pub(super) fn sessions(&self) -> usize {
        self.control.sessions.load(Ordering::SeqCst)
    }
    pub(super) fn stop(&mut self) {
        self.control.stopping.store(true, Ordering::SeqCst);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl Drop for RunningServer {
    fn drop(&mut self) {
        self.stop();
    }
}

struct Server {
    database: Database,
    // Serialize requests and revision checks together. Three-user deployment, one writer.
    revision: Mutex<u64>,
    login_attempts: Mutex<Vec<Instant>>,
}

#[derive(Default)]
struct Session {
    auth: AuthSession,
    baseline: Option<u64>,
    authenticated_at: Option<Instant>,
    pending_print: Option<Vec<PreparationOutput>>,
    counter: Option<Arc<AtomicUsize>>,
    counted: bool,
}

impl Session {
    fn expire_at(&mut self, now: Instant) {
        if self
            .authenticated_at
            .is_some_and(|at| now.saturating_duration_since(at) >= SESSION_LIFETIME)
        {
            let _ = self.auth.invalidate();
            self.pending_print = None;
            self.authenticated_at = None;
        }
    }
    fn update_count(&mut self) {
        let authenticated = self.auth.current_user().ok().flatten().is_some();
        if let Some(counter) = &self.counter {
            if authenticated && !self.counted {
                counter.fetch_add(1, Ordering::SeqCst);
            }
            if !authenticated && self.counted {
                counter.fetch_sub(1, Ordering::SeqCst);
            }
        }
        self.counted = authenticated;
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        if self.counted {
            if let Some(counter) = &self.counter {
                counter.fetch_sub(1, Ordering::SeqCst);
            }
        }
    }
}

impl Server {
    fn execute(&self, session: &mut Session, request: Request, loopback: bool) -> RpcResult {
        if request.version != PROTOCOL_VERSION {
            return Err(error(
                "version_mismatch",
                "Client and server versions do not match. Update both installations.",
            ));
        }
        let command = request.command.as_str();
        let args = &request.args;
        if !args.is_object() {
            return Err(error("invalid_argument", "Expected an argument object."));
        }
        if command == "health_status" {
            return Ok(
                json!({"backendRunning":true,"databaseConnected":true,"schemaVersion":self.database.schema_version().map_err(|_| error("database_unavailable", "The server database is unavailable."))?}),
            );
        }
        if command == "get_startup_status" {
            return Ok(
                json!({"databaseReady":true,"databaseLocation":"OncoFlow LAN server","issue":null}),
            );
        }
        let kind = access(command)
            .or_else(|| match command {
                "prepare_client_print"
                | "complete_client_print"
                | "cancel_client_print"
                | "get_diagnostics" => Some("read"),
                _ => None,
            })
            .ok_or_else(|| {
                error(
                    "server_only",
                    "This operation must be performed on the server with clients disconnected.",
                )
            })?;
        if command == "bootstrap_user" && !loopback {
            return Err(error("server_setup_required", "Create the first administrator on the server PC using a desktop client connected to 127.0.0.1."));
        }
        if command == "login" || command == "bootstrap_user" {
            let mut attempts = self
                .login_attempts
                .lock()
                .map_err(|_| error("server_error", "Sign-in is unavailable."))?;
            attempts.retain(|t| t.elapsed() < Duration::from_secs(60));
            if attempts.len() >= 10 {
                return Err(error(
                    "rate_limited",
                    "Too many sign-in attempts. Wait one minute and try again.",
                ));
            }
            attempts.push(Instant::now());
            session
                .auth
                .invalidate()
                .map_err(|_| error("server_error", "Session reset failed."))?;
            session.baseline = None;
            session.pending_print = None;
        }
        session.expire_at(Instant::now());
        let mut revision = self
            .revision
            .lock()
            .map_err(|_| error("server_error", "The server is unavailable."))?;
        let auth = AuthService::new(&self.database, &session.auth);
        if kind != "auth" {
            auth.current_user().map_err(|_| {
                error(
                    "authentication_required",
                    "Your server session ended. Reconnect and sign in again.",
                )
            })?;
        }
        // Refresh the cached role/type too, so account changes apply on the next request.
        if (kind == "write" || command == "prepare_client_print")
            && session.baseline != Some(*revision)
        {
            return Err(error("edit_conflict", "Another user changed shared data. Reload the workspace and sign in again before editing. Your change was not saved."));
        }
        let result = match command {
            "get_diagnostics" => {
                let startup = crate::recovery::StartupState::ready();
                let mut diagnostics =
                    crate::recovery::RecoveryService::new(&self.database, &session.auth, &startup)
                        .diagnostics()
                        .map_err(|_| {
                            error("server_error", "Server diagnostics could not be loaded.")
                        })?;
                diagnostics.database_location = "OncoFlow LAN server".into();
                diagnostics.automatic_backup_policy =
                    "Backups and restores are managed on the server PC with clients disconnected."
                        .into();
                encode(diagnostics)
            }
            "prepare_client_print" => {
                if session.pending_print.is_some() {
                    return Err(error("print_pending", "A previous print has not been acknowledged. Check the printer queue before reconnecting."));
                }
                let service = OutputService::new(&self.database, &session.auth);
                let render_locally = args
                    .get("renderLocally")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                let outputs = if let Some(id) = args.get("preparationId") {
                    vec![service
                        .get_preparation_output(
                            serde_json::from_value(id.clone())
                                .map_err(|_| error("invalid_argument", "Invalid preparation."))?,
                        )
                        .map_err(|e| {
                            serde_json::to_value(crate::output::commands::CommandError::from(e))
                                .unwrap()
                        })?]
                } else {
                    service
                        .get_order_outputs(
                            argument(args, "orderId")?,
                            &argument::<Vec<i64>>(args, "preparationIds")?,
                        )
                        .map_err(|e| {
                            serde_json::to_value(crate::output::commands::CommandError::from(e))
                                .unwrap()
                        })?
                };
                if render_locally {
                    let response = json!({"outputs": outputs});
                    session.pending_print = Some(outputs);
                    return Ok(response);
                }
                let config = argument(args, "config")?;
                let bytes = crate::hardware::renderer::render_preparation_labels(&outputs, &config)
                    .map_err(|_| {
                        error(
                            "print_error",
                            "The server could not render the selected labels.",
                        )
                    })?;
                session.pending_print = Some(outputs);
                Ok(json!({"bytes":bytes}))
            }
            "cancel_client_print" => {
                session.pending_print = None;
                Ok(Value::Null)
            }
            "complete_client_print" => {
                let outputs = session.pending_print.take().ok_or_else(|| {
                    error("print_missing", "No pending print exists in this session.")
                })?;
                let service = OutputService::new(&self.database, &session.auth);
                let mut recorded = Vec::new();
                for output in outputs {
                    recorded.push(service.record_rendered_label_print_request(output.label.preparation_id,
                        "windows_lan_client_spooler", crate::hardware::LABEL_RENDERER_VERSION, &output.label.print_time)
                        .map_err(|_| error("print_audit_error", "The printer accepted the job, but its server audit could not be completed. Check the queue before retrying."))?);
                }
                encode(recorded)
            }
            _ => dispatch(&self.database, &session.auth, command, args),
        };
        // Advance even after a failed write: some legacy workflows may commit before
        // producing their response. This deliberately errs toward detecting a conflict.
        if kind == "write" {
            *revision += 1;
            session.baseline = Some(*revision);
        }
        if result.is_ok() && matches!(command, "login" | "bootstrap_user") {
            session.baseline = Some(*revision);
            session.authenticated_at = Some(Instant::now());
        }
        if command == "logout" {
            session.baseline = None;
            session.pending_print = None;
            session.authenticated_at = None;
        }
        result
    }
}

/// Make a consistent SQLite snapshot in a new file while the server is stopped.
#[cfg(any(test, feature = "server-cli"))]
pub fn backup_server(
    directory: &Path,
    destination: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let _lock = lock_database(directory)?;
    let source = rusqlite::Connection::open_with_flags(
        directory.join(crate::db::DATABASE_FILENAME),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )?;
    crate::db::configure_connection(&source)?;
    if crate::db::read_schema_version(&source)?.is_none() {
        return Err("Not an OncoFlow database".into());
    }
    crate::db::validate_connection(&source)?;
    // Never overwrite a previous backup or the active database.
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)?;
    let mut target = rusqlite::Connection::open(destination)?;
    rusqlite::backup::Backup::new(&source, &mut target)?.run_to_completion(
        100,
        Duration::from_millis(5),
        None,
    )?;
    crate::db::configure_connection(&target)?;
    crate::db::validate_connection(&target)?;
    Ok(())
}

fn tls_config(directory: &Path) -> Result<Arc<ServerConfig>, Box<dyn std::error::Error>> {
    let cert = CertificateDer::from(fs::read(directory.join("server-certificate.der"))?);
    let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(fs::read(
        directory.join("server-private-key.der"),
    )?));
    let config =
        ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_safe_default_protocol_versions()?
            .with_no_client_auth()
            .with_single_cert(vec![cert], key)?;
    Ok(Arc::new(config))
}

// Retry socket timeouts inside Read so partial TLS records and framed messages
// remain intact, while shutdown can interrupt otherwise idle connections.
struct ServerSocket {
    socket: TcpStream,
    control: Option<Arc<Control>>,
    idle_limit: Duration,
    last_read: Instant,
}
impl Read for ServerSocket {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        loop {
            if self
                .control
                .as_ref()
                .is_some_and(|c| c.stopping.load(Ordering::SeqCst))
            {
                return Err(io::Error::new(
                    io::ErrorKind::ConnectionAborted,
                    "Server stopped",
                ));
            }
            match self.socket.read(buffer) {
                Ok(n) => {
                    if n > 0 {
                        self.last_read = Instant::now();
                    }
                    return Ok(n);
                }
                Err(e)
                    if matches!(
                        e.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                    ) && self.last_read.elapsed() < self.idle_limit =>
                {
                    continue
                }
                Err(e) => return Err(e),
            }
        }
    }
}
impl Write for ServerSocket {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.socket.write(buffer)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.socket.flush()
    }
}

fn serve_connection(
    socket: TcpStream,
    tls: Arc<ServerConfig>,
    server: Arc<Server>,
    control: Option<Arc<Control>>,
) -> io::Result<()> {
    // Windows can inherit the listener's nonblocking mode on accepted sockets.
    // Each TLS worker uses blocking reads with explicit timeouts.
    socket.set_nonblocking(false)?;
    let loopback = socket.peer_addr()?.ip().is_loopback();
    socket.set_read_timeout(Some(Duration::from_millis(250)))?;
    socket.set_write_timeout(Some(Duration::from_secs(5)))?;
    socket.set_nodelay(true)?;
    let connection = ServerConnection::new(tls).map_err(io::Error::other)?;
    let counter = control.as_ref().map(|c| c.sessions.clone());
    let socket = ServerSocket {
        socket,
        control,
        idle_limit: Duration::from_secs(15),
        last_read: Instant::now(),
    };
    let mut stream = StreamOwned::new(connection, socket);
    // Complete handshake with a short deadline before allowing an idle work session.
    while stream.conn.is_handshaking() {
        stream.conn.complete_io(&mut stream.sock)?;
    }
    stream.sock.idle_limit = Duration::from_secs(30 * 60);
    let mut session = Session::default();
    session.counter = counter;
    loop {
        let request: Request = read_frame(&mut stream)?;
        let response = Response {
            result: server.execute(&mut session, request, loopback),
        };
        session.update_count();
        write_frame(&mut stream, &response)?;
    }
}

/// Initialize certificates in an operator-owned directory. Never overwrites keys or data.
pub fn initialize_server(directory: &Path) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(directory)?;
    let cert_path = directory.join("server-certificate.der");
    let key_path = directory.join("server-private-key.der");
    if cert_path.exists() || key_path.exists() {
        return Err("Server certificates already exist; refusing to overwrite".into());
    }
    let rcgen::CertifiedKey { cert, signing_key } =
        rcgen::generate_simple_self_signed(vec![SERVER_NAME.into()])?;
    use std::io::Write;
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(key_path)?
        .write_all(&signing_key.serialize_der())?;
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(cert_path)?
        .write_all(cert.der())?;
    Ok(())
}

/// Run the server in the foreground; Windows Task Scheduler may supervise this process.
#[cfg(feature = "server-cli")]
pub fn run_server(directory: &Path, listen: SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
    let server = RunningServer::start(directory, listen)?;
    println!(
        "OncoFlow server listening on {} (TLS). Keep this process running.",
        server.address
    );
    while server.running() {
        std::thread::sleep(Duration::from_millis(250));
    }
    Err("The server listener stopped".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    const PASSWORD: &str = "Synthetic-test-only-42!";
    #[test]
    fn cancellation_is_authenticated_and_stale_clients_cannot_check_or_cancel_again() {
        let (_dir, server) = fixture();
        let mut a = Session::default();
        let mut b = Session::default();
        assert_eq!(
            call(
                &server,
                &mut a,
                "get_order_cancellation_preview",
                json!({"orderId":1,"itemId":null})
            )
            .unwrap_err()["code"],
            "authentication_required"
        );
        bootstrap(&server, &mut a);
        server.database.open().unwrap().execute_batch(
            "INSERT INTO patients(id,legacy_hn) VALUES(1,'SYN-CANCEL');
             INSERT INTO drugs(id,legacy_dcode,drug_name,marker) VALUES(1,'SYN-D','Synthetic',1);
             INSERT INTO orders(id,legacy_orderid,patient_id,oncoflow_created,order_time) VALUES(1,'SYN-O',1,1,'2026-09-29T09:00');
             INSERT INTO order_items(id,order_id,drug_id) VALUES(1,1,1);
             INSERT INTO preparation_tasks(id,source_order_id,source_order_item_id,drug_id,preparation_date) VALUES(1,1,1,1,'2026-09-29');"
        ).unwrap();
        let username = a.auth.require_user().unwrap().username;
        login(&server, &mut b, &username);
        let preview = call(
            &server,
            &mut a,
            "get_order_cancellation_preview",
            json!({"orderId":1,"itemId":null}),
        )
        .unwrap();
        let input = json!({"orderId":1,"input":{"itemId":null,"revision":preview["revision"],"reason":"Synthetic excess order","acknowledged":true,"tasks":[{"taskId":1,"actuallyPrepared":false}]}});
        let result = call(&server, &mut a, "cancel_order", input.clone()).unwrap();
        assert_eq!(result["workflowStatus"], "cancelled");
        assert_eq!(
            call(
                &server,
                &mut b,
                "check_preparation_task",
                json!({"taskId":1})
            )
            .unwrap_err()["code"],
            "edit_conflict"
        );
        assert_eq!(
            call(&server, &mut b, "cancel_order", input).unwrap_err()["code"],
            "edit_conflict"
        );
        login(&server, &mut b, &username);
        assert!(call(
            &server,
            &mut b,
            "check_preparation_task",
            json!({"taskId":1})
        )
        .is_err());
    }
    #[test]
    fn backup_requires_existing_database_preserves_data_and_refuses_overwrite() {
        let empty = tempfile::tempdir().unwrap();
        assert!(backup_server(empty.path(), &empty.path().join("backup.db")).is_err());
        assert!(!empty.path().join("oncoflow.db").exists());
        let (dir, server) = fixture();
        let mut session = Session::default();
        bootstrap(&server, &mut session);
        call(
            &server,
            &mut session,
            "create_patient",
            json!({"input":{"hn":"SYNTHETIC-BACKUP"}}),
        )
        .unwrap();
        let target = dir.path().join("backup.db");
        backup_server(dir.path(), &target).unwrap();
        let backup = rusqlite::Connection::open(&target).unwrap();
        let total: i64 = backup
            .query_row("SELECT COUNT(*) FROM patients", [], |row| row.get(0))
            .unwrap();
        assert_eq!(total, 1);
        assert!(backup_server(dir.path(), &target).is_err());
    }
    fn fixture() -> (tempfile::TempDir, Server) {
        let dir = tempfile::tempdir().unwrap();
        let database = Database::initialize(dir.path().join("oncoflow.db")).unwrap();
        (
            dir,
            Server {
                database,
                revision: Mutex::new(0),
                login_attempts: Mutex::new(Vec::new()),
            },
        )
    }
    fn request(command: &str, args: Value) -> Request {
        Request {
            version: PROTOCOL_VERSION,
            command: command.into(),
            args,
        }
    }
    fn call(server: &Server, session: &mut Session, command: &str, args: Value) -> RpcResult {
        server.execute(session, request(command, args), true)
    }
    fn bootstrap(server: &Server, session: &mut Session) {
        call(server, session, "bootstrap_user", json!({"input":{"username":"testadmin","displayName":"Test Admin","password":PASSWORD}})).unwrap();
    }
    fn login(server: &Server, session: &mut Session, username: &str) {
        call(
            server,
            session,
            "login",
            json!({"input":{"username":username,"password":PASSWORD}}),
        )
        .unwrap();
    }
    #[test]
    fn client_print_uses_frozen_data_and_session_local_acknowledgement() {
        let (_dir, server) = fixture();
        let mut session = Session::default();
        bootstrap(&server, &mut session);
        server.database.open().unwrap().execute_batch(
            "INSERT INTO patients(id,legacy_hn,first_name) VALUES(1,'SYN-FONT','Synthetic');
             INSERT INTO drugs(id,legacy_dcode,drug_name,marker) VALUES(1,'SYN-FONT-D','Synthetic drug',1);
             INSERT INTO orders(id,legacy_orderid,patient_id,oncoflow_created) VALUES(1,'SYN-FONT-O',1,1);
             INSERT INTO order_items(id,order_id,drug_id,dose) VALUES(1,1,1,10);
             INSERT INTO preparation_tasks(id,source_order_id,source_order_item_id,drug_id,state,preparation_date,prepared_at,verified_at,prepared_by_user_id,verified_by_user_id)
             VALUES(1,1,1,1,'verified','2026-09-29','2026-09-29T09:00:00','2026-09-29T09:01:00',1,1);"
        ).unwrap();
        // No server font or printer configuration is needed for local rasterization.
        let args = json!({"preparationId":1,"renderLocally":true});
        let prepared = call(&server, &mut session, "prepare_client_print", args.clone()).unwrap();
        let outputs: Vec<PreparationOutput> =
            serde_json::from_value(prepared["outputs"].clone()).unwrap();
        assert_eq!(outputs[0].label.patient_identifier, "SYN-FONT");
        assert!(prepared.get("bytes").is_none());
        assert_eq!(
            call(&server, &mut session, "prepare_client_print", args.clone()).unwrap_err()["code"],
            "print_pending"
        );
        let mut other = Session::default();
        login(&server, &mut other, "testadmin");
        assert_eq!(
            call(&server, &mut other, "complete_client_print", json!({})).unwrap_err()["code"],
            "print_missing"
        );
        call(&server, &mut session, "cancel_client_print", json!({})).unwrap();
        assert_eq!(
            call(&server, &mut session, "complete_client_print", json!({})).unwrap_err()["code"],
            "print_missing"
        );
        let prepared = call(&server, &mut session, "prepare_client_print", args).unwrap();
        assert_eq!(prepared["outputs"][0]["printRequestCount"], 0);
        let completed = call(&server, &mut session, "complete_client_print", json!({})).unwrap();
        assert_eq!(completed[0]["printRequestCount"], 1);
        assert_eq!(
            completed[0]["label"]["snapshotId"],
            outputs[0].label.snapshot_id
        );
    }

    #[test]
    fn three_sessions_share_data_without_sharing_identity_and_reject_stale_writes() {
        let (_dir, server) = fixture();
        let (mut a, mut b, mut c) = (Session::default(), Session::default(), Session::default());
        bootstrap(&server, &mut a);
        for name in ["tester2", "tester3"] {
            call(&server, &mut a, "create_user", json!({"input":{"username":name,"displayName":name,"password":PASSWORD,"userType":"pharmacist"}})).unwrap();
        }
        login(&server, &mut b, "tester2");
        login(&server, &mut c, "tester3");
        let created = call(
            &server,
            &mut a,
            "create_patient",
            json!({"input":{"hn":"SYNTHETIC-LAN-001","firstName":"Synthetic"}}),
        )
        .unwrap();
        for session in [&mut b, &mut c] {
            let list = call(&server, session, "list_patients", json!({"request":{}})).unwrap();
            assert_eq!(list["total"], 1);
            // Reading after somebody else's write must not silently refresh an old form's baseline.
            let conflict = call(&server, session, "update_patient", json!({"patientId":created["id"],"input":{"hn":"SYNTHETIC-LAN-001","firstName":"Stale overwrite"}})).unwrap_err();
            assert_eq!(conflict["code"], "edit_conflict");
        }
        assert_eq!(
            call(&server, &mut b, "get_current_user", json!({})).unwrap()["username"],
            "tester2"
        );
        call(&server, &mut a, "logout", json!({})).unwrap();
        assert_eq!(
            call(&server, &mut b, "get_current_user", json!({})).unwrap()["username"],
            "tester2"
        );
        assert_eq!(
            call(&server, &mut a, "list_patients", json!({"request":{}})).unwrap_err()["code"],
            "authentication_required"
        );
        login(&server, &mut b, "tester2");
        let updated = call(&server, &mut b, "update_patient", json!({"patientId":created["id"],"input":{"hn":"SYNTHETIC-LAN-001","firstName":"Reviewed change"}})).unwrap();
        assert_eq!(updated["firstName"], "Reviewed change");
    }
    #[test]
    fn unauthenticated_clients_cannot_read_clinical_data_or_take_over_bootstrap() {
        let (_dir, server) = fixture();
        let mut session = Session::default();
        assert_eq!(
            call(&server, &mut session, "get_patient", json!({"patientId":1})).unwrap_err()["code"],
            "authentication_required"
        );
        assert_eq!(
            server
                .execute(&mut session, request("bootstrap_user", json!({})), false)
                .unwrap_err()["code"],
            "server_setup_required"
        );
        for command in [
            "restore_database",
            "open_data_folder",
            "arbitrary_sql",
            "print_test_label",
            "list_system_label_fonts",
        ] {
            assert_eq!(
                call(&server, &mut session, command, json!({})).unwrap_err()["code"],
                "server_only"
            );
        }
        let mut wrong = request("health_status", json!({}));
        wrong.version = 999;
        assert_eq!(
            server.execute(&mut session, wrong, true).unwrap_err()["code"],
            "version_mismatch"
        );
    }
    #[test]
    fn admin_password_reset_revokes_lan_sessions_and_rejects_stale_writes() {
        let (_dir, server) = fixture();
        let mut admin = Session::default();
        bootstrap(&server, &mut admin);
        let user = call(&server, &mut admin, "create_user", json!({"input":{"username":"resetclient","displayName":"Synthetic client","password":PASSWORD,"userType":"pharmacist"}})).unwrap();
        let mut client = Session::default();
        login(&server, &mut client, "resetclient");
        assert_eq!(
            call(
                &server,
                &mut client,
                "reset_user_password",
                json!({"userId":user["id"],"newPassword":"replacement password 42!"})
            )
            .unwrap_err()["code"],
            "admin_required"
        );
        login(&server, &mut admin, "testadmin");
        let mut second_admin = Session::default();
        login(&server, &mut second_admin, "testadmin");
        call(
            &server,
            &mut admin,
            "reset_user_password",
            json!({"userId":user["id"],"newPassword":"replacement password 42!"}),
        )
        .unwrap();
        assert_eq!(
            call(
                &server,
                &mut second_admin,
                "reset_user_password",
                json!({"userId":user["id"],"newPassword":"stale password 99!"})
            )
            .unwrap_err()["code"],
            "edit_conflict"
        );
        assert_eq!(
            call(&server, &mut client, "list_patients", json!({"request":{}})).unwrap_err()["code"],
            "authentication_required"
        );
        assert!(call(
            &server,
            &mut client,
            "login",
            json!({"input":{"username":"resetclient","password":PASSWORD}})
        )
        .is_err());
        assert!(call(
            &server,
            &mut client,
            "login",
            json!({"input":{"username":"resetclient","password":"replacement password 42!"}})
        )
        .is_ok());
    }

    #[test]
    fn expired_and_deactivated_sessions_are_rejected() {
        let (_dir, server) = fixture();
        let mut admin = Session::default();
        bootstrap(&server, &mut admin);
        let user = call(&server, &mut admin, "create_user", json!({"input":{"username":"tester2","displayName":"Second tester","password":PASSWORD,"userType":"pharmacist"}})).unwrap();
        let mut client = Session::default();
        login(&server, &mut client, "tester2");
        call(&server, &mut admin, "update_user", json!({"userId":user["id"],"input":{"username":"tester2","displayName":"Second tester","userType":"pharmacist","role":"pharmacist","active":false}})).unwrap();
        assert_eq!(
            call(&server, &mut client, "list_patients", json!({"request":{}})).unwrap_err()["code"],
            "authentication_required"
        );
        // Advance the test clock: subtracting eight hours can underflow on a
        // Windows machine that has only recently booted.
        admin.expire_at(admin.authenticated_at.unwrap() + SESSION_LIFETIME);
        assert_eq!(
            call(&server, &mut admin, "list_patients", json!({"request":{}})).unwrap_err()["code"],
            "authentication_required"
        );
    }
    #[test]
    fn database_directory_cannot_be_opened_by_two_processes() {
        let dir = tempfile::tempdir().unwrap();
        let lock = lock_database(dir.path()).unwrap();
        assert!(lock_database(dir.path()).is_err());
        drop(lock);
        assert!(lock_database(dir.path()).is_ok());
    }
    #[test]
    fn malformed_arguments_and_login_failures_do_not_leak_database_details() {
        let (_dir, server) = fixture();
        let mut session = Session::default();
        bootstrap(&server, &mut session);
        assert_eq!(
            call(
                &server,
                &mut session,
                "get_patient",
                json!({"patientId":"invalid"})
            )
            .unwrap_err()["code"],
            "invalid_argument"
        );
        assert!(call(
            &server,
            &mut session,
            "login",
            json!({"input":{"username":"testadmin","password":"wrong"}})
        )
        .is_err());
        assert_eq!(
            call(&server, &mut session, "get_patient", json!({"patientId":1})).unwrap_err()["code"],
            "authentication_required"
        );
    }
}

#[cfg(test)]
pub(super) fn test_listener(
    directory: &Path,
    count: usize,
) -> (SocketAddr, std::thread::JoinHandle<()>) {
    let tls = tls_config(directory).unwrap();
    let server = Arc::new(Server {
        database: Database::initialize(directory.join("oncoflow.db")).unwrap(),
        revision: Mutex::new(0),
        login_attempts: Mutex::new(Vec::new()),
    });
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let handle = std::thread::spawn(move || {
        let mut workers = Vec::new();
        for socket in listener.incoming().take(count) {
            let (tls, server) = (tls.clone(), server.clone());
            workers.push(std::thread::spawn(move || {
                let _ = serve_connection(socket.unwrap(), tls, server, None);
            }));
        }
        for worker in workers {
            worker.join().unwrap();
        }
    });
    (address, handle)
}
