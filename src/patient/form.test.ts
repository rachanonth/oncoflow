import { describe, expect, it } from "vitest";

import {
  emptyPatientForm,
  formToInput,
  validatePatientForm,
  type PatientFormValues,
} from "./form";
import { patientName } from "./format";
import { calculateAgeYears } from "./age";

function form(overrides: Partial<PatientFormValues> = {}): PatientFormValues {
  return { ...emptyPatientForm, hn: "SYN-001", ...overrides };
}

describe("patient form validation", () => {
  it("requires an HN and rejects invalid measurements and dates", () => {
    const errors = validatePatientForm(
      form({ hn: "  ", weightKg: "-1", heightCm: "301", birthDate: "2025-02-30" }),
    );

    expect(errors.hn).toBeDefined();
    expect(errors.weightKg).toBeDefined();
    expect(errors.heightCm).toBeDefined();
    expect(errors.birthDate).toBeDefined();
  });

  it("trims text and converts empty optional fields to null", () => {
    const input = formToInput(
      form({
        hn: "  SYN-TH-01  ",
        firstName: "  สมชาย  ",
        lastName: "   ",
        weightKg: "62.5",
      }),
    );

    expect(input.hn).toBe("SYN-TH-01");
    expect(input.firstName).toBe("สมชาย");
    expect(input.lastName).toBeNull();
    expect(input.weightKg).toBe(62.5);
  });

  it("accepts real calendar dates", () => {
    expect(
      validatePatientForm(form({ birthDate: "2024-02-29" })),
    ).toEqual({});
  });

  it("validates and stores an exact age when birth date is unavailable", () => {
    expect(validatePatientForm(form({ ageYears: "0" }))).toEqual({});
    expect(validatePatientForm(form({ ageYears: "42" }))).toEqual({});
    expect(validatePatientForm(form({ ageYears: "42.5" })).ageYears).toBeDefined();
    expect(validatePatientForm(form({ ageYears: "151" })).ageYears).toBeDefined();
    expect(formToInput(form({ ageYears: "42" })).ageYears).toBe(42);
  });

  it("calculates completed years from birth date", () => {
    expect(calculateAgeYears("2000-08-25", "2026-08-25")).toBe(26);
    expect(calculateAgeYears("2000-08-26", "2026-08-25")).toBe(25);
    expect(calculateAgeYears("2027-01-01", "2026-08-25")).toBeNull();
  });

  it("stores selected sex values and keeps Not specified as null", () => {
    expect(formToInput(form({ sex: "Male" })).sex).toBe("Male");
    expect(formToInput(form({ sex: "Female" })).sex).toBe("Female");
    expect(formToInput(form({ sex: "" })).sex).toBeNull();
  });
});

describe("patient display", () => {
  it("preserves Thai names", () => {
    expect(
      patientName({ title: "นาย", firstName: "สมชาย", lastName: "ทดสอบ" }),
    ).toBe("นาย สมชาย ทดสอบ");
  });
});
