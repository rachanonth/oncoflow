# OncoFlow desktop clients on a Windows LAN

Requested deployment: one Windows server PC and three installed desktop clients.
No internet connection or external hospital database is used during normal work.

```text
Desktop 1 ─┐
Desktop 2 ─┼─ encrypted TCP / port 7443 ─ Rust server ─ local oncoflow.db
Desktop 3 ─┘
```

## Recommended: host directly inside the desktop app

The desktop installer now includes the server. A separate server console is optional.
The same installed app can be a standalone workstation, the host PC, or a LAN client.

On the PC that will hold the shared database:

1. Open **Connection settings** and select **Host server on this PC**.
2. Confirm the server data folder. The default is this installation's existing
   app-data folder. To reuse a previous server, select the folder containing its
   `oncoflow.db` and certificate/key pair. Stop the old server first.
3. Keep port `7443` unless your network requires another port. Save, close the app
   completely, and reopen it.
4. The server starts automatically. Click **Server online** / **Server offline**
   in the top bar to see database health, the host's LAN IPv4 addresses, uptime,
   and the number of signed-in connections (including this PC).
5. Select **Export certificate for clients** and copy that public certificate to
   the other PCs. Only the public certificate is exported. Existing certificates
   are reused across restart; the private key never leaves the server folder.
6. On each other PC, choose **LAN client**, enter one of the host's LAN addresses
   and the same port, select the public certificate, save, and restart the app.

You can sign in and work on the host PC too. Its workspace connects to the embedded
server through loopback, so it uses the same authentication, serialization, and
edit-conflict checks as every other client. Hosting never adds a second direct
database writer. Initial administrator setup on the host works through loopback.

**Keep the host app open while others work.** The status panel has Start/Stop
controls; stopping asks for confirmation, disconnects clients, finishes in-flight
requests, and releases the database lock. Closing a running host asks whether to
keep running or stop and exit. A stopped host starts again on the next app launch.
This is an in-app server, not a Windows background service. It does not run when
the host PC is asleep, powered off, or the app has exited.

Status is checked about every ten seconds, with a network timeout for an unreachable
server. Health probes use separate encrypted connections and do not reset users'
login sessions or inflate the signed-in count. A green status on the host confirms
its own server and database, not the remote PCs' firewall access.

Allow **oncoflow.exe** through Windows Firewall on the private LAN. An older rule
for **oncoflow-server.exe** does not apply to the embedded host. If Windows asks
when the host first starts, select private networks. An administrator may instead
create a program-specific rule as below, using the actual installed app path:

```powershell
New-NetFirewallRule -DisplayName "OncoFlow desktop host" -Direction Inbound -Action Allow -Protocol TCP -LocalPort 7443 -Profile Private -RemoteAddress LocalSubnet -Program "C:\Path\To\OncoFlow\oncoflow.exe"
```

No database is automatically moved or merged when changing modes. No firewall
rule is created silently. Do not run the embedded host and console server against
the same database at the same time. Backups and recovery still require server
maintenance as described below.

## Shared database and compatibility

