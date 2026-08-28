import { useEffect, useMemo, useState } from "react";
import type { ReactNode } from "react";

import { commandError, getPreparationCountReport } from "../api/commands";
import { BuddhistDateInput } from "../components/BuddhistDateInput";
import { currentBangkokDateTimeValue, displayDate } from "../shared/dateTime";
import type { PreparationCountReport as ReportData, PreparationCountReportRow, ReportInterval } from "../types/report";

export type ReportGroupBy = "pharmacist" | "drug";

export interface GroupedPreparationPeriod {
  periodStart: string;
  totalPrescriptions: number;
  totalBottles: number;
  items: Array<{ key: string; label: string; prescriptionCount: number; bottleCount: number }>;
}

export function PreparationCountReport({ navigation }: { navigation?: ReactNode }) {
  const [interval, setInterval] = useState<ReportInterval>("daily");
  const [groupBy, setGroupBy] = useState<ReportGroupBy>("pharmacist");
  const [range, setRange] = useState(() => defaultReportRange("daily"));
  const [state, setState] = useState<{ loading: boolean; report: ReportData | null; error: string | null }>({ loading: true, report: null, error: null });
  const request = useMemo(() => ({ interval, dateFrom: range.dateFrom, dateTo: range.dateTo }), [interval, range]);

  useEffect(() => {
    let active = true;
    setState((current) => ({ ...current, loading: true, error: null }));
    const timeout = window.setTimeout(() => {
      void getPreparationCountReport(request).then((report) => {
        if (active) setState({ loading: false, report, error: null });
      }).catch((error: unknown) => {
        if (active) setState({ loading: false, report: null, error: commandError(error).message ?? "ไม่สามารถโหลดรายงานได้" });
      });
    }, 150);
    return () => { active = false; window.clearTimeout(timeout); };
  }, [request]);

  function selectInterval(next: ReportInterval) {
    setInterval(next);
    setRange(defaultReportRange(next));
  }

  const rows = state.report?.rows ?? [];
  const drugCount = new Set(rows.map((row) => row.drugId)).size;
  const preparerCount = new Set(rows.map((row) => row.preparerUserId ?? `name:${row.preparerName}`)).size;
  const groupedPeriods = aggregateReportRows(rows, groupBy);

  return <section className="workspace report-workspace" aria-labelledby="preparation-count-heading">
    <div className="page-heading"><div><p className="eyebrow">Reports</p><h1 id="preparation-count-heading">จำนวนการเตรียมยาสะสม</h1><p className="page-summary">สรุปจำนวนตำรับและจำนวนขวดที่เตรียม แยกตามยาและเภสัชกรผู้เตรียม</p></div></div>
    {navigation}

    <div className="surface report-filter-card">
      <div className="report-interval-filter" role="group" aria-label="ความละเอียดของรายงาน">
        {(["daily", "weekly", "monthly"] as const).map((value) => <button key={value} type="button" className={interval === value ? "is-active" : ""} aria-pressed={interval === value} onClick={() => selectInterval(value)}>{intervalLabel(value)}</button>)}
      </div>
      <div className="report-date-range">
        <label className="compact-filter">ตั้งแต่<BuddhistDateInput value={range.dateFrom} onChange={(dateFrom) => setRange((current) => ({ ...current, dateFrom }))} invalid={Boolean(range.dateFrom && range.dateTo && range.dateFrom > range.dateTo)} /></label>
        <span aria-hidden="true">–</span>
        <label className="compact-filter">ถึง<BuddhistDateInput value={range.dateTo} onChange={(dateTo) => setRange((current) => ({ ...current, dateTo }))} invalid={Boolean(range.dateFrom && range.dateTo && range.dateFrom > range.dateTo)} /></label>
      </div>
      <p>{defaultRangeDescription(interval)}</p>
    </div>

    {state.error ? <div className="surface state-panel state-panel--error" role="alert"><span className="state-icon">!</span><h2>โหลดรายงานไม่สำเร็จ</h2><p>{state.error}</p></div> : <>
      <div className="report-summary-grid" aria-busy={state.loading}>
        <ReportSummary label="จำนวนตำรับ" value={state.loading ? "…" : String(state.report?.totalPrescriptions ?? 0)} suffix="ตำรับ" />
        <ReportSummary label="จำนวนขวดที่เตรียม" value={state.loading ? "…" : String(state.report?.totalBottles ?? 0)} suffix="ขวด" />
        <ReportSummary label="รายการยา" value={state.loading ? "…" : String(drugCount)} suffix="drugs" />
        <ReportSummary label="เภสัชกรผู้เตรียม" value={state.loading ? "…" : String(preparerCount)} suffix="people" />
      </div>

      <div className="surface list-card report-table-card">
        <div className="report-table-heading"><div><h2>รายละเอียดการเตรียมยา</h2><p>{displayDate(range.dateFrom)} – {displayDate(range.dateTo)} · {groupedPeriods.length} ช่วงเวลา</p></div><div className="report-group-control"><span>Group by</span><div className="report-group-filter" role="group" aria-label="จัดกลุ่มรายงานตาม"><button type="button" className={groupBy === "pharmacist" ? "is-active" : ""} aria-pressed={groupBy === "pharmacist"} onClick={() => setGroupBy("pharmacist")}>Pharmacist</button><button type="button" className={groupBy === "drug" ? "is-active" : ""} aria-pressed={groupBy === "drug"} onClick={() => setGroupBy("drug")}>Drug</button></div></div></div>
        {state.loading ? <div className="list-skeleton" aria-label="กำลังโหลดรายงาน">{[1, 2, 3].map((value) => <div className="skeleton-row" key={value}><span/><span/><span/></div>)}</div> : rows.length === 0 ? <div className="state-panel"><span className="state-icon">▥</span><h2>ไม่มีข้อมูลการเตรียมยา</h2><p>ไม่พบรายการที่บันทึกว่าเตรียมแล้วในช่วงวันที่ที่เลือก</p></div> : <PreparationCountTable interval={interval} rows={rows} groupBy={groupBy} />}
        <div className="list-footer"><span>นับ 1 preparation line เป็น 1 ตำรับ · จำนวนขวดจาก final containers</span><span>เฉพาะสถานะ Prepared และ Verified</span></div>
      </div>
    </>}
  </section>;
}

