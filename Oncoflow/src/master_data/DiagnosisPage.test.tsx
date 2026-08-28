import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { DiagnosisTable, validateDiagnosis } from "./DiagnosisPage";

describe("diagnosis master data", () => {
  it("renders Thai diagnosis names without codes or internal IDs", () => {
    const html = renderToStaticMarkup(<DiagnosisTable records={[{ id: 811, name: "มะเร็งเต้านมทดสอบ" }]} loading={false} onEdit={() => undefined} />);
    expect(html).toContain("มะเร็งเต้านมทดสอบ");
    expect(html).not.toContain(">811<");
    expect(html).not.toContain("legacy");
  });

  it("requires a bounded diagnosis name and accepts Thai text", () => {
    expect(validateDiagnosis({ name: " " }).name).toBeTruthy();
    expect(validateDiagnosis({ name: "x".repeat(201) }).name).toBeTruthy();
    expect(validateDiagnosis({ name: " มะเร็งทดสอบ " })).toEqual({});
  });
});
