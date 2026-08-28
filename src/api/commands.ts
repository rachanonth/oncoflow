import { invoke } from "@tauri-apps/api/core";

import type { HealthStatus } from "../types/health";
import type {
  DrugDetail,
  DrugFormOptions,
  DrugInput,
  DrugListRequest,
  DrugListResponse,
} from "../types/drug";
import type {
  PatientDetail,
  PatientFormOptions,
  PatientInput,
  PatientListRequest,
  PatientListResponse,
} from "../types/patient";
import type {
  RegimenDetail,
  RegimenGroupInput,
  RegimenInput,
  RegimenItemInput,
  RegimenListRequest,
  RegimenListResponse,
  RegimenLookups,
  RegimenReorderInput,
} from "../types/regimen";
import type {
  OrderDetail,
  OrderInput,
  OrderItemInput,
  OrderListRequest,
  OrderListResponse,
  OrderLookups,
  OrderReorderInput,
} from "../types/order";
import type { SafetyEvaluation } from "../types/safety";
import type {
  PreparationQueueRequest,
  PreparationQueueResponse,
  PreparationTask,
  PreparationTaskInput,
  PreparationWorkspace,
} from "../types/preparation";
import type {
  AuthState,
  BootstrapUserInput,
  ChangePasswordInput,
  CreateUserInput,
  CurrentUser,
  LoginInput,
  ManagedUser,
  UpdateUserInput,
} from "../types/auth";
import type {
  InventoryAdjustmentInput,
  InventoryDetail,
  InventoryListRequest,
  InventoryListResponse,
  InventoryManualIssueInput,
  InventoryMovementListResponse,
  InventoryMovementResult,
  InventoryReceiptInput,
} from "../types/inventory";
import type { PreparationOutput } from "../types/output";
import type { InventoryUsageReport, InventoryUsageReportRequest, PreparationCountReport, PreparationCountReportRequest } from "../types/report";
import type { LabelPrinterConfig, PreparationBatchPrintResult, PreparationPrintResult, PrintJobReceipt } from "../types/hardware";
import type {
  BackupResult,
  Diagnostics,
  PrinterQueueStatus,
  RestoreInput,
  RestorePreflight,
  RestoreResult,
  StartupStatus,
} from "../types/recovery";
import type {
  DoctorInput,
  DoctorRecord,
  DiagnosisInput,
  DiagnosisRecord,
  DiluentInput,
  DiluentRecord,
  MasterDataListRequest,
  RouteInput,
  RouteRecord,
  WardInput,
  WardRecord,
} from "../types/masterData";
import type { PageGuidanceRecord, UpdatePageGuidanceInput } from "../types/guidance";
import type { ApplicationSettings, UpdateApplicationSettingsInput } from "../types/settings";

export function getHealthStatus(): Promise<HealthStatus> {
  return invoke<HealthStatus>("health_status");
}

export function getAuthState(): Promise<AuthState> { return invoke<AuthState>("get_auth_state"); }
export function bootstrapUser(input: BootstrapUserInput): Promise<AuthState> { return invoke<AuthState>("bootstrap_user", { input }); }
export function loginUser(input: LoginInput): Promise<AuthState> { return invoke<AuthState>("login", { input }); }
export function logoutUser(): Promise<AuthState> { return invoke<AuthState>("logout"); }
export function getCurrentUser(): Promise<CurrentUser> { return invoke<CurrentUser>("get_current_user"); }
export function changePassword(input: ChangePasswordInput): Promise<void> { return invoke<void>("change_password", { input }); }
export function listUsers(): Promise<ManagedUser[]> { return invoke<ManagedUser[]>("list_users"); }
export function createUser(input: CreateUserInput): Promise<ManagedUser> { return invoke<ManagedUser>("create_user", { input }); }
export function updateUser(userId: number, input: UpdateUserInput): Promise<ManagedUser> { return invoke<ManagedUser>("update_user", { userId, input }); }
export function listPageGuidance(): Promise<PageGuidanceRecord[]> { return invoke<PageGuidanceRecord[]>("list_page_guidance"); }
export function updatePageGuidance(input: UpdatePageGuidanceInput): Promise<PageGuidanceRecord> { return invoke<PageGuidanceRecord>("update_page_guidance", { input }); }
export function getApplicationSettings(): Promise<ApplicationSettings> { return invoke<ApplicationSettings>("get_application_settings"); }
export function updateApplicationSettings(input: UpdateApplicationSettingsInput): Promise<ApplicationSettings> { return invoke<ApplicationSettings>("update_application_settings", { input }); }

