import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { PageDescription, PageDescriptionContent, toGuidanceMap } from "./PageGuidance";
import { validateGuidance } from "./GuidanceSettings";
import { PAGE_DESCRIPTIONS } from "./pageDescriptions";

describe("page descriptions and Guidance", () => {
  it("renders the fixed Thai patient description", () => {
    const html = renderToStaticMarkup(<PageDescription pageKey="patients" />);
    expect(html).toContain("ค้นหาและดูแลข้อมูลผู้ป่วย");
    expect(html).not.toContain("Search and maintain patient records");
  });

  it("maps only persisted Guidance and validates the Unicode limit", () => {
    expect(toGuidanceMap([{ pageKey: "patients", guidance: "ตรวจสอบ HN" }, { pageKey: "orders", guidance: null }])).toEqual({ patients: "ตรวจสอบ HN" });
    expect(validateGuidance("ก".repeat(500))).toBe(true);
    expect(validateGuidance("ก".repeat(501))).toBe(false);
  });

  it("renders optional Guidance separately from the fixed Thai description", () => {
    const html = renderToStaticMarkup(<PageDescriptionContent pageKey="patients" guidance="Verify HN before creating a record." />);
    expect(html).toContain("Guidance");
    expect(html).toContain("Verify HN before creating a record.");
    expect(html).toContain("ค้นหาและดูแลข้อมูลผู้ป่วย");
  });

  it("keeps every standard description concise without redundant storage wording", () => {
    for (const page of PAGE_DESCRIPTIONS) {
      expect(page.description).not.toMatch(/ที่จัดเก็บ|ภายในเครื่อง|เครื่องนี้|OncoFlow|ฐานข้อมูล/);
    }
  });
});
