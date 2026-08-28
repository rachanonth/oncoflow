import type { DrugDetail, DrugInput } from "../types/drug";
import { parseDuration } from "../shared/duration";

export type TriState = "" | "true" | "false";

export interface DrugFormValues {
  code: string;
  name: string;
  unitId: string;
  dosePerPack: string;
  volumePerPackMl: string;
  package: string;
  detail: string;
  price: string;
  theory: string;
  marker: boolean;
  defaultDiluentId: string;
  defaultRouteId: string;
  defaultRate: string;
  warning: string;
  storage: string;
  flag: boolean;
  expiryTime: string;
  expiryStorage: string;
  maxDose: string;
  maxDilutionAlert: TriState;
  maxDilutionHard: string;
  cumulativeAlert: TriState;
  cumulativeAlertHard: string;
  dilutionIncompatibility: string;
  inventoryCut: TriState;
  inventoryMin: string;
  inventoryMax: string;
  inventoryEnabled: boolean;
}

export type DrugFormErrors = Partial<Record<keyof DrugFormValues, string>>;

export const emptyDrugForm: DrugFormValues = {
  code: "",
  name: "",
  unitId: "",
  dosePerPack: "",
  volumePerPackMl: "",
  package: "",
  detail: "",
  price: "",
  theory: "",
  marker: false,
  defaultDiluentId: "",
  defaultRouteId: "",
  defaultRate: "",
  warning: "",
  storage: "",
  flag: false,
  expiryTime: "",
  expiryStorage: "",
  maxDose: "",
  maxDilutionAlert: "",
  maxDilutionHard: "",
  cumulativeAlert: "",
  cumulativeAlertHard: "",
  dilutionIncompatibility: "",
  inventoryCut: "",
  inventoryMin: "",
  inventoryMax: "",
  inventoryEnabled: false,
};

export function withSuggestedDrugCode(values: DrugFormValues, suggestedCode: string): DrugFormValues {
  return values.code ? values : { ...values, code: suggestedCode };
}

export function drugToForm(drug: DrugDetail): DrugFormValues {
  const text = (value: string | null) => value ?? "";
  const number = (value: number | null) => value?.toString() ?? "";
  const tri = (value: boolean | null): TriState =>
    value === null ? "" : value ? "true" : "false";
  return {
    code: drug.code,
    name: drug.name,
    unitId: number(drug.unitId),
    dosePerPack: number(drug.dosePerPack),
    volumePerPackMl: number(drug.volumePerPackMl),
    package: text(drug.package),
    detail: text(drug.detail),
    price: number(drug.price),
    theory: text(drug.theory),
    marker: drug.marker,
    defaultDiluentId: number(drug.defaultDiluentId),
    defaultRouteId: number(drug.defaultRouteId),
    defaultRate: text(drug.defaultRate),
    warning: text(drug.warning),
    storage: text(drug.storage),
    flag: drug.flag,
    expiryTime: text(drug.expiryTime),
    expiryStorage: text(drug.expiryStorage),
    maxDose: number(drug.maxDose),
    maxDilutionAlert: tri(drug.maxDilutionAlert),
    maxDilutionHard: number(drug.maxDilutionHard),
    cumulativeAlert: tri(drug.cumulativeAlert),
    cumulativeAlertHard: number(drug.cumulativeAlertHard),
    dilutionIncompatibility: text(drug.dilutionIncompatibility),
    inventoryCut: tri(drug.inventoryCut),
    inventoryMin: number(drug.inventoryMin),
    inventoryMax: number(drug.inventoryMax),
    inventoryEnabled: drug.inventoryEnabled,
  };
}

export function validateDrugForm(values: DrugFormValues): DrugFormErrors {
  const errors: DrugFormErrors = {};
  if (!values.code.trim()) errors.code = "Drug code is required.";
  if (!values.name.trim()) errors.name = "Drug name is required.";
  if (Array.from(values.code.trim()).length > 64) {
    errors.code = "Drug code must be 64 characters or fewer.";
  }
  if (Array.from(values.name.trim()).length > 255) {
    errors.name = "Drug name must be 255 characters or fewer.";
  }
  for (const [field, label] of [
    ["dosePerPack", "Dose per pack"],
    ["volumePerPackMl", "Volume per pack"],
    ["price", "Price"],
    ["maxDose", "Maximum dose"],
    ["maxDilutionHard", "Maximum dilution threshold"],
    ["cumulativeAlertHard", "Cumulative alert threshold"],
    ["inventoryMin", "Minimum inventory"],
    ["inventoryMax", "Maximum inventory"],
  ] as const) {
    validateNonNegative(values[field], field, label, errors);
  }
  validateDuration(values.defaultRate, "defaultRate", "Default rate", false, errors);
  validateDuration(values.expiryTime, "expiryTime", "Expiry time", true, errors);
  const minimum = optionalNumber(values.inventoryMin);
  const maximum = optionalNumber(values.inventoryMax);
  if (minimum !== null && maximum !== null && minimum > maximum) {
    errors.inventoryMax = "Maximum inventory must be at least the minimum.";
  }
  return errors;
}

export function formToDrugInput(values: DrugFormValues): DrugInput {
  const optional = (value: string) => value.trim() || null;
  const id = (value: string) => (value ? Number(value) : null);
  const tri = (value: TriState) => (value === "" ? null : value === "true");
  return {
    code: values.code.trim(),
    name: values.name.trim(),
    unitId: id(values.unitId),
    dosePerPack: optionalNumber(values.dosePerPack),
    volumePerPackMl: optionalNumber(values.volumePerPackMl),
    package: optional(values.package),
    detail: optional(values.detail),
    price: optionalNumber(values.price),
    theory: optional(values.theory),
    marker: values.marker,
    defaultDiluentId: id(values.defaultDiluentId),
    defaultRouteId: id(values.defaultRouteId),
    defaultRate: optional(values.defaultRate),
    warning: optional(values.warning),
    storage: optional(values.storage),
    flag: values.flag,
    expiryTime: optional(values.expiryTime),
    expiryStorage: optional(values.expiryStorage),
    maxDose: optionalNumber(values.maxDose),
    maxDilutionAlert: tri(values.maxDilutionAlert),
    maxDilutionHard: optionalNumber(values.maxDilutionHard),
    cumulativeAlert: tri(values.cumulativeAlert),
    cumulativeAlertHard: optionalNumber(values.cumulativeAlertHard),
    dilutionIncompatibility: optional(values.dilutionIncompatibility),
    inventoryCut: tri(values.inventoryCut),
    inventoryMin: optionalNumber(values.inventoryMin),
    inventoryMax: optionalNumber(values.inventoryMax),
    inventoryEnabled: values.inventoryEnabled,
  };
}

function optionalNumber(value: string): number | null {
  return value.trim() ? Number(value) : null;
}

function validateNonNegative(
  value: string,
  field: keyof DrugFormValues,
  label: string,
  errors: DrugFormErrors,
) {
  if (!value.trim()) return;
  const parsed = Number(value);
  if (!Number.isFinite(parsed) || parsed < 0) {
    errors[field] = `${label} must be zero or greater.`;
  }
}

function validateDuration(
  value: string,
  field: "defaultRate" | "expiryTime",
  label: string,
  allowClock: boolean,
  errors: DrugFormErrors,
) {
  const parsed = parseDuration(value, { allowClock });
  if (parsed.value !== null && Number(parsed.value) < 0) {
    errors[field] = `${label} must be zero or greater.`;
  }
}
