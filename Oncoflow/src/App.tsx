import { useCallback, useEffect, useState } from "react";

import { getAuthState, getStartupStatus, logoutUser } from "./api/commands";
import { AccountSettings } from "./auth/AccountSettings";
import { AuthFrame, FirstRunSetup, LoginScreen } from "./auth/AuthScreens";
import { SessionIdentity } from "./auth/SessionIdentity";
import { UserManagement } from "./auth/UserManagement";
import { DrugDetail } from "./drug/DrugDetail";
import { DrugForm } from "./drug/DrugForm";
import { DrugList } from "./drug/DrugList";
import { PatientDetail } from "./patient/PatientDetail";
import { PatientForm } from "./patient/PatientForm";
import { PatientList } from "./patient/PatientList";
import { OrderDetail } from "./order/OrderDetail";
import { OrderForm } from "./order/OrderForm";
import { OrderList } from "./order/OrderList";
import { RegimenDetail } from "./regimen/RegimenDetail";
import { RegimenForm } from "./regimen/RegimenForm";
import { RegimenList } from "./regimen/RegimenList";
import { PreparationQueue } from "./preparation/PreparationQueue";
import { PreparationWorkspace } from "./preparation/PreparationWorkspace";
import { PreparationCountReport } from "./report/PreparationCountReport";
import { InventoryList } from "./inventory/InventoryList";
import { InventoryDetail } from "./inventory/InventoryDetail";
import { HardwareSettings } from "./hardware/HardwareSettings";
import { BackupRestore } from "./recovery/BackupRestore";
import { DatabaseRecoveryScreen } from "./recovery/DatabaseRecoveryScreen";
import { Diagnostics } from "./recovery/Diagnostics";
import { DoctorsPage, WardsPage } from "./master_data/MasterDataPages";
import { DiluentsPage, RoutesPage } from "./master_data/MedicationLookups";
import { DiagnosisPage } from "./master_data/DiagnosisPage";
import { GuidanceSettings } from "./guidance/GuidanceSettings";
import { PageGuidanceProvider } from "./guidance/PageGuidance";
import { GeneralSettings } from "./settings/GeneralSettings";
import type { DrugDetail as DrugDetailType } from "./types/drug";
import type { PatientDetail as PatientDetailType } from "./types/patient";
import type { RegimenDetail as RegimenDetailType } from "./types/regimen";
import type { OrderDetail as OrderDetailType } from "./types/order";
import type { AuthState, CurrentUser } from "./types/auth";
import type { StartupStatus } from "./types/recovery";

type View =
  | { kind: "patients" }
  | { kind: "detail"; patientId: number }
  | { kind: "create"; returnToOrderHn?: string }
  | { kind: "edit"; patient: PatientDetailType }
  | { kind: "drugs" }
  | { kind: "drugDetail"; drugId: number; returnToOrder?: { orderId: number; patientId?: number; preparationDate?: string } }
  | { kind: "drugCreate" }
  | { kind: "drugEdit"; drug: DrugDetailType; returnToOrder?: { orderId: number; patientId?: number; preparationDate?: string } }
  | { kind: "regimens" }
  | { kind: "regimenDetail"; regimenId: number }
  | { kind: "regimenCreate" }
  | { kind: "regimenEdit"; regimen: RegimenDetailType }
  | { kind: "orders" }
  | { kind: "orderDetail"; orderId: number; patientId?: number; preparationDate?: string }
  | { kind: "orderCreate"; patientId?: number; initialPatientHn?: string }
  | { kind: "orderEdit"; order: OrderDetailType; patientId?: number; preparationDate?: string }
  | { kind: "preparation" }
  | { kind: "preparationWorkspace"; orderId: number; preparationDate: string }
  | { kind: "reports" }
  | { kind: "inventory" }
  | { kind: "inventoryDetail"; drugId: number }
  | { kind: "account" }
  | { kind: "general" }
  | { kind: "users" }
  | { kind: "guidance" }
  | { kind: "doctorsMaster" }
  | { kind: "wardsMaster" }
  | { kind: "diluentsMaster" }
  | { kind: "routesMaster" }
  | { kind: "diagnosesMaster" }
  | { kind: "hardware" }
  | { kind: "backup" }
  | { kind: "status" };

