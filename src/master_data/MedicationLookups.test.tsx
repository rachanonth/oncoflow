import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { DiluentTable, RouteTable, validateDiluent, validateRoute } from "./MedicationLookups";

describe("diluent and route master data", () => {
  it("renders Thai names and diluent volume without showing compatibility codes", () => {
    const routeHtml = renderToStaticMarkup(<RouteTable records={[{ id: 701, legacyCode: "R-HIDDEN", name: "ให้ทางหลอดเลือดดำ" }]} loading={false} onEdit={() => undefined} />);
    const diluentHtml = renderToStaticMarkup(<DiluentTable records={[{ id: 702, legacyCode: "D-HIDDEN", name: "สารละลายทดสอบ", volumeMl: 100.5 }]} loading={false} onEdit={() => undefined} />);

    expect(routeHtml).toContain("ให้ทางหลอดเลือดดำ");
    expect(diluentHtml).toContain("สารละลายทดสอบ");
    expect(diluentHtml).toContain("100.5");
    expect(routeHtml).not.toContain("R-HIDDEN");
    expect(diluentHtml).not.toContain("D-HIDDEN");
  });

  it("requires names and validates optional volume", () => {
    expect(validateRoute({ name: " " }).name).toBeTruthy();
    expect(validateDiluent({ name: "", volumeMl: "-1" })).toEqual({
      name: expect.any(String),
      volumeMl: expect.any(String),
    });
    expect(validateDiluent({ name: "Synthetic", volumeMl: "not-a-number" }).volumeMl).toBeTruthy();
  });

  it("accepts Thai names, zero, decimals, and NULL-equivalent blank volume", () => {
    expect(validateRoute({ name: " ฉีดเข้าหลอดเลือดดำ " })).toEqual({});
    expect(validateDiluent({ name: " น้ำเกลือทดสอบ ", volumeMl: "" })).toEqual({});
    expect(validateDiluent({ name: "น้ำเกลือทดสอบ", volumeMl: "0" })).toEqual({});
    expect(validateDiluent({ name: "น้ำเกลือทดสอบ", volumeMl: "100.5" })).toEqual({});
  });
});
