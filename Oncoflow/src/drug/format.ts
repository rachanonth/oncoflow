export function displayDrugValue(value: string | number | null): string {
  return value === null || value === "" ? "—" : String(value);
}

export function displayFlag(value: boolean | null): string {
  if (value === null) return "Not recorded";
  return value ? "Enabled" : "Disabled";
}

export function numberWithUnit(value: number | null, unit: string): string | null {
  return value === null ? null : `${value} ${unit}`;
}