The server owns the SQLite file on its local disk. Do not place it on an SMB share,
mapped network drive, OneDrive, or a synchronization folder. SQLite remains suitable
for this small deployment because only the server accesses it and server requests
are serialized. See [SQLite's network guidance](https://www.sqlite.org/useovernet.html).

The existing Rust command/service implementations and migrations are reused. The
clinical formulas, legacy parity fixtures, and compatibility schema are unchanged.
The inspected integration points were `src-tauri/src/lib.rs`, the domain command
modules, `auth/service.rs`, `db/mod.rs`, `output/service.rs`, and
`hardware/commands.rs`. No MDB was opened or modified for this transport change.
The sole migration source remains `legacy/AllTable.mdb`; HOMC and `dbo_*` connections
remain prohibited.

LAN commands have an explicit access classification in `src-tauri/lan-commands.txt`.
The build generates argument adapters from the original Rust signatures, so the
desktop and server cannot acquire separately maintained business-rule copies.
Each TLS connection owns an independent authenticated session. Every clinical read
and write requires server-side authentication; existing administrative permissions
are also enforced. Account activation and role changes are checked on each request.
Sessions expire after eight hours or connection loss, with a 30-minute idle socket
timeout. No user password or session token is persisted on the client.

The initial conflict policy is deliberately conservative: after another user saves
any shared change, this session cannot save another edit or initiate label printing
until the user reloads and signs in. Reads do not reset that protection. Reloading
discards unsaved forms, and the operator must review the current record before
editing again. This is a whole-database check, so unrelated edits can cause a
conflict too. Requests are never automatically replayed after network failure.

Printing uses the server's verified preparation output, then rasterizes on the
client PC with its locally selected label font and submits to that PC's Windows
printer queue. Font discovery and settings are local, including in host mode.
Rendering failures cancel the pending print before spooler submission.
The server records the print
audit after Windows accepts the job. A disconnect between printing and audit cannot
prove whether paper printed; the UI tells the operator to inspect the queue before
retrying. No automatic reprint occurs.

## Optional: build the desktop and console server

From the repository root, with the normal Windows Tauri build prerequisites:

```powershell
npm ci
npm run build
cargo build --manifest-path src-tauri/Cargo.toml --release --features server-cli --bin oncoflow-server
cargo tauri build
```

Server executable: `src-tauri/target/release/oncoflow-server.exe`.
Desktop installer: `src-tauri/target/release/bundle/nsis/`.
Use matching builds on the server and all clients. The optional `oncoflow-server.exe` is a console program,
not an automatically installed Windows Service. It must remain running. An
administrator can supervise it with Windows Task Scheduler at startup under the
account owning the data directory; configure the task to run without a visible
window and restart on failure. Do not run multiple server instances against one
database directory.

## Prepare the server

1. Give the server a stable LAN IP using a DHCP reservation. The example below uses
   `192.168.1.10`; replace it with your actual IP.
2. Keep the server powered on and disable sleep during working hours.
3. Choose a data folder on a local disk. Restrict its Windows permissions to the
   server account and authorized administrators. It holds patient data and a TLS
   private key. Do not share this folder with client users.
4. Close existing OncoFlow installations before moving data. Keep a validated
   backup of the authoritative database. Copy that one database into the server
   folder as `oncoflow.db`. This release does not merge multiple PCs' databases.
   If no database is present, the server creates a new empty database.
5. Generate a unique certificate once, then start the server:

```powershell
.\oncoflow-server.exe init --data-dir "C:\OncoFlowData"
.\oncoflow-server.exe run --data-dir "C:\OncoFlowData" --listen "0.0.0.0:7443"
```

`init` refuses to overwrite existing keys. Copy only `server-certificate.der` to
the client computers using a trusted transfer. Never copy `server-private-key.der`
to clients. The clients store the public certificate and verify the server using
it; certificate verification is never disabled. Keep the clocks correct on all PCs.
When replacing or renewing the certificate, distribute the new public certificate
and select it again in every client's connection settings.

On the server, an administrator can create a firewall rule limited to the private
LAN. Replace the program path with the actual server executable location:

```powershell
New-NetFirewallRule -DisplayName "OncoFlow LAN" -Direction Inbound -Action Allow -Protocol TCP -LocalPort 7443 -Profile Private -RemoteAddress LocalSubnet -Program "C:\OncoFlow\oncoflow-server.exe"
```

Use a private network profile. No router port forwarding is needed. The server's
default listen address, if omitted, is loopback only for initial setup.

## Connect the desktops

1. Install the new OncoFlow desktop build.
2. Select **Connection settings**, available above the login screen and even when
   the server is unavailable.
3. Select **LAN client — central server**.
4. Enter the server IP and port `7443`.
5. Select the copied `server-certificate.der` file.
6. Save. This checks the encrypted connection. Close OncoFlow completely and reopen.
7. Sign in with an account stored in the server database.

For a new database, create the first administrator using an installed desktop on
the server PC configured as a LAN client with IP `127.0.0.1`. Initial account creation
is rejected from other machines. Then create the three user accounts under Users.
Existing migrated modern accounts keep their credentials. Legacy plaintext
passwords are not accepted.

The server PC can also run a desktop client while hosting the server. A client
never opens or creates a fallback `oncoflow.db`. Existing standalone data is left
in place when switching modes; it is neither uploaded nor merged automatically.
`connection.json` in the desktop app-data directory stores only the server IP, port,
and public certificate. A damaged configuration fails closed instead of selecting
a different database silently.

## Backups and recovery

Backups are operator managed in this release; there is no automatic daily backup
job. Make a daily backup and keep protected copies off the server. Database copies
contain patient data. Client Backup & restore displays a server-maintenance notice
and cannot restore the central database over the LAN.

To create a verified snapshot, disconnect clients, stop the server with Ctrl+C (or
stop its scheduled task), then run:

```powershell
.\oncoflow-server.exe backup --data-dir "C:\OncoFlowData" --destination "D:\OncoFlowBackups\oncoflow-2026-09-18.db"
```

The destination folder must already exist. The command refuses to overwrite an
existing file and validates SQLite integrity and foreign keys. Choose a new filename
each time. It refuses to run while another new OncoFlow process owns the directory.
Restart the server afterward. This CLI snapshot has no desktop backup manifest or
user audit entry; retain the operator's backup record separately.

For recovery, stop all clients and the server, preserve the entire current server
data folder, and restore a validated OncoFlow backup as `oncoflow.db` on the server.
Do not mix journal/WAL sidecars from another database with the restored file. Keep
the certificate/key pair unchanged. Start the server, check diagnostics and known
records, then let users reconnect. Do not replace a database while the server is
running. Existing desktop recovery tooling remains available in standalone mode;
do not point an active standalone app or migration CLI at the running server's file.

## Acceptance before clinical use

Automated tests exercise isolated sessions, shared synthetic patient records,
stale-write rejection, authentication, expired/deactivated sessions, TLS certificate
validation, bounded messages, and the no-local-fallback rule. Existing domain parity
tests remain applicable. No real patient database is used in these tests.

On the actual three PCs, verify sign-in, shared patient/order changes, a deliberate
edit conflict, network interruption recovery, and each physical label printer.
Perform a backup and restore drill on a disposable copy. Loopback tests cannot
verify your actual firewall, LAN connectivity, Windows task setup, or paper output.
