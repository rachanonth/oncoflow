import { useState } from "react";

import { InventoryUsageReport } from "./InventoryUsageReport";
import { PreparationCountReport } from "./PreparationCountReport";
import { WardDeliveryReport } from "./WardDeliveryReport";
import { DiluentUsageReport } from "./DiluentUsageReport";

type ReportKey = "preparation_count" | "inventory_usage" | "ward_delivery" | "diluent_usage";

export function ReportsWorkspace() {
  const [active, setActive] = useState<ReportKey>("preparation_count");
  const navigation = <nav className="report-navigation" aria-label="เลือกรายงาน">
    <button type="button" className={active === "preparation_count" ? "is-active" : ""} aria-current={active === "preparation_count" ? "page" : undefined} onClick={() => setActive("preparation_count")}><span>01</span>จำนวนการเตรียมยา</button>
    <button type="button" className={active === "inventory_usage" ? "is-active" : ""} aria-current={active === "inventory_usage" ? "page" : undefined} onClick={() => setActive("inventory_usage")}><span>02</span>การใช้ยาและ Stock</button>
    <button type="button" className={active === "ward_delivery" ? "is-active" : ""} aria-current={active === "ward_delivery" ? "page" : undefined} onClick={() => setActive("ward_delivery")}><span>03</span>ใบส่งยาหอผู้ป่วย</button>
    <button type="button" className={active === "diluent_usage" ? "is-active" : ""} aria-current={active === "diluent_usage" ? "page" : undefined} onClick={() => setActive("diluent_usage")}><span>04</span>จำนวนใช้ Diluent รายวัน</button>
  </nav>;
  if (active === "diluent_usage") return <DiluentUsageReport navigation={navigation} />;
  return active === "preparation_count"
    ? <PreparationCountReport navigation={navigation} />
    : active === "inventory_usage" ? <InventoryUsageReport navigation={navigation} /> : <WardDeliveryReport navigation={navigation} />;
}
