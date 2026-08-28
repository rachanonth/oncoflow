import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import type { OrderDetail as Detail, OrderItemDetail } from "../types/order";
import { PatientOrderHistory, PatientOrderHistoryTable } from "./PatientOrderHistory";
import { calculateBodySurfaceArea, OrderCumulativeDose, OrderDetailHeader, OrderDrugsTable, OrderMeasurements, OrderStatusPanel, OrderSummary } from "./OrderDetail";

const order: Detail = {
  id: 1,
  orderId: "OF-001",
  patientId: 2,
  patientHn: "HN-123",
  patientName: "Synthetic Patient",
  weightKg: 70,
  heightCm: 175,
  assignedPreparerUserId: 7,
  assignedPreparerName: "เภสัชกรผู้เตรียม",
  wardId: 3,
  wardName: "Synthetic Ward",
  doctorId: 4,
  doctorName: "Synthetic Doctor",
  regimenId: 5,
  regimenName: "Synthetic Regimen",
  note: "Synthetic note",
  orderTime: "2026-08-25T09:30",
  orderType: "LX",
  appointmentFlag: true,
  legacyWorker: "Legacy Worker",
  editWorker: null,
  sideEffectText: null,
  sideEffectRecorder: null,
  sideEffectRecordTime: null,
  medicationErrorText: null,
  editable: true,
  workflowStatus: "active",
  workflowStatusReason: null,
  workflowStatusChangedAt: null,
  workflowStatusChangedBy: null,
  statusEvents: [],
  cumulativeDoses: [],
  items: [],
};

const drug: OrderItemDetail = {
  id: 10,
  drugId: 20,
  drugName: "Synthetic drug",
  diluentId: 30,
  diluentName: "NSS",
  diluentVolumeMl: 100,
  routeId: 40,
  routeName: "IV",
  startDate: "2026-08-25",
  stopDate: "2026-08-25",
  dose: 125,
  doseText: "125",
  scheduleTime: "09:00",
  numberOfDrug: 2,
  missing: true,
  printed: false,
  rate: "60 min",
  orderingNo: 1,
  runningNo: 1,
  runningSum: 1,
  inventoryDate: null,
  sourceRegimenItemId: null,
  regimenDoseText: null,
  regimenUnitText: "mg",
  regimenRouteText: null,
  regimenDetails: null,
  regimenItemGroup: null,
  regimenDuration: null,
  regimenStartDay: null,
  regimenOrderingNo: null,
};