export function listDoctors(request: MasterDataListRequest): Promise<DoctorRecord[]> { return invoke<DoctorRecord[]>("list_doctors", { request }); }
export function createDoctor(input: DoctorInput): Promise<DoctorRecord> { return invoke<DoctorRecord>("create_doctor", { input }); }
export function updateDoctor(doctorId: number, input: DoctorInput): Promise<DoctorRecord> { return invoke<DoctorRecord>("update_doctor", { doctorId, input }); }
export function listWards(request: MasterDataListRequest): Promise<WardRecord[]> { return invoke<WardRecord[]>("list_wards", { request }); }
export function createWard(input: WardInput): Promise<WardRecord> { return invoke<WardRecord>("create_ward", { input }); }
export function updateWard(wardId: number, input: WardInput): Promise<WardRecord> { return invoke<WardRecord>("update_ward", { wardId, input }); }
export function listRoutes(request: MasterDataListRequest): Promise<RouteRecord[]> { return invoke<RouteRecord[]>("list_routes", { request }); }
export function createRoute(input: RouteInput): Promise<RouteRecord> { return invoke<RouteRecord>("create_route", { input }); }
export function updateRoute(routeId: number, input: RouteInput): Promise<RouteRecord> { return invoke<RouteRecord>("update_route", { routeId, input }); }
export function listDiluents(request: MasterDataListRequest): Promise<DiluentRecord[]> { return invoke<DiluentRecord[]>("list_diluents", { request }); }
export function createDiluent(input: DiluentInput): Promise<DiluentRecord> { return invoke<DiluentRecord>("create_diluent", { input }); }
export function updateDiluent(diluentId: number, input: DiluentInput): Promise<DiluentRecord> { return invoke<DiluentRecord>("update_diluent", { diluentId, input }); }
export function listDiagnoses(request: MasterDataListRequest): Promise<DiagnosisRecord[]> { return invoke<DiagnosisRecord[]>("list_diagnoses", { request }); }
export function createDiagnosis(input: DiagnosisInput): Promise<DiagnosisRecord> { return invoke<DiagnosisRecord>("create_diagnosis", { input }); }
export function updateDiagnosis(diagnosisId: number, input: DiagnosisInput): Promise<DiagnosisRecord> { return invoke<DiagnosisRecord>("update_diagnosis", { diagnosisId, input }); }

export function listDrugs(request: DrugListRequest): Promise<DrugListResponse> {
  return invoke<DrugListResponse>("list_drugs", { request });
}

export function getDrug(drugId: number): Promise<DrugDetail> {
  return invoke<DrugDetail>("get_drug", { drugId });
}

export function createDrug(input: DrugInput): Promise<DrugDetail> {
  return invoke<DrugDetail>("create_drug", { input });
}

export function updateDrug(drugId: number, input: DrugInput): Promise<DrugDetail> {
  return invoke<DrugDetail>("update_drug", { drugId, input });
}

export function getDrugFormOptions(): Promise<DrugFormOptions> {
  return invoke<DrugFormOptions>("drug_form_options");
}

export function listRegimens(request: RegimenListRequest): Promise<RegimenListResponse> {
  return invoke<RegimenListResponse>("list_regimens", { request });
}

export function getRegimen(regimenId: number): Promise<RegimenDetail> {
  return invoke<RegimenDetail>("get_regimen", { regimenId });
}

export function createRegimen(input: RegimenInput): Promise<RegimenDetail> {
  return invoke<RegimenDetail>("create_regimen", { input });
}

export function updateRegimen(regimenId: number, input: RegimenInput): Promise<RegimenDetail> {
  return invoke<RegimenDetail>("update_regimen", { regimenId, input });
}

export function addRegimenGroup(regimenId: number, input: RegimenGroupInput): Promise<RegimenDetail> {
  return invoke<RegimenDetail>("add_regimen_group", { regimenId, input });
}

export function updateRegimenGroup(regimenId: number, groupId: number, input: RegimenGroupInput): Promise<RegimenDetail> {
  return invoke<RegimenDetail>("update_regimen_group", { regimenId, groupId, input });
}

export function deleteRegimenGroup(regimenId: number, groupId: number): Promise<RegimenDetail> {
  return invoke<RegimenDetail>("delete_regimen_group", { regimenId, groupId });
}

export function addRegimenItem(regimenId: number, input: RegimenItemInput): Promise<RegimenDetail> {
  return invoke<RegimenDetail>("add_regimen_item", { regimenId, input });
}

export function updateRegimenItem(regimenId: number, itemId: number, input: RegimenItemInput): Promise<RegimenDetail> {
  return invoke<RegimenDetail>("update_regimen_item", { regimenId, itemId, input });
}

export function deleteRegimenItem(regimenId: number, itemId: number): Promise<RegimenDetail> {
  return invoke<RegimenDetail>("delete_regimen_item", { regimenId, itemId });
}

