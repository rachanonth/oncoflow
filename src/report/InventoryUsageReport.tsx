import { useEffect, useMemo, useState } from "react";
import type { ReactNode } from "react";

import { commandError, getInventoryUsageReport } from "../api/commands";
import { BuddhistDateInput } from "../components/BuddhistDateInput";
import { displayDate } from "../shared/dateTime";
import type { InventoryUsageReport as ReportData, InventoryUsageReportRow, ReportInterval } from "../types/report";
import { defaultReportRange, formatReportPeriod } from "./PreparationCountReport";

export function InventoryUsageReport({ navigation }: { navigation?: ReactNode }) {
  const [interval, setInterval] = useState<ReportInterval>("daily");
  const [range, setRange] = useState(() => defaultReportRange("daily"));
  const [state, setState] = useState<{ loading: boolean; report: ReportData | null; error: string | null }>({ loading: true, report: null, error: null });
  const request = useMemo(() => ({ interval, dateFrom: range.dateFrom, dateTo: range.dateTo }), [interval, range]);

  useEffect(() => {
    let active = true;
    setState((current) => ({ ...current, loading: true, error: null }));
    const timeout = window.setTimeout(() => {
      void getInventoryUsageReport(request).then((report) => {
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
  const periods = groupInventoryUsagePeriods(rows);
  return <section className="workspace report-workspace" aria-labelledby="inventory-usage-heading">
    <div className="page-heading"><div><p className="eyebrow">Reports</p><h1 id="inventory-usage-heading">การใช้ยาและตัด Stock</h1><p className="page-summary">เปรียบเทียบจำนวนตำรับ ขวดที่เตรียม และ vial/ampoule ต้นทางที่ตัดจาก Stock จริง</p></div></div>
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
        <ReportSummary label="จำนวนตำรับ" value={state.loading ? "…" : formatNumber(state.report?.totalPrescriptions ?? 0)} suffix="ตำรับ" />
        <ReportSummary label="ขวดที่เตรียม" value={state.loading ? "…" : formatNumber(state.report?.totalPreparedBottles ?? 0)} suffix="ขวด" />
        <ReportSummary label="ตัดจาก Stock" value={state.loading ? "…" : formatNumber(state.report?.totalIssuedSourceContainers ?? 0)} suffix="ภาชนะต้นทาง" />
        <ReportSummary label="รายการยา" value={state.loading ? "…" : formatNumber(state.report?.drugCount ?? 0)} suffix="drugs" />
      </div>

      <div className="surface list-card report-table-card inventory-usage-card">
        <div className="report-table-heading"><div><h2>รายละเอียดการใช้ยา</h2><p>{displayDate(range.dateFrom)} – {displayDate(range.dateTo)} · {periods.length} ช่วงเวลา</p></div><span className="inventory-usage-legend">ยอด Stock เป็นยอดปัจจุบัน</span></div>
        {state.loading ? <div className="list-skeleton" aria-label="กำลังโหลดรายงาน">{[1, 2, 3].map((value) => <div className="skeleton-row" key={value}><span/><span/><span/><span/></div>)}</div> : rows.length === 0 ? <div className="state-panel"><span className="state-icon">▦</span><h2>ไม่มีข้อมูลการใช้ยา</h2><p>ไม่พบรายการที่เตรียมแล้วในช่วงวันที่ที่เลือก</p></div> : <InventoryUsageTable interval={interval} rows={rows} />}
        <div className="list-footer"><span>ตัด Stock เมื่อรายการผ่านการ Verify และบันทึก posting สำเร็จ</span><span>ยอดคงเหลือคำนวณจาก inventory ledger</span></div>
      </div>
    </>}
  </section>;
}

function ReportSummary({ label, value, suffix }: { label: string; value: string; suffix: string }) {
  return <article className="surface report-summary"><span>{label}</span><div><strong>{value}</strong><small>{suffix}</small></div></article>;
}

export interface InventoryUsagePeriod {
  periodStart: string;
  prescriptionCount: number;
  preparedBottleCount: number;
  issuedSourceContainerCount: number;
  rows: InventoryUsageReportRow[];
}

export function groupInventoryUsagePeriods(rows: InventoryUsageReportRow[]): InventoryUsagePeriod[] {
  const periods = new Map<string, InventoryUsageReportRow[]>();
  for (const row of rows) periods.set(row.periodStart, [...(periods.get(row.periodStart) ?? []), row]);
  return [...periods.entries()].sort(([left], [right]) => left.localeCompare(right)).map(([periodStart, periodRows]) => ({
    periodStart,
    rows: [...periodRows].sort((left, right) => left.drugName.localeCompare(right.drugName, "th")),
    prescriptionCount: periodRows.reduce((total, row) => total + row.prescriptionCount, 0),
    preparedBottleCount: periodRows.reduce((total, row) => total + row.preparedBottleCount, 0),
    issuedSourceContainerCount: periodRows.reduce((total, row) => total + row.issuedSourceContainerCount, 0),
  }));
}

export function InventoryUsageTable({ interval, rows }: { interval: ReportInterval; rows: InventoryUsageReportRow[] }) {
  const periods = groupInventoryUsagePeriods(rows);
  const totals = periods.reduce((current, period) => ({ prescriptions: current.prescriptions + period.prescriptionCount, bottles: current.bottles + period.preparedBottleCount, issued: current.issued + period.issuedSourceContainerCount }), { prescriptions: 0, bottles: 0, issued: 0 });
  return <div className="table-scroll"><table className="patient-table inventory-usage-table"><thead><tr><th>{periodColumnLabel(interval)}</th><th>ยา</th><th>จำนวนที่เตรียม</th><th>ตัด Stock จริง</th><th>คงเหลือปัจจุบัน</th></tr></thead><tbody>{periods.flatMap((period) => period.rows.map((row, index) => <tr key={`${period.periodStart}:${row.drugId}`} className={index === 0 ? "report-period-start" : undefined}>{index === 0 && <td rowSpan={period.rows.length}><div className="report-period-cell"><span className="report-period">{formatReportPeriod(period.periodStart, interval)}</span><small>{period.prescriptionCount.toLocaleString("th-TH")} ตำรับ / {period.preparedBottleCount.toLocaleString("th-TH")} ขวด</small><small>ตัด Stock {formatNumber(period.issuedSourceContainerCount)} ภาชนะ</small></div></td>}<td><span className="inventory-usage-code">{row.drugCode}</span><strong>{row.drugName}</strong><InventoryExceptions row={row} /></td><td><div className="report-quantity"><strong>{formatNumber(row.prescriptionCount)}</strong><span>ตำรับ</span><b aria-hidden="true">/</b><strong>{formatNumber(row.preparedBottleCount)}</strong><span>ขวด</span></div></td><td><div className="inventory-issued"><strong>{formatNumber(row.issuedSourceContainerCount)}</strong><span>{row.sourcePackage}</span></div></td><td><InventoryBalance row={row} /></td></tr>))}</tbody><tfoot><tr><th colSpan={2} scope="row">รวมทั้งหมด</th><td><div className="report-quantity report-quantity--total"><strong>{formatNumber(totals.prescriptions)}</strong><span>ตำรับ</span><b aria-hidden="true">/</b><strong>{formatNumber(totals.bottles)}</strong><span>ขวด</span></div></td><td><div className="inventory-issued inventory-issued--total"><strong>{formatNumber(totals.issued)}</strong><span>ภาชนะ</span></div></td><td /></tr></tfoot></table></div>;
}

function InventoryExceptions({ row }: { row: InventoryUsageReportRow }) {
  const items = [
    row.awaitingVerificationCount > 0 ? { tone: "waiting", label: `รอตรวจ ${row.awaitingVerificationCount}` } : null,
    row.manualReconciliationCount > 0 ? { tone: "manual", label: `กระทบยอดเอง ${row.manualReconciliationCount}` } : null,
    row.trackingDisabledCount > 0 ? { tone: "muted", label: `ไม่ติดตาม Stock ${row.trackingDisabledCount}` } : null,
    row.unrecordedInventoryCount > 0 ? { tone: "manual", label: `ไม่มีประวัติตัด ${row.unrecordedInventoryCount}` } : null,
  ].filter((item): item is { tone: string; label: string } => item !== null);
  return items.length > 0 ? <div className="inventory-exceptions">{items.map((item) => <span className={`inventory-exception inventory-exception--${item.tone}`} key={item.label}>{item.label}</span>)}</div> : null;
}

function InventoryBalance({ row }: { row: InventoryUsageReportRow }) {
  return <div className="inventory-balance"><strong>{row.currentStock === null ? "—" : formatNumber(row.currentStock)}</strong><span>{row.sourcePackage}</span><small className={`stock-state stock-state--${row.stockState}`}>{stockStateLabel(row.stockState)}</small>{row.minimumStock !== null && <em>Min {formatNumber(row.minimumStock)}</em>}</div>;
}

function stockStateLabel(state: InventoryUsageReportRow["stockState"]): string {
  return { untracked: "ไม่ติดตาม", unknown: "ไม่ทราบยอด", shortage: "ติดลบ", out: "หมด", low: "ต่ำ", normal: "ปกติ" }[state];
}

function intervalLabel(interval: ReportInterval): string { return interval === "daily" ? "Daily" : interval === "weekly" ? "Weekly" : "Monthly"; }
function periodColumnLabel(interval: ReportInterval): string { return interval === "daily" ? "วันที่" : interval === "weekly" ? "สัปดาห์" : "เดือน"; }
function defaultRangeDescription(interval: ReportInterval): string { return interval === "daily" ? "ค่าเริ่มต้น: เดือนปัจจุบัน" : interval === "weekly" ? "ค่าเริ่มต้น: 12 สัปดาห์ล่าสุด" : "ค่าเริ่มต้น: ปีปัจจุบัน"; }
function formatNumber(value: number): string { return value.toLocaleString("th-TH", { maximumFractionDigits: 2 }); }