describe("order detail presentation", () => {
  it("puts patient identity first and uses a pencil-only edit action", () => {
    const html = renderToStaticMarkup(<OrderDetailHeader order={order} onEdit={() => undefined} />);

    expect(html).toContain("HN-123");
    expect(html).toContain("Synthetic Patient");
    expect(html).toContain("Order: <strong>OF-001</strong>");
    expect(html).toContain("<svg");
    expect(html).toContain('aria-label="Edit order OF-001"');
    expect(html).not.toContain("Local order");
  });

  it("limits the overview to the requested five fields", () => {
    const html = renderToStaticMarkup(<OrderSummary order={order} />);

    for (const label of ["Date / time", "Regimen", "Doctor", "Ward", "Notes"]) expect(html).toContain(label);
    for (const label of ["Order ID", "Patient", "HN", "Legacy type", "Appointment flag", "Record mode"]) expect(html).not.toContain(`<dt>${label}</dt>`);
  });

  it("shows the order-time measurement snapshot and Mosteller BSA", () => {
    const html = renderToStaticMarkup(<OrderMeasurements order={order} saving={false} onSave={async () => undefined} />);

    expect(html).toContain("Order-time snapshot");
    expect(html).toContain("70 kg");
    expect(html).toContain("175 cm");
    expect(html).toContain("1.84 m²");
    expect(html).toContain("Mosteller formula");
    expect(html).toContain('aria-label="Edit order weight"');
  });

  it("calculates BSA only when both positive measurements are available", () => {
    expect(calculateBodySurfaceArea(70, 175)).toBeCloseTo(1.8447, 4);
    expect(calculateBodySurfaceArea(null, 175)).toBeNull();
    expect(calculateBodySurfaceArea(70, null)).toBeNull();
  });

  it("shows dose units and separates route/rate without legacy metadata", () => {
    const html = renderToStaticMarkup(<OrderDrugsTable order={{ ...order, items: [drug] }} mutating={false} onMove={() => undefined} onEdit={() => undefined} onRemove={() => undefined} />);

    for (const heading of ["Drug / dose", "Preparation", "Route / rate", "Schedule"]) expect(html).toContain(heading);
    expect(html).toContain('class="order-dose">125<small class="order-dose__unit">mg</small>');
    expect(html.indexOf("Preparation")).toBeLessThan(html.indexOf("Route / rate"));
    expect(html).toContain("IV");
    expect(html).toContain("60 min");
    for (const hidden of ["Legacy metadata", "Legacy quantity", "Missing:", "Printed:"]) expect(html).not.toContain(hidden);
  });

  it("requires no routine attendance confirmation and exposes only the no-show exception", () => {
    const html = renderToStaticMarkup(<OrderStatusPanel order={order} busy={false} onNoShow={async () => undefined} onReschedule={async () => undefined} />);
    expect(html).toContain("ผู้ป่วยไม่มาตามนัด");
    expect(html).not.toContain("No action is required for normal attendance");
    expect(html).not.toContain("Continue order on new date");
  });

  it("supports an embedded patient order history without a duplicate create action", () => {
    const html = renderToStaticMarkup(<PatientOrderHistory patientId={order.patientId} currentOrderId={order.id} onOpen={() => undefined} />);
    expect(html).toContain("Order history");
    expect(html).toContain("Loading order history");
    expect(html).not.toContain("New order");
  });

  it("shows drug names in order history and opens an order from its number", () => {
    const html = renderToStaticMarkup(<PatientOrderHistoryTable items={[{
      id: 2,
      orderId: "OF-002",
      patientId: order.patientId,
      patientHn: order.patientHn,
      patientName: order.patientName,
      orderTime: order.orderTime,
      regimenName: order.regimenName,
      doctorName: order.doctorName,
      wardName: order.wardName,
      orderType: order.orderType,
      itemCount: 2,
      drugs: [
        { drugName: "Paclitaxel", doseText: "175", unitText: "mg" },
        { drugName: "Carboplatin", doseText: "450", unitText: "mg" },
      ],
      editable: true,
      workflowStatus: "active",
    }]} onOpen={() => undefined} />);
    expect(html).toContain("Paclitaxel 175 mg · Carboplatin 450 mg");
    expect(html).toContain('aria-label="Open order OF-002"');
    expect(html).not.toContain("row-action");
  });

  it("shows only the cumulative summaries supplied by the existing safety rule", () => {
    const items = [{
      drugId: 20,
      drugName: "Doxorubicin",
      totalDose: "200.6",
      threshold: "450",
    }];
    const html = renderToStaticMarkup(<OrderCumulativeDose items={items} onOpenDrug={() => undefined} />);
    expect(html).toContain("Cumulative dose");
    expect(html).toContain("Doxorubicin");
    expect(html).toContain("201 mg/m²");
    expect(html).not.toContain("200.6 mg/m²");
    expect(html).toContain(">Threshold<");
    expect(html).not.toContain("Configured threshold");
    expect(html).toContain('aria-label="Open Doxorubicin drug master record"');
    expect(renderToStaticMarkup(<OrderCumulativeDose items={items} />)).not.toContain("drug master record");
    expect(html).toContain("450 mg/m²");
    expect(html).toContain("Σ (recorded dose ÷ order-time BSA snapshot)");
    expect(html).not.toContain("Review required");
  });

  it("offers one continue action for an order held after a no-show", () => {
    const held: Detail = { ...order, workflowStatus: "on_hold", workflowStatusReason: "no_show", statusEvents: [{ id: 1, eventType: "no_show", fromStatus: "active", toStatus: "on_hold", effectiveDate: "2026-08-25", relatedDate: null, actorDisplayName: "Synthetic assistant", occurredAt: "2026-08-25T10:00" }] };
    const html = renderToStaticMarkup(<OrderStatusPanel order={held} busy={false} onNoShow={async () => undefined} onReschedule={async () => undefined} />);
    expect(html).toContain("Order on hold");
    expect(html).toContain("Continue order on new date");
    expect(html).toContain("original order and drug start/stop dates have not been changed");
  });
});
