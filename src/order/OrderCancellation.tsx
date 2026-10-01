import { useState } from "react";
import { cancelOrder, commandError, getOrderCancellationPreview } from "../api/commands";
import type { CancellationPreview, OrderDetail } from "../types/order";
import { displayDate, displayDateTime } from "../shared/dateTime";

export function OrderCancellation({ order, initialItemId = null, onSaved }: { order: OrderDetail; initialItemId?: number | null; onSaved: (order: OrderDetail) => void }) {
  const [itemId, setItemId] = useState<number | null>(initialItemId);
  const [preview, setPreview] = useState<CancellationPreview | null>(null);
  const [reason, setReason] = useState("");
  const [decisions, setDecisions] = useState<Record<number, boolean>>({});
  const [acknowledged, setAcknowledged] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  async function review() {
    setBusy(true); setError(null); setPreview(null); setDecisions({}); setAcknowledged(false);
    try { setPreview(await getOrderCancellationPreview(order.id, itemId)); }
    catch (value) { setError(commandError(value).message ?? "ไม่สามารถตรวจสอบรายการได้"); }
    finally { setBusy(false); }
  }
  async function submit(event: React.FormEvent) {
    event.preventDefault(); if (!preview || !canSubmit) return;
    setBusy(true); setError(null);
    try {
      const result = await cancelOrder(order.id, { itemId: preview.itemId, revision: preview.revision, reason: reason.trim(), acknowledged,
        tasks: preview.tasks.map((task) => ({ taskId: task.id, actuallyPrepared: decisions[task.id] })) });
      setPreview(null); setReason(""); setItemId(null); onSaved(result);
    } catch (value) {
      setError(commandError(value).message ?? "ยกเลิกไม่สำเร็จ กรุณาโหลดข้อมูลตรวจสอบใหม่");
      setPreview(null); setAcknowledged(false);
    } finally { setBusy(false); }
  }
  const canSubmit = !!preview && !!reason.trim() && reason.trim().length <= 500 && acknowledged && preview.tasks.every((task) => typeof decisions[task.id] === "boolean");
  const lines = preview?.order.items.filter((item) => !item.cancelled && (preview.itemId === null || item.id === preview.itemId)) ?? [];
  return <section className="detail-section surface order-cancellation" aria-labelledby="order-cancellation-heading">
    <h2 id="order-cancellation-heading">ยกเลิก order / รายการยาที่คีย์ผิด</h2>
    {order.editable && <>
      <p>ยกเลิกโดยเก็บประวัติ มีผลทุกวันของรายการที่เลือก รวมถึงวันในอนาคต</p>
      <label>ขอบเขตการยกเลิก<select disabled={busy} value={itemId ?? "all"} onChange={(event) => { setItemId(event.target.value === "all" ? null : Number(event.target.value)); setPreview(null); setAcknowledged(false); }}>
        <option value="all">ทั้ง order — ทุกรายการที่ยังไม่ยกเลิก</option>
        {order.items.filter((item) => !item.cancelled).map((item) => <option key={item.id} value={item.id}>{item.orderingNo ?? item.id}. {item.drugName} {item.doseText} — {displayDate(item.startDate)} ถึง {displayDate(item.stopDate)}</option>)}
      </select></label>
      <button className="button button--secondary" disabled={busy} type="button" onClick={() => void review()}>{busy ? "กำลังดำเนินการ…" : "ตรวจสอบผลกระทบก่อนยกเลิก"}</button>
    </>}
    {error && <p className="form-error-summary" role="alert">{error}</p>}
    {preview && <form onSubmit={(event) => void submit(event)}>
      <p><strong>HN {preview.order.patientHn} · {preview.order.patientName} · Order {preview.order.orderId}</strong></p>
      <p>จะยกเลิก {lines.length} รายการยา ทุกวันของรายการนั้น และงานเตรียมที่มีอยู่ {preview.tasks.length} งาน</p>
      <ul>{lines.map((line) => <li key={line.id}>{line.drugName} {line.doseText} {line.regimenUnitText} · {displayDate(line.startDate)} ถึง {displayDate(line.stopDate)}</li>)}</ul>
      {preview.tasks.map((task) => <div className="cancellation-task" key={task.id}>
        <strong>{task.drugName} · {displayDate(task.preparationDate)} · {task.finalContainerCount} ภาชนะ</strong>
        <p>สถานะในระบบ: {task.state} · Stock: {task.inventoryStatus ?? "ยังไม่มีรายการตัด"}{task.printed ? " · เคยส่งพิมพ์ฉลากแล้ว" : ""}</p>
        <label>การเตรียมจริง — {task.drugName} {displayDate(task.preparationDate)}<select required disabled={busy} value={decisions[task.id] === undefined ? "" : decisions[task.id] ? "yes" : "no"} onChange={(event) => setDecisions((old) => ({ ...old, [task.id]: event.target.value === "yes" }))}>
          <option value="" disabled>เลือกตามงานที่เกิดขึ้นจริง</option>
          <option value="no">ยังไม่ได้เตรียมจริง — ไม่นับยอดเตรียม</option>
          <option value="yes" disabled={task.state === "pending"}>เตรียมจริงแล้ว — คงยอดงาน แยกเป็นยกเลิก</option>
        </select></label>
        {task.state === "pending" && <small>หากเตรียมจริงแล้ว ให้บันทึกงานเตรียมให้ครบก่อนกลับมายกเลิก</small>}
      </div>)}
      <label>เหตุผลการยกเลิก<textarea required maxLength={500} disabled={busy} value={reason} onChange={(event) => setReason(event.target.value)} placeholder="เช่น ลงผิดคน / คีย์ยาเกิน / คีย์ซ้ำ" /></label>
      <p>รายการจะออกจากคิวเตรียมและใบส่งยา และพิมพ์ฉลากใช้งานต่อไม่ได้ ระบบไม่คืน stock อัตโนมัติ</p>
      <label className="cancellation-ack"><input type="checkbox" checked={acknowledged} disabled={busy} onChange={(event) => setAcknowledged(event.target.checked)} />ฉันตรวจสอบผู้ป่วยและขอบเขตแล้ว จะจัดการฉลาก/ใบส่งยาที่พิมพ์ไปแล้ว และตรวจสอบ stock ตามการใช้จริง</label>
      <div className="item-actions"><button type="button" className="button button--secondary" disabled={busy} onClick={() => setPreview(null)}>กลับ</button><button className="button button--primary" disabled={busy || !canSubmit} type="submit">ยืนยันยกเลิกและเก็บประวัติ</button></div>
    </form>}
    {(order.cancellations ?? []).map((event) => <article className="cancellation-history" key={event.id}>
      <strong>{event.itemId === null ? "ยกเลิกทั้ง order" : `ยกเลิกรายการ: ${order.items.find((item) => item.id === event.itemId)?.drugName ?? event.itemId}`}</strong>
      <p>{event.reason}</p><small>{event.actorName} · {displayDateTime(event.occurredAt)}</small>
      <ul>{event.tasks.map((task) => <li key={task.id}>{task.drugName} · {displayDate(task.preparationDate)} · {task.actuallyPrepared ? "เตรียมแล้ว–ยกเลิก (คงยอดงานจริง)" : "ยกเลิกก่อนเตรียม (ไม่นับยอด)"}{task.inventoryStatus ? " · ตรวจสอบ stock: " + task.inventoryStatus : ""}</li>)}</ul>
    </article>)}
  </section>;
}
