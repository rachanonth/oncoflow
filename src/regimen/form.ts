import type {
  RegimenDetail,
  RegimenGroupDetail,
  RegimenGroupInput,
  RegimenInput,
  RegimenItemDetail,
  RegimenItemInput,
} from "../types/regimen";

export interface RegimenFormValues {
  code: string;
  name: string;
  marker: boolean;
  flag: boolean;
  cycleCheck: boolean;
  autoMode: boolean;
  drugAlert: boolean;
  appointmentAlert: boolean;
  counselAlert: boolean;
}

export interface RegimenGroupFormValues {
  note: string;
  cycleDay: string;
  cycleCount: string;
}

export interface RegimenItemFormValues {
  regimenGroupId: string;
  drugId: string;
  doseText: string;
  unitText: string;
  routeText: string;
  details: string;
  itemGroup: string;
  duration: string;
  startDay: string;
  orderingNo: string;
  defaultDiluentId: string;
  defaultRouteId: string;
  defaultRate: string;
}

export type FormErrors = Record<string, string>;

export const emptyRegimenValues: RegimenFormValues = {
  code: "", name: "", marker: false, flag: false, cycleCheck: false,
  autoMode: false, drugAlert: false, appointmentAlert: false, counselAlert: false,
};

export const emptyGroupValues: RegimenGroupFormValues = { note: "", cycleDay: "", cycleCount: "" };

export function emptyItemValues(groupId: number): RegimenItemFormValues {
  return {
    regimenGroupId: String(groupId), drugId: "", doseText: "", unitText: "",
    routeText: "", details: "", itemGroup: "", duration: "", startDay: "",
    orderingNo: "", defaultDiluentId: "", defaultRouteId: "", defaultRate: "",
  };
}

export function regimenToValues(regimen: RegimenDetail): RegimenFormValues {
  return {
    code: regimen.code, name: regimen.name, marker: regimen.marker, flag: regimen.flag,
    cycleCheck: regimen.cycleCheck, autoMode: regimen.autoMode, drugAlert: regimen.drugAlert,
    appointmentAlert: regimen.appointmentAlert, counselAlert: regimen.counselAlert,
  };
}

export function groupToValues(group: RegimenGroupDetail): RegimenGroupFormValues {
  return { note: group.note ?? "", cycleDay: text(group.cycleDay), cycleCount: text(group.cycleCount) };
}

export function itemToValues(item: RegimenItemDetail): RegimenItemFormValues {
  return {
    regimenGroupId: String(item.regimenGroupId), drugId: String(item.drugId),
    doseText: item.doseText ?? "", unitText: item.unitText ?? "", routeText: item.routeText ?? "",
    details: item.details ?? "", itemGroup: item.itemGroup ?? "", duration: item.duration ?? "",
    startDay: text(item.startDay), orderingNo: text(item.orderingNo),
    defaultDiluentId: text(item.defaultDiluentId), defaultRouteId: text(item.defaultRouteId),
    defaultRate: item.defaultRate ?? "",
  };
}

export function validateRegimen(values: RegimenFormValues): FormErrors {
  const errors: FormErrors = {};
  if (!values.code.trim()) errors.code = "Regimen code is required.";
  if (!values.name.trim()) errors.name = "Regimen name is required.";
  if (values.code.trim().length > 64) errors.code = "Use 64 characters or fewer.";
  if (values.name.trim().length > 255) errors.name = "Use 255 characters or fewer.";
  return errors;
}

export function validateGroup(values: RegimenGroupFormValues): FormErrors {
  const errors: FormErrors = {};
  validateOptionalInteger(values.cycleDay, "cycleDay", errors);
  validateOptionalInteger(values.cycleCount, "cycleCount", errors);
  if (values.note.trim().length > 255) errors.note = "Use 255 characters or fewer.";
  return errors;
}

export function validateItem(values: RegimenItemFormValues): FormErrors {
  const errors: FormErrors = {};
  if (!values.drugId) errors.drugId = "Drug is required.";
  if (!values.regimenGroupId) errors.regimenGroupId = "Treatment group is required.";
  for (const [field, value] of [["duration", values.duration], ["startDay", values.startDay], ["orderingNo", values.orderingNo]] as const) {
    validateOptionalInteger(value, field, errors);
  }
  if (values.itemGroup.trim().length > 2) errors.itemGroup = "Use 2 characters or fewer.";
  const numericDose = Number(values.doseText.trim());
  if (values.doseText.trim() && !Number.isNaN(numericDose) && (!Number.isFinite(numericDose) || numericDose < 0)) {
    errors.doseText = "Numeric dose values cannot be negative.";
  }
  return errors;
}

export function toRegimenInput(values: RegimenFormValues): RegimenInput {
  return { ...values, code: values.code.trim(), name: values.name.trim() };
}

export function toGroupInput(values: RegimenGroupFormValues): RegimenGroupInput {
  return { note: optional(values.note), cycleDay: integer(values.cycleDay), cycleCount: integer(values.cycleCount) };
}

export function toItemInput(values: RegimenItemFormValues): RegimenItemInput {
  return {
    regimenGroupId: Number(values.regimenGroupId), drugId: Number(values.drugId),
    doseText: optional(values.doseText), unitText: optional(values.unitText),
    routeText: optional(values.routeText), details: optional(values.details),
    itemGroup: optional(values.itemGroup), duration: integer(values.duration),
    startDay: integer(values.startDay), orderingNo: integer(values.orderingNo),
    defaultDiluentId: integer(values.defaultDiluentId), defaultRouteId: integer(values.defaultRouteId),
    defaultRate: optional(values.defaultRate),
  };
}

function validateOptionalInteger(value: string, field: string, errors: FormErrors) {
  if (!value.trim()) return;
  const parsed = Number(value);
  if (!Number.isInteger(parsed) || parsed < 0) errors[field] = "Enter a whole number of zero or greater.";
}

function optional(value: string): string | null {
  return value.trim() || null;
}

function integer(value: string): number | null {
  return value.trim() ? Number(value) : null;
}

function text(value: number | null): string {
  return value === null ? "" : String(value);
}
