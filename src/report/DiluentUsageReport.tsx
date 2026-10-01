import { useEffect, useState, type ReactNode } from "react";
import { commandError, getDiluentUsageReport } from "../api/commands";
import { BuddhistDateInput } from "../components/BuddhistDateInput";
import { currentBangkokDateTimeValue, displayDate } from "../shared/dateTime";
import type { DiluentUsageReport as ReportData } from "../types/report";

export function DiluentUsageReport({ navigation }: { navigation?: ReactNode }) {
  const [date, setDate] = useState(() => currentBangkokDateTimeValue().slice(0, 10));
  const [refresh, setRefresh] = useState(0);
  const [state, setState] = useState<{ loading: boolean; report: ReportData | null; error: string | null }>({ loading: true, report: null, error: null });

  useEffect(() => {
    let active = true;
    setState({ loading: true, report: null, error: null });
    void getDiluentUsageReport(date).then((report) => {
      if (active) setState({ loading: false, report, error: null });
    }).catch((error: unknown) => {
      if (active) setState({ loading: false, report: null, error: commandError(error).message ?? "ไม่สามารถโหลดรายงาน Diluent ได้" });
    });
    return () => { active = false; };
  }, [date, refresh]);

  const report = !state.loading && state.report?.preparationDate === date ? state.report : null;
  return <section className="workspace report-workspace" aria-labelledby="diluent-usage-heading">
    <div className="page-heading"><div><p className="eyebrow">Reports</p><h1 id="diluent-usage-heading">จำนวนใช้ Diluent รายวัน</h1><p className="page-summary">สรุปจำนวนขวดแยกตามสารละลายและปริมาตร · เฉพาะ Checked ที่ไม่ยกเลิก</p></div></div>
    {navigation}
    <div className="surface report-filter-card diluent-usage-controls">
      <label className="compact-filter">วันที่เตรียมยา<BuddhistDateInput value={date} onChange={setDate} /></label>
      <button type="button" className="button button--secondary" onClick={() => { setState({ loading: true, report: null, error: null }); setRefresh((value) => value + 1); }}>โหลดใหม่</button>
      <button type="button" className="button button--primary" disabled={!report?.rows.length} onClick={() => window.print()}>พิมพ์รายงาน Diluent</button>
    </div>
    {state.loading ? <div className="state-panel" aria-busy="true">กำลังโหลดรายงาน Diluent…</div> : state.error ? <div className="state-panel state-panel--error" role="alert">{state.error}</div> : report && (report.rows.length === 0 ? <div className="state-panel">ไม่มีรายการ Checked สำหรับวันที่เลือก</div> : <DiluentUsageDocument report={report} />)}
  </section>;
}

export function DiluentUsageDocument({ report }: { report: ReportData }) {
  const hasMissing = report.rows.some((row) => row.diluentName === null || row.volumeMl === null);
  return <article className="diluent-usage-print-root">
    <header><h2>จำนวนใช้ Diluent รายวัน</h2><p>วันที่เตรียมยา: {displayDate(report.preparationDate)}</p><p>เฉพาะรายการ Checked ที่ไม่ยกเลิก</p></header>
    <table><thead><tr><th scope="col">Diluent</th><th scope="col">ปริมาตร (mL)</th><th scope="col">จำนวน (ขวด)</th></tr></thead>
      <tbody>{report.rows.map((row) => <tr key={JSON.stringify([row.diluentName, row.volumeMl])}><td>{row.diluentName ?? "ไม่ระบุสารละลาย"}</td><td>{row.volumeMl ?? "ไม่ระบุปริมาตร"}</td><td>{row.bottleCount.toLocaleString("th-TH")}</td></tr>)}</tbody>
      <tfoot><tr><th scope="row" colSpan={2}>รวมทั้งหมด</th><td>{report.totalBottles.toLocaleString("th-TH")} ขวด</td></tr></tfoot>
    </table>
    <p className="diluent-usage-dose-count">จำนวนยาที่เตรียม {report.totalDoses.toLocaleString("th-TH")} Dose</p>
    <p className="diluent-usage-note">นับตามจำนวนขวดที่บันทึกในการเตรียมยา ไม่ใช่จำนวนขวดที่ตัดจากคลัง</p>
    {hasMissing && <p className="diluent-usage-note">มีรายการไม่ระบุสารละลายหรือปริมาตร แสดงแยกไว้และรวมจำนวนขวดในยอดทั้งหมด</p>}
  </article>;
}