export default function App() {
  const [startup, setStartup] = useState<{ loading: boolean; status: StartupStatus | null; error: string | null }>({ loading: true, status: null, error: null });
  const loadStartup = useCallback(async () => {
    setStartup({ loading: true, status: null, error: null });
    try { setStartup({ loading: false, status: await getStartupStatus(), error: null }); }
    catch (error) { setStartup({ loading: false, status: null, error: error instanceof Error ? error.message : String(error) }); }
  }, []);
  useEffect(() => { void loadStartup(); }, [loadStartup]);
  if (startup.loading) return <AuthFrame eyebrow="Local startup" title="Opening OncoFlow" summary="Checking the local SQLite database before clinical data is loaded."><div className="auth-loading" aria-busy="true">Preparing the local workspace…</div></AuthFrame>;
  if (!startup.status) return <AuthFrame eyebrow="Local startup" title="Startup status unavailable" summary="OncoFlow could not determine whether the local database is safe to open."><div className="auth-error" role="alert">{startup.error ?? "Unknown local startup error."}</div><button className="button button--secondary auth-submit" type="button" onClick={() => void loadStartup()}>Try again</button></AuthFrame>;
  if (!startup.status.databaseReady) return <DatabaseRecoveryScreen status={startup.status} onReady={(status) => setStartup({ loading: false, status, error: null })} />;
  return <AuthenticationGate />;
}

function AuthenticationGate() {
  const [auth, setAuth] = useState<{ loading: boolean; state: AuthState | null; error: string | null }>({ loading: true, state: null, error: null });
  const loadAuth = useCallback(async () => {
    setAuth({ loading: true, state: null, error: null });
    try { setAuth({ loading: false, state: await getAuthState(), error: null }); }
    catch (error) { setAuth({ loading: false, state: null, error: error instanceof Error ? error.message : String(error) }); }
  }, []);
  useEffect(() => { void loadAuth(); }, [loadAuth]);
  if (auth.loading) return <AuthFrame eyebrow="Local startup" title="Opening OncoFlow" summary="Checking the local account and SQLite database."><div className="auth-loading" aria-busy="true">Preparing the local workspace…</div></AuthFrame>;
  if (!auth.state) return <AuthFrame eyebrow="Local startup" title="Authentication unavailable" summary="The local account state could not be loaded."><div className="auth-error" role="alert">{auth.error ?? "Unknown local authentication error."}</div><button className="button button--secondary auth-submit" type="button" onClick={() => void loadAuth()}>Try again</button></AuthFrame>;
  if (auth.state.needsBootstrap) return <FirstRunSetup onAuthenticated={(state) => setAuth({ loading: false, state, error: null })} />;
  if (!auth.state.authenticated || !auth.state.currentUser) return <LoginScreen onAuthenticated={(state) => setAuth({ loading: false, state, error: null })} />;
  return <AuthenticatedApp user={auth.state.currentUser} onAuthState={(state) => setAuth({ loading: false, state, error: null })} />;
}

