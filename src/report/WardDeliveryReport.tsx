import { useEffect, useState, type ReactNode } from "react";
import { flushSync } from "react-dom";

import { commandError, getWardDeliveryReport } from "../api/commands";
import { BuddhistDateInput } from "../components/BuddhistDateInput";
import { currentBangkokDateTimeValue, displayDate } from "../shared/dateTime";
import type { WardDeliveryReport as ReportData, WardDeliveryRow } from "../types/report";

export function groupWardDelivery(rows: WardDeliveryRow[]) {
  const wards = new Map<number | null, { id: number | null; name: string; count: number; patients: Map<number, { id: number; hn: string; name: string; items: WardDeliveryRow[] }> }>();
  for (const row of rows) {
    const ward = wards.get(row.wardId) ?? { id: row.wardId, name: row.wardName, count: 0, patients: new Map() };
    const patient = ward.patients.get(row.patientId) ?? { id: row.patientId, hn: row.patientHn, name: row.patientName, items: [] };
    patient.items.push(row);
    ward.patients.set(row.patientId, patient);
    ward.count += 1;
    wards.set(row.wardId, ward);
  }
  return [...wards.values()];
}

export function WardDeliveryDialog({ date, onClose }: { date: string; onClose: () => void }) {
  return <div className="ward-delivery-backdrop">
    <section className="ward-delivery-dialog" role="dialog" aria-modal="true" aria-labelledby="ward-delivery-heading">
      <div className="ward-delivery-dialog-close"><button type="button" className="button button--secondary" onClick={onClose} aria-label="ปิดใบส่งยา">ปิด</button></div>
      <WardDeliveryReport initialDate={date} />
    </section>
  </div>;
}

export function WardDeliveryReport({ navigation, initialDate }: { navigation?: ReactNode; initialDate?: string }) {
  const [date, setDate] = useState(() => initialDate ?? currentBangkokDateTimeValue().slice(0, 10));
  const [ward, setWard] = useState("all");
  const [refresh, setRefresh] = useState(0);
  const [state, setState] = useState<{ loading: boolean; report: ReportData | null; error: string | null }>({ loading: true, report: null, error: null });
  const [printedAt, setPrintedAt] = useState(() => new Date());

  useEffect(() => {
    let active = true;
    setState({ loading: true, report: null, error: null });
    void getWardDeliveryReport(date).then((report) => {
      if (active) { setState({ loading: false, report, error: null }); setPrintedAt(new Date()); }
    }).catch((error: unknown) => {
      if (active) setState({ loading: false, report: null, error: commandError(error).message ?? "ไม่สามารถโหลดใบส่งยาได้" });
    });
    return () => { active = false; };
  }, [date, refresh]);

  useEffect(() => {
    const updateTime = () => flushSync(() => setPrintedAt(new Date()));
    window.addEventListener("beforeprint", updateTime);
    return () => window.removeEventListener("beforeprint", updateTime);
  }, []);

  const currentReport = !state.loading && state.report?.preparationDate === date ? state.report : null;
  const rows = currentReport?.rows ?? [];
  const wards = groupWardDelivery(rows);
  const selectedRows = ward === "all" ? rows : rows.filter((row) => String(row.wardId) === ward);

  return <section className="workspace report-workspace ward-delivery-workspace" aria-labelledby="ward-delivery-heading">
    <div className="page-heading"><div><p className="eyebrow">Reports</p><h1 id="ward-delivery-heading">ใบส่งยาหอผู้ป่วย</h1><p className="page-summary">รายการที่ตรวจสอบแล้ว (Checked) ตามวันที่เตรียมยา · รวมจำนวนตามรายการยา</p></div></div>
    {navigation}
    <div className="surface report-filter-card ward-delivery-controls">
      <label className="compact-filter">วันที่เตรียมยา<BuddhistDateInput value={date} onChange={(value) => { setDate(value); setWard("all"); }} /></label>
      <label className="compact-filter">หอผู้ป่วย<select value={ward} onChange={(event) => setWard(event.target.value)}><option value="all">ทุกหอผู้ป่วย</option>{wards.map((item) => <option key={String(item.id)} value={String(item.id)}>{item.name}</option>)}</select></label>
      <button type="button" className="button button--secondary" onClick={() => { setState({ loading: true, report: null, error: null }); setRefresh((value) => value + 1); }}>โหลดใหม่</button>
      <button type="button" className="button button--primary" disabled={!currentReport || selectedRows.length === 0} onClick={() => window.print()}>พิมพ์ใบส่งยา</button>
    </div>
    {state.loading ? <div className="state-panel" aria-busy="true">กำลังโหลดใบส่งยา…</div> : state.error ? <div className="state-panel state-panel--error" role="alert">{state.error}</div> : selectedRows.length === 0 ? <div className="state-panel">ไม่มีรายการที่ตรวจสอบแล้วสำหรับวันที่และหอผู้ป่วยที่เลือก</div> : currentReport && <WardDeliveryDocument date={currentReport.preparationDate} rows={selectedRows} printedAt={printedAt} />}
  </section>;
}

export function WardDeliveryDocument({ date, rows, printedAt }: { date: string; rows: WardDeliveryRow[]; printedAt: Date }) {
  const wards = groupWardDelivery(rows);
  const time = new Intl.DateTimeFormat("en-GB", { timeZone: "Asia/Bangkok", hour: "2-digit", minute: "2-digit", second: "2-digit", hourCycle: "h23" }).format(printedAt);
  return <div className="ward-delivery-print-root">
    <header><h2>รายชื่อผู้ป่วยที่ได้รับยาเคมีบำบัด</h2><div className="ward-delivery-datetime"><span>วันที่: {displayDate(date)}</span><span>เวลา: {time}</span></div></header>
    {wards.map((ward) => <section className="ward-delivery-ward" key={String(ward.id)}>
      <h3>หอผู้ป่วย: {ward.name}</h3>
      <ol>{[...ward.patients.values()].map((patient) => <li className="ward-delivery-patient" key={patient.id}>
        <div className="ward-delivery-patient-name">{patient.name} <span className="ward-delivery-hn">(HN {patient.hn})</span></div>
        <ul>{patient.items.map((item) => <li key={item.taskId}>{formatDeliveryMedicine(item)}</li>)}</ul>
      </li>)}</ol>
      <footer><strong>รวม {ward.count} รายการ</strong><span>ผู้รับยา <span className="ward-delivery-signature" /></span><span>เวลา <span className="ward-delivery-received-time" /></span></footer>
    </section>)}
    <footer className="ward-delivery-total">รวมทั้งหมด {rows.length} รายการ</footer>
  </div>;
}

export function formatDeliveryMedicine(item: WardDeliveryRow): string {
  const dose = [item.orderedDoseText?.trim() || "ไม่ระบุขนาดยา", item.doseUnitText?.trim()].filter(Boolean).join(" ");
  const diluent = item.diluentName?.trim() || "ไม่ระบุสารละลาย";
  const volume = item.diluentVolumeMl === null ? "ไม่ระบุปริมาตร" : `${item.diluentVolumeMl} mL`;
  return `${item.drugName} ${dose} in ${diluent} ${volume}`;
}
