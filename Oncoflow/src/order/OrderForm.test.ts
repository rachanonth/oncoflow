import { describe, expect, it } from "vitest";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";

import { findPatientByHn, OrderPatientMatch } from "./OrderForm";

const patients = [
  { id: 1, hn: "HN-001", label: "First Patient" },
  { id: 2, hn: "100234", label: "Second Patient" },
];

describe("New order patient HN lookup", () => {
  it("finds the patient by an exact HN without case or surrounding-space sensitivity", () => {
    expect(findPatientByHn(" hn-001 ", patients)?.label).toBe("First Patient");
  });

  it("does not select a patient for a partial or unknown HN", () => {
    expect(findPatientByHn("HN", patients)).toBeUndefined();
    expect(findPatientByHn("unknown", patients)).toBeUndefined();
  });

  it("offers patient creation only after an entered HN is not found", () => {
    const missing = renderToStaticMarkup(
      createElement(OrderPatientMatch, { patientHn: "NEW-001", onCreatePatient: () => undefined }),
    );
    const empty = renderToStaticMarkup(
      createElement(OrderPatientMatch, { patientHn: "", onCreatePatient: () => undefined }),
    );
    const found = renderToStaticMarkup(
      createElement(OrderPatientMatch, { patientHn: "HN-001", selectedPatient: patients[0], onCreatePatient: () => undefined }),
    );

    expect(missing).toContain("Patient not found");
    expect(missing).toContain("Add new patient");
    expect(empty).not.toContain("Add new patient");
    expect(found).not.toContain("Add new patient");
  });
});
