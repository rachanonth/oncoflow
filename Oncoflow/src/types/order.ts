export type OrderSortField = "date" | "orderId" | "patient";
export type SortDirection = "asc" | "desc";
export type OrderWorkflowStatus = "active" | "on_hold" | "legacy";

export interface OrderStatusEvent {
  id: number;
  eventType: "no_show" | "rescheduled";
  fromStatus: OrderWorkflowStatus;
  toStatus: OrderWorkflowStatus;
  effectiveDate: string;
  relatedDate: string | null;
  actorDisplayName: string;
  occurredAt: string;
}

export interface CumulativeDoseSummary {
  drugId: number;
  drugName: string;
  totalDose: string | null;
  threshold: string | null;
}

export interface OrderListRequest {
  search?: string | null;
  patientId?: number | null;
  dateFrom?: string | null;
  dateTo?: string | null;
  sortBy?: OrderSortField;
  sortDirection?: SortDirection;
  limit?: number;
  offset?: number;
}

export interface OrderSummary {
  id: number;
  orderId: string;
  patientId: number;
  patientHn: string;
  patientName: string;
  orderTime: string | null;
  regimenName: string | null;
  doctorName: string | null;
  wardName: string | null;
  orderType: string | null;
  itemCount: number;
  drugs: Array<{ drugName: string; doseText: string | null; unitText: string | null }>;
  editable: boolean;
  workflowStatus: OrderWorkflowStatus;
}

export interface OrderListResponse { items: OrderSummary[]; total: number }

export interface OrderItemDetail {
  id: number;
  drugId: number;
  drugName: string;
  diluentId: number | null;
  diluentName: string | null;
  diluentVolumeMl: number | null;
  routeId: number | null;
  routeName: string | null;
  startDate: string | null;
  stopDate: string | null;
  dose: number | null;
  doseText: string | null;
  scheduleTime: string | null;
  numberOfDrug: number | null;
  missing: boolean;
  printed: boolean;
  rate: string | null;
  orderingNo: number | null;
  runningNo: number | null;
  runningSum: number | null;
  inventoryDate: string | null;
  sourceRegimenItemId: number | null;
  regimenDoseText: string | null;
  regimenUnitText: string | null;
  regimenRouteText: string | null;
  regimenDetails: string | null;
  regimenItemGroup: string | null;
  regimenDuration: string | null;
  regimenStartDay: number | null;
  regimenOrderingNo: number | null;
}

export interface OrderDetail {
  id: number;
  orderId: string;
  patientId: number;
  patientHn: string;
  patientName: string;
  weightKg: number | null;
  heightCm: number | null;
  assignedPreparerUserId: number | null;
  assignedPreparerName: string | null;
  wardId: number | null;
  wardName: string | null;
  doctorId: number | null;
  doctorName: string | null;
  regimenId: number | null;
  regimenName: string | null;
  note: string | null;
  orderTime: string | null;
  orderType: string | null;
  appointmentFlag: boolean;
  legacyWorker: string | null;
  editWorker: string | null;
  sideEffectText: string | null;
  sideEffectRecorder: string | null;
  sideEffectRecordTime: string | null;
  medicationErrorText: string | null;
  editable: boolean;
  workflowStatus: OrderWorkflowStatus;
  workflowStatusReason: string | null;
  workflowStatusChangedAt: string | null;
  workflowStatusChangedBy: string | null;
  statusEvents: OrderStatusEvent[];
  cumulativeDoses: CumulativeDoseSummary[];
  items: OrderItemDetail[];
}

export interface OrderNoShowInput { scheduledDate: string }
export interface OrderRescheduleInput { missedDate: string; newDate: string }

export interface OrderInput {
  patientId: number;
  wardId: number | null;
  doctorId: number | null;
  regimenId: number | null;
  note: string | null;
  orderTime: string | null;
  orderType: string | null;
  appointmentFlag: boolean;
  assignedPreparerUserId: number | null;
}

export interface OrderWeightInput {
  weightKg: number | null;
}

export interface OrderItemInput {
  drugId: number;
  diluentId: number | null;
  diluentVolumeMl: number | null;
  routeId: number | null;
  startDate: string | null;
  stopDate: string | null;
  doseText: string | null;
  scheduleTime: string | null;
  numberOfDrug: number | null;
  missing: boolean;
  rate: string | null;
}

export interface OrderReorderInput { itemIds: number[] }
export interface OrderLookupOption { id: number; label: string }
export interface PatientOrderLookupOption extends OrderLookupOption { hn: string }
export interface DiluentOrderLookupOption extends OrderLookupOption { volumeMl: number | null }
export interface OrderLookups {
  patients: PatientOrderLookupOption[];
  regimens: OrderLookupOption[];
  drugs: OrderLookupOption[];
  routes: OrderLookupOption[];
  diluents: DiluentOrderLookupOption[];
  doctors: OrderLookupOption[];
  wards: OrderLookupOption[];
  preparationPharmacists: OrderLookupOption[];
}
