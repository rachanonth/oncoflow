import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { DoctorTable, validateDoctor, validateWard, WardTable } from "./MasterDataPages";

describe("doctor and ward master data", () => {
  it("renders Thai names without exposing internal numeric identifiers", () => {
    const doctorHtml = renderToStaticMarkup(<DoctorTable records={[{ id: 901, legacyCode: "D-SYN", name: "นพ. ทดสอบ ระบบ" }]} loading={false} onEdit={() => undefined} />);
    const wardHtml = renderToStaticMarkup(<WardTable records={[{ id: 902, legacyCode: "W-SYN", name: "หอผู้ป่วยเคมีบำบัด", telephone: "1234" }]} loading={false} onEdit={() => undefined} />);

    expect(doctorHtml).toContain("นพ. ทดสอบ ระบบ");
    expect(wardHtml).toContain("หอผู้ป่วยเคมีบำบัด");
    expect(wardHtml).toContain("1234");
    expect(doctorHtml).not.toContain("D-SYN");
    expect(wardHtml).not.toContain("W-SYN");
    expect(doctorHtml).not.toContain(">901<");
    expect(wardHtml).not.toContain(">902<");
    expect(doctorHtml).toContain('aria-label="Edit doctor นพ. ทดสอบ ระบบ"');
    expect(wardHtml).toContain('aria-label="Edit ward หอผู้ป่วยเคมีบำบัด"');
    expect(doctorHtml).toContain("<svg");
    expect(wardHtml).toContain("<svg");
  });

  it("renders existing records as inline editors", () => {
    const doctorHtml = renderToStaticMarkup(<DoctorTable
      records={[{ id: 901, legacyCode: "D-SYN", name: "Original doctor" }]}
      loading={false}
      onEdit={() => undefined}
      editor={{ recordId: 901, values: { name: "Edited doctor" }, errors: {}, busy: false, onChange: () => undefined, onCancel: () => undefined, onSubmit: () => undefined }}
    />);
    const wardHtml = renderToStaticMarkup(<WardTable
      records={[{ id: 902, legacyCode: "W-SYN", name: "Original ward", telephone: "1234" }]}
      loading={false}
      onEdit={() => undefined}
      editor={{ recordId: 902, values: { name: "Edited ward", telephone: "5678" }, errors: {}, busy: false, onNameChange: () => undefined, onTelephoneChange: () => undefined, onCancel: () => undefined, onSubmit: () => undefined }}
    />);

    expect(doctorHtml).toContain("master-data-inline-editor--doctor");
    expect(doctorHtml).toContain('value="Edited doctor"');
    expect(wardHtml).toContain("master-data-inline-editor--ward");
    expect(wardHtml).toContain('value="Edited ward"');
    expect(wardHtml).toContain('value="5678"');
  });

  it("requires a doctor or ward name and limits telephone length", () => {
    expect(validateDoctor({ name: "  " })).toEqual({
      name: expect.any(String),
    });
    expect(validateWard({ name: "", telephone: "1".repeat(101) })).toEqual({
      name: expect.any(String),
      telephone: expect.any(String),
    });
  });

  it("accepts trimmed Thai names and optional blank metadata", () => {
    expect(validateDoctor({ name: " พญ. ตัวอย่าง " })).toEqual({});
    expect(validateWard({ name: " หอผู้ป่วยตัวอย่าง ", telephone: "" })).toEqual({});
  });
});
