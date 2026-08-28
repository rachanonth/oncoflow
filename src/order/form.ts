import type { OrderDetail, OrderInput, OrderItemDetail, OrderItemInput } from "../types/order";
import { currentBangkokDateTimeValue } from "../shared/dateTime";

export interface OrderFormValues {
  patientId: string; wardId: string; doctorId: string; regimenId: string;
  note: string; orderTime: string; orderType: string; appointmentFlag: boolean; assignedPreparerUserId: string;
}
export interface OrderItemFormValues {
  drugId: string; diluentId: string; diluentVolumeMl: string; routeId: string; startDate: string;
  stopDate: string; doseText: string; scheduleTime: string; numberOfDrug: string;
  missing: boolean; rateValue: string; rateUnit: RateUnit; rateOriginal: string; rateTouched: boolean;
}
export type RateUnit = "minute" | "hour";
export type FormErrors = Record<string, string>;

export function acquireSubmissionLock(lock: { current: boolean }): boolean {
  if (lock.current) return false;
  lock.current = true;
  return true;
}

export function emptyOrderValues(patientId?: number): OrderFormValues {
  return { patientId: patientId ? String(patientId) : "", wardId: "", doctorId: "", regimenId: "", note: "", orderTime: localDateTime(), orderType: "", appointmentFlag: false, assignedPreparerUserId: "" };
}
export function emptyOrderItemValues(now = new Date()): OrderItemFormValues {
  const today = currentBangkokDateTimeValue(now).slice(0, 10);
  return { drugId: "", diluentId: "", diluentVolumeMl: "", routeId: "", startDate: today, stopDate: today, doseText: "", scheduleTime: "", numberOfDrug: "", missing: false, rateValue: "", rateUnit: "minute", rateOriginal: "", rateTouched: false };
}

export function orderToValues(order: OrderDetail): OrderFormValues {
  return { patientId: String(order.patientId), wardId: text(order.wardId), doctorId: text(order.doctorId), regimenId: text(order.regimenId), note: order.note ?? "", orderTime: toDateTimeLocal(order.orderTime), orderType: order.orderType ?? "", appointmentFlag: order.appointmentFlag, assignedPreparerUserId: text(order.assignedPreparerUserId) };
}
export function itemToValues(item: OrderItemDetail): OrderItemFormValues {
  return { drugId: String(item.drugId), diluentId: text(item.diluentId), diluentVolumeMl: text(item.diluentVolumeMl), routeId: text(item.routeId), startDate: item.startDate?.slice(0, 10) ?? "", stopDate: item.stopDate?.slice(0, 10) ?? "", doseText: item.doseText ?? "", scheduleTime: item.scheduleTime?.slice(0, 5) ?? "", numberOfDrug: text(item.numberOfDrug), missing: item.missing, ...rateValues(item.rate) };
}

export function validateOrder(values: OrderFormValues): FormErrors {
  const errors: FormErrors = {};
  if (!values.patientId) errors.patientId = "Patient is required.";
  if (!values.assignedPreparerUserId) errors.assignedPreparerUserId = "Preparation pharmacist is required.";
  if (values.orderType.trim().length > 2) errors.orderType = "Use 2 characters or fewer.";
  if (values.orderTime && Number.isNaN(Date.parse(values.orderTime))) errors.orderTime = "Enter a valid date and time.";
  return errors;
}
export function validateOrderItem(values: OrderItemFormValues): FormErrors {
  const errors: FormErrors = {};
  if (!values.drugId) errors.drugId = "Drug is required.";
  if (values.startDate && values.stopDate && values.stopDate < values.startDate) errors.stopDate = "Stop date cannot be before start date.";
  if (values.numberOfDrug.trim()) {
    const quantity = Number(values.numberOfDrug);
    if (!Number.isFinite(quantity) || quantity < 0) errors.numberOfDrug = "Enter zero or a positive number.";
  }
  if (values.diluentVolumeMl.trim()) {
    const volume = Number(values.diluentVolumeMl);
    if (!Number.isFinite(volume) || volume < 0) errors.diluentVolumeMl = "Enter zero or a positive volume.";
  }
  if (values.rateValue.trim()) {
    const rate = Number(values.rateValue);
    if (!Number.isFinite(rate) || rate < 0) errors.rateValue = "Enter zero or a positive rate.";
  }
  const numericDose = Number(values.doseText.trim());
  if (values.doseText.trim() && !Number.isNaN(numericDose) && (!Number.isFinite(numericDose) || numericDose < 0)) errors.doseText = "Numeric dose cannot be negative.";
  return errors;
}
export function toOrderInput(values: OrderFormValues): OrderInput {
  return { patientId: Number(values.patientId), wardId: id(values.wardId), doctorId: id(values.doctorId), regimenId: id(values.regimenId), note: optional(values.note), orderTime: optional(values.orderTime), orderType: optional(values.orderType), appointmentFlag: values.appointmentFlag, assignedPreparerUserId: id(values.assignedPreparerUserId) };
}
export function toOrderItemInput(values: OrderItemFormValues): OrderItemInput {
  return { drugId: Number(values.drugId), diluentId: id(values.diluentId), diluentVolumeMl: optionalNumber(values.diluentVolumeMl), routeId: id(values.routeId), startDate: optional(values.startDate), stopDate: optional(values.stopDate), doseText: optional(values.doseText), scheduleTime: optional(values.scheduleTime), numberOfDrug: optionalNumber(values.numberOfDrug), missing: values.missing, rate: values.rateTouched ? serializedRate(values.rateValue, values.rateUnit) : optional(values.rateOriginal) };
}

export function convertRateValue(value: string, from: RateUnit, to: RateUnit): string {
  if (from === to || !value.trim()) return value;
  const numeric = Number(value);
  if (!Number.isFinite(numeric)) return value;
  const converted = from === "minute" ? numeric / 60 : numeric * 60;
  return Number(converted.toFixed(6)).toString();
}

export function rateValues(value: string | null): Pick<OrderItemFormValues, "rateValue" | "rateUnit" | "rateOriginal" | "rateTouched"> {
  const original = value?.trim() ?? "";
  const match = /^(?:over\s+)?(\d+(?:\.\d*)?|\.\d+)\s*(minutes?|mins?|hours?|hrs?)$/i.exec(original);
  return {
    rateValue: match?.[1] ?? "",
    rateUnit: match?.[2].toLowerCase().startsWith("h") ? "hour" : "minute",
    rateOriginal: original,
    rateTouched: false,
  };
}

function serializedRate(value: string, unit: RateUnit): string | null {
  const trimmed = value.trim();
  return trimmed ? `${trimmed} ${unit === "minute" ? "min" : "hr"}` : null;
}
function optional(value: string): string | null { return value.trim() || null; }
function id(value: string): number | null { return value ? Number(value) : null; }
function optionalNumber(value: string): number | null { return value.trim() ? Number(value) : null; }
function text(value: number | null): string { return value === null ? "" : String(value); }
function localDateTime(): string { return currentBangkokDateTimeValue(); }
function toDateTimeLocal(value: string | null): string { return value ? value.replace(" ", "T").slice(0, 16) : ""; }
