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

export function drugNameWithPackageSize(drug: {
  name: string;
  unit: string | null;
  dosePerPack: number | null;
  volumePerPackMl: number | null;
}): string {
  const size: string[] = [];
  if (drug.dosePerPack !== null && drug.dosePerPack > 0) {
    const unit = drug.unit?.trim().replace(/\.$/, "") || "unit not set";
    size.push(`${drug.dosePerPack} ${unit}`);
  }
  if (drug.volumePerPackMl !== null && drug.volumePerPackMl > 0) {
    size.push(`${drug.volumePerPackMl} mL`);
  }
  return size.length ? `${drug.name} (${size.join(" / ")})` : drug.name;
}