function AuthenticatedApp({ user, onAuthState }: { user: CurrentUser; onAuthState: (state: AuthState) => void }) {
  const [view, setView] = useState<View>({ kind: "patients" });
  const [sidebarCollapsed, setSidebarCollapsed] = useState(() => {
    if (typeof window === "undefined") return false;
    try { return window.localStorage.getItem("oncoflow_sidebar_collapsed") === "true"; }
    catch { return false; }
  });
  const [medicationExpanded, setMedicationExpanded] = useState(false);
  const [masterDataExpanded, setMasterDataExpanded] = useState(false);
  const [settingsExpanded, setSettingsExpanded] = useState(false);
  const [logoutError, setLogoutError] = useState<string | null>(null);
  const [logoutBusy, setLogoutBusy] = useState(false);
  const medicationActive = ["drugs", "drugDetail", "drugCreate", "drugEdit", "regimens", "regimenDetail", "regimenCreate", "regimenEdit", "inventory", "inventoryDetail"].includes(view.kind);
  const masterDataActive = ["doctorsMaster", "wardsMaster", "diluentsMaster", "routesMaster", "diagnosesMaster"].includes(view.kind);
  const settingsActive = ["general", "account", "users", "guidance", "hardware", "backup", "status"].includes(view.kind);

  useEffect(() => {
    if (medicationActive) setMedicationExpanded(true);
  }, [medicationActive]);

  useEffect(() => {
    if (masterDataActive) setMasterDataExpanded(true);
  }, [masterDataActive]);

  useEffect(() => {
    if (settingsActive) setSettingsExpanded(true);
  }, [settingsActive]);

  async function signOut() {
    setLogoutError(null); setLogoutBusy(true);
    try { onAuthState(await logoutUser()); }
    catch (error) { setLogoutError(error instanceof Error ? error.message : String(error)); }
    finally { setLogoutBusy(false); }
  }

  function toggleSidebar() {
    setSidebarCollapsed((current) => {
      const next = !current;
      try { window.localStorage.setItem("oncoflow_sidebar_collapsed", String(next)); }
      catch { /* The layout still works when storage is unavailable. */ }
      return next;
    });
  }

  return (
    <PageGuidanceProvider>
    <div className={sidebarCollapsed ? "desktop-shell is-sidebar-collapsed" : "desktop-shell"}>
      <aside className="sidebar">
        <div className="brand">
          <div className="brand-mark" aria-hidden="true">O</div>
          <div className="brand-copy">
            <span className="brand-name">OncoFlow</span>
            <span className="brand-subtitle">Clinical workspace</span>
          </div>
          <button className="sidebar-collapse-toggle" type="button" aria-label={sidebarCollapsed ? "Expand sidebar" : "Collapse sidebar"} aria-expanded={!sidebarCollapsed} aria-controls="main-sidebar-navigation" title={sidebarCollapsed ? "Expand sidebar" : "Collapse sidebar"} onClick={toggleSidebar}>{sidebarCollapsed ? "›" : "‹"}</button>
        </div>

        <nav className="sidebar-nav" id="main-sidebar-navigation" aria-label="Main navigation">
          <section className="nav-group nav-group--workspace" aria-labelledby="workspace-nav-heading">
            <h2 className="nav-group__title" id="workspace-nav-heading">Workspace</h2>
            <div className="nav-group__items">
              <button
                type="button"
                className={["patients", "detail", "create", "edit"].includes(view.kind) ? "nav-item is-active" : "nav-item"}
                onClick={() => setView({ kind: "patients" })}
                title="Patients"
              >
                <span className="nav-icon" aria-hidden="true">♙</span>
                Patients
              </button>
              <button
                type="button"
                className={["orders", "orderDetail", "orderCreate", "orderEdit"].includes(view.kind) ? "nav-item is-active" : "nav-item"}
                onClick={() => setView({ kind: "orders" })}
                title="Orders"
              >
                <span className="nav-icon" aria-hidden="true">▤</span>
                Orders
              </button>
              <button
                type="button"
                className={["preparation", "preparationWorkspace"].includes(view.kind) ? "nav-item is-active" : "nav-item"}
                onClick={() => setView({ kind: "preparation" })}
                title="Preparation"
              >
                <span className="nav-icon" aria-hidden="true">⌁</span>
                Preparation
              </button>
              <button
                type="button"
                className={view.kind === "reports" ? "nav-item is-active" : "nav-item"}
                onClick={() => setView({ kind: "reports" })}
                title="Reports"
              >
                <span className="nav-icon" aria-hidden="true">▥</span>
                Reports
              </button>
            </div>
          </section>

          <CollapsibleNavGroup
            id="medication-navigation"
            title="Medication management"
            expanded={medicationExpanded}
            active={medicationActive}
            onToggle={() => setMedicationExpanded((value) => !value)}
          >
            <button
              type="button"
              className={["drugs", "drugDetail", "drugCreate", "drugEdit"].includes(view.kind) ? "nav-item is-active" : "nav-item"}
              onClick={() => setView({ kind: "drugs" })}
              title="Drugs"
            >
              <span className="nav-icon" aria-hidden="true">Rx</span>
              Drugs
            </button>
            <button
              type="button"
              className={["regimens", "regimenDetail", "regimenCreate", "regimenEdit"].includes(view.kind) ? "nav-item is-active" : "nav-item"}
              onClick={() => setView({ kind: "regimens" })}
              title="Regimens"
            >
              <span className="nav-icon" aria-hidden="true">≋</span>
              Regimens
            </button>
            <button
              type="button"
              className={["inventory", "inventoryDetail"].includes(view.kind) ? "nav-item is-active" : "nav-item"}
              onClick={() => setView({ kind: "inventory" })}
              title="Inventory"
            >
              <span className="nav-icon" aria-hidden="true">▦</span>
              Inventory
            </button>
          </CollapsibleNavGroup>

          {user.role === "admin" && <CollapsibleNavGroup
            id="master-data-navigation"
            title="Master data"
            expanded={masterDataExpanded}
            active={masterDataActive}
            onToggle={() => setMasterDataExpanded((value) => !value)}
          >
              <button
                type="button"
                className={view.kind === "doctorsMaster" ? "nav-item is-active" : "nav-item"}
                onClick={() => setView({ kind: "doctorsMaster" })}
                title="Doctors"
              >
                <span className="nav-icon" aria-hidden="true">✚</span>
                Doctors
              </button>
              <button
                type="button"
                className={view.kind === "wardsMaster" ? "nav-item is-active" : "nav-item"}
                onClick={() => setView({ kind: "wardsMaster" })}
                title="Wards"
              >
                <span className="nav-icon" aria-hidden="true">⌂</span>
                Wards
              </button>
              <button
                type="button"
                className={view.kind === "diluentsMaster" ? "nav-item is-active" : "nav-item"}
                onClick={() => setView({ kind: "diluentsMaster" })}
                title="Diluents"
              >
                <span className="nav-icon" aria-hidden="true">◒</span>
                Diluents
              </button>
              <button
                type="button"
                className={view.kind === "routesMaster" ? "nav-item is-active" : "nav-item"}
                onClick={() => setView({ kind: "routesMaster" })}
                title="Routes"
              >
                <span className="nav-icon" aria-hidden="true">↗</span>
                Routes
              </button>
              <button
                type="button"
                className={view.kind === "diagnosesMaster" ? "nav-item is-active" : "nav-item"}
                onClick={() => setView({ kind: "diagnosesMaster" })}
                title="Diagnosis"
              >
                <span className="nav-icon" aria-hidden="true">◇</span>
                Diagnosis
              </button>
          </CollapsibleNavGroup>}

          <CollapsibleNavGroup
            id="settings-navigation"
            title="Settings"
            expanded={settingsExpanded}
            active={settingsActive}
            onToggle={() => setSettingsExpanded((value) => !value)}
          >
            {user.role === "admin" && <button
              type="button"
              className={view.kind === "general" ? "nav-item is-active" : "nav-item"}
              onClick={() => setView({ kind: "general" })}
              title="General"
            >
              <span className="nav-icon" aria-hidden="true">⚙</span>
              General
            </button>}
            <button
              type="button"
              className={view.kind === "account" ? "nav-item is-active" : "nav-item"}
              onClick={() => setView({ kind: "account" })}
              title="Account"
            >
              <span className="nav-icon" aria-hidden="true">◎</span>
              Account
            </button>
            {user.role === "admin" && <button
              type="button"
              className={view.kind === "users" ? "nav-item is-active" : "nav-item"}
              onClick={() => setView({ kind: "users" })}
              title="Users"
            >
              <span className="nav-icon" aria-hidden="true">♟</span>
              Users
            </button>}
            {user.role === "admin" && <button
              type="button"
              className={view.kind === "guidance" ? "nav-item is-active" : "nav-item"}
              onClick={() => setView({ kind: "guidance" })}
              title="Guidance"
            >
              <span className="nav-icon" aria-hidden="true">✎</span>
              Guidance
            </button>}
            <button
              type="button"
              className={view.kind === "hardware" ? "nav-item is-active" : "nav-item"}
              onClick={() => setView({ kind: "hardware" })}
              title="Hardware"
            >
              <span className="nav-icon" aria-hidden="true">▣</span>
              Hardware
            </button>
            <button
              type="button"
              className={view.kind === "backup" ? "nav-item is-active" : "nav-item"}
              onClick={() => setView({ kind: "backup" })}
              title="Backup & restore"
            >
              <span className="nav-icon" aria-hidden="true">↺</span>
              Backup &amp; restore
            </button>
            <button
              type="button"
              className={view.kind === "status" ? "nav-item is-active" : "nav-item"}
              onClick={() => setView({ kind: "status" })}
              title="Diagnostics"
            >
              <span className="nav-icon" aria-hidden="true">◉</span>
              Diagnostics
            </button>
          </CollapsibleNavGroup>
        </nav>

        <SessionIdentity user={user} busy={logoutBusy} error={logoutError} onLogout={() => void signOut()} />

        <div className="local-badge" title="Local SQLite database">
          <span className="local-badge__dot" aria-hidden="true" />
          <div>
            <strong>Local-only</strong>
            <span>oncoflow.db</span>
          </div>
        </div>
      </aside>

      <main className="main-content">
        {view.kind === "patients" && (
          <PatientList
            onCreate={() => setView({ kind: "create" })}
            onOpen={(patientId) => setView({ kind: "detail", patientId })}
          />
        )}
        {view.kind === "detail" && (
          <PatientDetail
            patientId={view.patientId}
            onBack={() => setView({ kind: "patients" })}
            onEdit={(patient) => setView({ kind: "edit", patient })}
            onOpenOrder={(orderId) => setView({ kind: "orderDetail", orderId, patientId: view.patientId })}
            onCreateOrder={(patientId) => setView({ kind: "orderCreate", patientId })}
          />
        )}
        {view.kind === "create" && (
          <PatientForm
            initialHn={view.returnToOrderHn}
            onCancel={() => view.returnToOrderHn !== undefined ? setView({ kind: "orderCreate", initialPatientHn: view.returnToOrderHn }) : setView({ kind: "patients" })}
            onSaved={(patient) => view.returnToOrderHn !== undefined ? setView({ kind: "orderCreate", patientId: patient.id }) : setView({ kind: "detail", patientId: patient.id })}
          />
        )}
        {view.kind === "edit" && (
          <PatientForm
            patient={view.patient}
            onCancel={() => setView({ kind: "detail", patientId: view.patient.id })}
            onSaved={(patient) => setView({ kind: "detail", patientId: patient.id })}
          />
        )}
        {view.kind === "drugs" && (
          <DrugList
            onCreate={() => setView({ kind: "drugCreate" })}
            onOpen={(drugId) => setView({ kind: "drugDetail", drugId })}
          />
        )}
        {view.kind === "drugDetail" && (
          <DrugDetail
            drugId={view.drugId}
            onBack={() => view.returnToOrder ? setView({ kind: "orderDetail", ...view.returnToOrder }) : setView({ kind: "drugs" })}
            onEdit={(drug) => setView({ kind: "drugEdit", drug, returnToOrder: view.returnToOrder })}
          />
        )}
        {view.kind === "drugCreate" && (
          <DrugForm
            onCancel={() => setView({ kind: "drugs" })}
            onSaved={(drug) => setView({ kind: "drugDetail", drugId: drug.id })}
          />
        )}
        {view.kind === "drugEdit" && (
          <DrugForm
            drug={view.drug}
            onCancel={() => setView({ kind: "drugDetail", drugId: view.drug.id, returnToOrder: view.returnToOrder })}
            onSaved={(drug) => setView({ kind: "drugDetail", drugId: drug.id, returnToOrder: view.returnToOrder })}
          />
        )}
        {view.kind === "regimens" && (
          <RegimenList
            onCreate={() => setView({ kind: "regimenCreate" })}
            onOpen={(regimenId) => setView({ kind: "regimenDetail", regimenId })}
          />
        )}
        {view.kind === "regimenDetail" && (
          <RegimenDetail
            regimenId={view.regimenId}
            onBack={() => setView({ kind: "regimens" })}
            onEdit={(regimen) => setView({ kind: "regimenEdit", regimen })}
          />
        )}
        {view.kind === "regimenCreate" && (
          <RegimenForm
            onCancel={() => setView({ kind: "regimens" })}
            onSaved={(regimen) => setView({ kind: "regimenDetail", regimenId: regimen.id })}
          />
        )}
        {view.kind === "regimenEdit" && (
          <RegimenForm
            regimen={view.regimen}
            onCancel={() => setView({ kind: "regimenDetail", regimenId: view.regimen.id })}
            onSaved={(regimen) => setView({ kind: "regimenDetail", regimenId: regimen.id })}
          />
        )}
        {view.kind === "orders" && (
          <OrderList
            onCreate={() => setView({ kind: "orderCreate" })}
            onOpen={(orderId) => setView({ kind: "orderDetail", orderId })}
          />
        )}
        {view.kind === "orderDetail" && (
          <OrderDetail
            orderId={view.orderId}
            backLabel={view.preparationDate ? "Chemotherapy preparation" : view.patientId ? "Patient" : "Orders"}
            onBack={() => view.preparationDate ? setView({ kind: "preparationWorkspace", orderId: view.orderId, preparationDate: view.preparationDate }) : view.patientId ? setView({ kind: "detail", patientId: view.patientId }) : setView({ kind: "orders" })}
            onEdit={(order) => setView({ kind: "orderEdit", order, patientId: view.patientId, preparationDate: view.preparationDate })}
            onOpenOrder={(orderId) => setView({ kind: "orderDetail", orderId, patientId: view.patientId })}
            onOpenDrug={user.role === "admin" ? (drugId) => setView({ kind: "drugDetail", drugId, returnToOrder: { orderId: view.orderId, patientId: view.patientId, preparationDate: view.preparationDate } }) : undefined}
          />
        )}
        {view.kind === "orderCreate" && (
          <OrderForm
            initialPatientId={view.patientId}
            initialPatientHn={view.initialPatientHn}
            onCreatePatient={(hn) => setView({ kind: "create", returnToOrderHn: hn })}
            onCancel={() => view.patientId ? setView({ kind: "detail", patientId: view.patientId }) : setView({ kind: "orders" })}
            onSaved={(order) => setView({ kind: "orderDetail", orderId: order.id, patientId: view.patientId })}
          />
        )}
        {view.kind === "orderEdit" && (
          <OrderForm
            order={view.order}
            onCancel={() => setView({ kind: "orderDetail", orderId: view.order.id, patientId: view.patientId, preparationDate: view.preparationDate })}
            onSaved={(order) => setView({ kind: "orderDetail", orderId: order.id, patientId: view.patientId, preparationDate: view.preparationDate })}
          />
        )}
        {view.kind === "preparation" && (
          <PreparationQueue onOpen={(orderId, preparationDate) => setView({ kind: "preparationWorkspace", orderId, preparationDate })} />
        )}
        {view.kind === "preparationWorkspace" && (
          <PreparationWorkspace orderId={view.orderId} preparationDate={view.preparationDate} onBack={() => setView({ kind: "preparation" })} onOpenOrder={() => setView({ kind: "orderDetail", orderId: view.orderId, preparationDate: view.preparationDate })} />
        )}
        {view.kind === "reports" && <PreparationCountReport />}
        {view.kind === "inventory" && (
          <InventoryList onOpen={(drugId) => setView({ kind: "inventoryDetail", drugId })} />
        )}
        {view.kind === "inventoryDetail" && (
          <InventoryDetail drugId={view.drugId} onBack={() => setView({ kind: "inventory" })} />
        )}
        {view.kind === "account" && <AccountSettings user={user} />}
        {view.kind === "general" && user.role === "admin" && <GeneralSettings />}
        {view.kind === "users" && user.role === "admin" && <UserManagement currentUserId={user.id} />}
        {view.kind === "guidance" && user.role === "admin" && <GuidanceSettings />}
        {view.kind === "doctorsMaster" && user.role === "admin" && <DoctorsPage />}
        {view.kind === "wardsMaster" && user.role === "admin" && <WardsPage />}
        {view.kind === "diluentsMaster" && user.role === "admin" && <DiluentsPage />}
        {view.kind === "routesMaster" && user.role === "admin" && <RoutesPage />}
        {view.kind === "diagnosesMaster" && user.role === "admin" && <DiagnosisPage />}
        {view.kind === "hardware" && <HardwareSettings />}
        {view.kind === "backup" && <BackupRestore />}
        {view.kind === "status" && <Diagnostics />}
      </main>
    </div>
    </PageGuidanceProvider>
  );
}

function CollapsibleNavGroup({ id, title, expanded, active, onToggle, children }: { id: string; title: string; expanded: boolean; active: boolean; onToggle: () => void; children: React.ReactNode }) {
  return (
    <section className="nav-group">
      <button
        className={`nav-group__toggle ${active ? "has-active" : ""}`.trim()}
        type="button"
        aria-expanded={expanded}
        aria-controls={id}
        title={title}
        onClick={onToggle}
      >
        <span>{title}</span>
        <span className="nav-group__chevron" aria-hidden="true">›</span>
      </button>
      <div className="nav-group__items" id={id} hidden={!expanded}>{children}</div>
    </section>
  );
}