function ReportSummary({ label, value, suffix }: { label: string; value: string; suffix: string }) {
  return <article className="surface report-summary"><span>{label}</span><div><strong>{value}</strong><small>{suffix}</small></div></article>;
}

export function PreparationCountTable({ interval, rows, groupBy }: { interval: ReportInterval; rows: PreparationCountReportRow[]; groupBy: ReportGroupBy }) {
  const periods = aggregateReportRows(rows, groupBy);
  const totalPrescriptions = periods.reduce((total, period) => total + period.totalPrescriptions, 0);
  const totalBottles = periods.reduce((total, period) => total + period.totalBottles, 0);
  return <div className="table-scroll"><table className="patient-table report-table"><thead><tr><th>{periodColumnLabel(interval)}</th><th>{groupBy === "pharmacist" ? "เภสัชกรผู้เตรียม" : "ยา"}</th><th>จำนวนที่เตรียม</th></tr></thead><tbody>{periods.flatMap((period) => period.items.map((item, index) => <tr key={`${period.periodStart}:${item.key}`} className={index === 0 ? "report-period-start" : undefined}>{index === 0 && <td rowSpan={period.items.length}><div className="report-period-cell"><span className="report-period">{formatReportPeriod(period.periodStart, interval)}</span><small>{formatPreparationQuantity(period.totalPrescriptions, period.totalBottles)}</small></div></td>}<td><strong>{item.label}</strong></td><td><PreparationQuantity prescriptions={item.prescriptionCount} bottles={item.bottleCount} /></td></tr>))}</tbody><tfoot><tr><th colSpan={2} scope="row">รวมทั้งหมด</th><td><PreparationQuantity prescriptions={totalPrescriptions} bottles={totalBottles} grandTotal /></td></tr></tfoot></table></div>;
}

function PreparationQuantity({ prescriptions, bottles, grandTotal = false }: { prescriptions: number; bottles: number; grandTotal?: boolean }) {
  return <div className={grandTotal ? "report-quantity report-quantity--total" : "report-quantity"}><strong>{prescriptions.toLocaleString("th-TH")}</strong><span>ตำรับ</span><b aria-hidden="true">/</b><strong>{bottles.toLocaleString("th-TH")}</strong><span>ขวด</span></div>;
}

