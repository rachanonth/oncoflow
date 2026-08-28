import { useState } from "react";

import { InventoryUsageReport } from "./InventoryUsageReport";
import { PreparationCountReport } from "./PreparationCountReport";

type ReportKey = "preparation_count" | "inventory_usage";

export function ReportsWorkspace() {
  const [active, setActive] = useState<ReportKey>("preparation_count");
  const navigation = <nav className="report-navigation" aria-label="เลือกรายงาน">
    <button type="button" className={active === "preparation_count" ? "is-active" : ""} aria-current={active === "preparation_count" ? "page" : undefined} onClick={() => setActive("preparation_count")}><span>01</span>จำนวนการเตรียมยา</button>
    <button type="button" className={active === "inventory_usage" ? "is-active" : ""} aria-current={active === "inventory_usage" ? "page" : undefined} onClick={() => setActive("inventory_usage")}><span>02</span>การใช้ยาและ Stock</button>
  </nav>;
  return active === "preparation_count"
    ? <PreparationCountReport navigation={navigation} />
    : <InventoryUsageReport navigation={navigation} />;
}
