import { currentBangkokDateTimeValue } from "../shared/dateTime";

/** Returns completed years at the supplied Bangkok calendar date. */
export function calculateAgeYears(
  birthDate: string | null,
  asOfDate = currentBangkokDateTimeValue().slice(0, 10),
): number | null {
  const birth = parseIsoDate(birthDate);
  const asOf = parseIsoDate(asOfDate);
  if (!birth || !asOf || compareDate(birth, asOf) > 0) return null;

  let years = asOf.year - birth.year;
  if (
    asOf.month < birth.month ||
    (asOf.month === birth.month && asOf.day < birth.day)
  ) {
    years -= 1;
  }
  return years;
}

function parseIsoDate(value: string | null) {
  if (!value || !/^\d{4}-\d{2}-\d{2}$/.test(value)) return null;
  const [year, month, day] = value.split("-").map(Number);
  const probe = new Date(Date.UTC(year, month - 1, day));
  if (
    probe.getUTCFullYear() !== year ||
    probe.getUTCMonth() !== month - 1 ||
    probe.getUTCDate() !== day
  ) {
    return null;
  }
  return { year, month, day };
}

function compareDate(
  left: { year: number; month: number; day: number },
  right: { year: number; month: number; day: number },
) {
  return left.year - right.year || left.month - right.month || left.day - right.day;
}