export function reorderRegimenItems(regimenId: number, input: RegimenReorderInput): Promise<RegimenDetail> {
  return invoke<RegimenDetail>("reorder_regimen_items", { regimenId, input });
}

export function getRegimenLookups(): Promise<RegimenLookups> {
  return invoke<RegimenLookups>("get_regimen_lookups");
}

export function listPatients(
  request: PatientListRequest,
): Promise<PatientListResponse> {
  return invoke<PatientListResponse>("list_patients", { request });
}

export function getPatient(patientId: number): Promise<PatientDetail> {
  return invoke<PatientDetail>("get_patient", { patientId });
}

export function createPatient(input: PatientInput): Promise<PatientDetail> {
  return invoke<PatientDetail>("create_patient", { input });
}

export function updatePatient(
  patientId: number,
  input: PatientInput,
): Promise<PatientDetail> {
  return invoke<PatientDetail>("update_patient", { patientId, input });
}

export function getPatientFormOptions(): Promise<PatientFormOptions> {
  return invoke<PatientFormOptions>("patient_form_options");
}

export function listOrders(request: OrderListRequest): Promise<OrderListResponse> { return invoke<OrderListResponse>("list_orders", { request }); }
export function listPatientOrders(patientId: number): Promise<OrderListResponse> { return invoke<OrderListResponse>("list_patient_orders", { patientId }); }
export function getOrder(orderId: number): Promise<OrderDetail> { return invoke<OrderDetail>("get_order", { orderId }); }
export function createOrder(input: OrderInput): Promise<OrderDetail> { return invoke<OrderDetail>("create_order", { input }); }
export function createOrderFromRegimen(input: OrderInput): Promise<OrderDetail> { return invoke<OrderDetail>("create_order_from_regimen", { input }); }
export function updateOrder(orderId: number, input: OrderInput): Promise<OrderDetail> { return invoke<OrderDetail>("update_order", { orderId, input }); }
export function updateOrderWeight(orderId: number, weightKg: number | null): Promise<OrderDetail> { return invoke<OrderDetail>("update_order_weight", { orderId, input: { weightKg } }); }
export function addOrderItem(orderId: number, input: OrderItemInput): Promise<OrderDetail> { return invoke<OrderDetail>("add_order_item", { orderId, input }); }
export function updateOrderItem(orderId: number, itemId: number, input: OrderItemInput): Promise<OrderDetail> { return invoke<OrderDetail>("update_order_item", { orderId, itemId, input }); }
export function removeOrderItem(orderId: number, itemId: number): Promise<OrderDetail> { return invoke<OrderDetail>("remove_order_item", { orderId, itemId }); }
export function reorderOrderItems(orderId: number, input: OrderReorderInput): Promise<OrderDetail> { return invoke<OrderDetail>("reorder_order_items", { orderId, input }); }
export function getOrderLookups(): Promise<OrderLookups> { return invoke<OrderLookups>("get_order_lookups"); }
export function recordOrderNoShow(orderId: number, scheduledDate: string): Promise<OrderDetail> { return invoke<OrderDetail>("record_order_no_show", { orderId, input: { scheduledDate } }); }
export function rescheduleOrder(orderId: number, missedDate: string, newDate: string): Promise<OrderDetail> { return invoke<OrderDetail>("reschedule_order", { orderId, input: { missedDate, newDate } }); }
export function evaluateOrderSafety(orderId: number): Promise<SafetyEvaluation> { return invoke<SafetyEvaluation>("evaluate_order_safety", { orderId }); }
export function listPreparationQueue(request: PreparationQueueRequest): Promise<PreparationQueueResponse> { return invoke<PreparationQueueResponse>("list_preparation_queue", { request }); }
export function getPreparationWorkspace(orderId: number, preparationDate: string): Promise<PreparationWorkspace> { return invoke<PreparationWorkspace>("get_preparation_workspace", { orderId, preparationDate }); }
export function initializePreparation(orderId: number, preparationDate: string): Promise<PreparationWorkspace> { return invoke<PreparationWorkspace>("initialize_preparation", { orderId, preparationDate }); }
export function updatePreparationTask(taskId: number, input: PreparationTaskInput): Promise<PreparationTask> { return invoke<PreparationTask>("update_preparation_task", { taskId, input }); }
export function markPreparationPrepared(taskId: number, preparedByUserId: number): Promise<PreparationTask> { return invoke<PreparationTask>("mark_preparation_prepared", { taskId, preparedByUserId }); }
export function verifyPreparationTask(taskId: number): Promise<PreparationTask> { return invoke<PreparationTask>("verify_preparation_task", { taskId }); }
export function checkPreparationTask(taskId: number): Promise<PreparationTask> { return invoke<PreparationTask>("check_preparation_task", { taskId }); }
export function checkPreparationTasks(taskIds: number[]): Promise<PreparationTask[]> { return invoke<PreparationTask[]>("check_preparation_tasks", { taskIds }); }
export function acknowledgePreparationSafetyFinding(orderId: number, preparationDate: string, findingId: string): Promise<PreparationWorkspace> { return invoke<PreparationWorkspace>("acknowledge_preparation_safety_finding", { orderId, preparationDate, findingId }); }
export function getPreparationCountReport(request: PreparationCountReportRequest): Promise<PreparationCountReport> { return invoke<PreparationCountReport>("get_preparation_count_report", { request }); }
export function getInventoryUsageReport(request: InventoryUsageReportRequest): Promise<InventoryUsageReport> { return invoke<InventoryUsageReport>("get_inventory_usage_report", { request }); }
export function getPreparationOutput(preparationId: number): Promise<PreparationOutput> { return invoke<PreparationOutput>("get_preparation_output", { preparationId }); }
export function listSystemPrinters(): Promise<string[]> { return invoke<string[]>("list_system_printers"); }
export function printTestLabel(config: LabelPrinterConfig): Promise<PrintJobReceipt> { return invoke<PrintJobReceipt>("print_test_label", { config }); }
export function printPreparationLabel(preparationId: number, config: LabelPrinterConfig): Promise<PreparationPrintResult> { return invoke<PreparationPrintResult>("print_preparation_label", { preparationId, config }); }
export function printOrderPreparationLabels(orderId: number, preparationIds: number[], config: LabelPrinterConfig): Promise<PreparationBatchPrintResult> { return invoke<PreparationBatchPrintResult>("print_order_preparation_labels", { orderId, preparationIds, config }); }
export function validatePrinterQueue(spoolerName: string | null): Promise<PrinterQueueStatus> { return invoke<PrinterQueueStatus>("validate_printer_queue", { spoolerName }); }