function formatPreparationQuantity(prescriptions: number, bottles: number): string {
  return `${prescriptions.toLocaleString("th-TH")} ตำรับ / ${bottles.toLocaleString("th-TH")} ขวด`;
}

export function aggregateReportRows(rows: PreparationCountReportRow[], groupBy: ReportGroupBy): GroupedPreparationPeriod[] {
  const periods = new Map<string, Map<string, { key: string; label: string; prescriptionCount: number; bottleCount: number }>>();
  for (const row of rows) {
    const key = groupBy === "drug" ? `drug:${row.drugId}` : row.preparerUserId === null ? `pharmacist-name:${row.preparerName}` : `pharmacist:${row.preparerUserId}`;
    const label = groupBy === "drug" ? row.drugName : row.preparerName;
    const items = periods.get(row.periodStart) ?? new Map();
    const current = items.get(key);
    items.set(key, { key, label, prescriptionCount: (current?.prescriptionCount ?? 0) + row.prescriptionCount, bottleCount: (current?.bottleCount ?? 0) + row.bottleCount });
    periods.set(row.periodStart, items);
  }
  return [...periods.entries()].sort(([left], [right]) => left.localeCompare(right)).map(([periodStart, values]) => {
    const items = [...values.values()].sort((left, right) => right.prescriptionCount - left.prescriptionCount || left.label.localeCompare(right.label, "th"));
    return { periodStart, items, totalPrescriptions: items.reduce((total, item) => total + item.prescriptionCount, 0), totalBottles: items.reduce((total, item) => total + item.bottleCount, 0) };
  });
}

export function defaultReportRange(interval: ReportInterval, today = currentBangkokDateTimeValue().slice(0, 10)): { dateFrom: string; dateTo: string } {
  const [year, month, day] = today.split("-").map(Number);
  const date = new Date(Date.UTC(year, month - 1, day));
  if (interval === "daily") return { dateFrom: isoDate(new Date(Date.UTC(year, month - 1, 1))), dateTo: isoDate(new Date(Date.UTC(year, month, 0))) };
  if (interval === "monthly") return { dateFrom: `${year}-01-01`, dateTo: `${year}-12-31` };
  const mondayOffset = (date.getUTCDay() + 6) % 7;
  const monday = addDays(date, -mondayOffset);
  return { dateFrom: isoDate(addDays(monday, -77)), dateTo: isoDate(addDays(monday, 6)) };
}

export function formatReportPeriod(periodStart: string, interval: ReportInterval): string {
  if (interval === "daily") return displayDate(periodStart);
  if (interval === "weekly") return `${displayDate(periodStart)} – ${displayDate(isoDate(addDays(parseIsoDate(periodStart), 6)))}`;
  const [year, month] = periodStart.split("-").map(Number);
  const monthName = ["มกราคม", "กุมภาพันธ์", "มีนาคม", "เมษายน", "พฤษภาคม", "มิถุนายน", "กรกฎาคม", "สิงหาคม", "กันยายน", "ตุลาคม", "พฤศจิกายน", "ธันวาคม"][month - 1];
  return monthName && year ? `${monthName} ${year + 543}` : periodStart;
}

function intervalLabel(interval: ReportInterval): string { return interval === "daily" ? "Daily" : interval === "weekly" ? "Weekly" : "Monthly"; }
function periodColumnLabel(interval: ReportInterval): string { return interval === "daily" ? "วันที่" : interval === "weekly" ? "สัปดาห์" : "เดือน"; }
function defaultRangeDescription(interval: ReportInterval): string { return interval === "daily" ? "ค่าเริ่มต้น: เดือนปัจจุบัน" : interval === "weekly" ? "ค่าเริ่มต้น: 12 สัปดาห์ล่าสุด" : "ค่าเริ่มต้น: ปีปัจจุบัน"; }
function parseIsoDate(value: string): Date { const [year, month, day] = value.split("-").map(Number); return new Date(Date.UTC(year, month - 1, day)); }
function addDays(value: Date, amount: number): Date { const next = new Date(value); next.setUTCDate(next.getUTCDate() + amount); return next; }
function isoDate(value: Date): string { return `${value.getUTCFullYear()}-${String(value.getUTCMonth() + 1).padStart(2, "0")}-${String(value.getUTCDate()).padStart(2, "0")}`; }