export function getStartupStatus(): Promise<StartupStatus> { return invoke<StartupStatus>("get_startup_status"); }
export function retryDatabaseInitialization(): Promise<StartupStatus> { return invoke<StartupStatus>("retry_database_initialization"); }
export function createDatabaseBackup(destinationDirectory: string): Promise<BackupResult> { return invoke<BackupResult>("create_database_backup", { destinationDirectory }); }
export function preflightDatabaseRestore(backupPath: string): Promise<RestorePreflight> { return invoke<RestorePreflight>("preflight_database_restore", { backupPath }); }
export function restoreDatabase(input: RestoreInput): Promise<RestoreResult> { return invoke<RestoreResult>("restore_database", { input }); }
export function getDiagnostics(): Promise<Diagnostics> { return invoke<Diagnostics>("get_diagnostics"); }
export function openDataFolder(): Promise<void> { return invoke<void>("open_data_folder"); }

export function listInventory(request: InventoryListRequest): Promise<InventoryListResponse> {
  return invoke<InventoryListResponse>("list_inventory", { request });
}
export function getLowStockItems(request: InventoryListRequest): Promise<InventoryListResponse> {
  return invoke<InventoryListResponse>("get_low_stock_items", { request });
}
export function getInventoryItem(drugId: number): Promise<InventoryDetail> {
  return invoke<InventoryDetail>("get_inventory_item", { drugId });
}
export function listInventoryMovements(drugId: number): Promise<InventoryMovementListResponse> {
  return invoke<InventoryMovementListResponse>("list_inventory_movements", { request: { drugId, limit: 500, offset: 0 } });
}
export function recordInventoryReceipt(input: InventoryReceiptInput): Promise<InventoryMovementResult> {
  return invoke<InventoryMovementResult>("record_inventory_receipt", { input });
}
export function recordInventoryAdjustment(input: InventoryAdjustmentInput): Promise<InventoryMovementResult> {
  return invoke<InventoryMovementResult>("record_inventory_adjustment", { input });
}
export function recordInventoryManualIssue(input: InventoryManualIssueInput): Promise<InventoryMovementResult> {
  return invoke<InventoryMovementResult>("record_inventory_manual_issue", { input });
}

export interface BackendCommandError {
  code?: string;
  message?: string;
  field?: string | null;
}

export function commandError(error: unknown): BackendCommandError {
  if (typeof error === "object" && error !== null) {
    const value = error as Record<string, unknown>;
    return {
      code: typeof value.code === "string" ? value.code : undefined,
      message:
        typeof value.message === "string" ? value.message : "Request failed.",
      field: typeof value.field === "string" ? value.field : null,
    };
  }
  return {
    message: error instanceof Error ? error.message : String(error),
  };
}
