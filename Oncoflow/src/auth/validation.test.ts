import { describe, expect, it } from "vitest";

import { validateBootstrap, validatePasswordChange } from "./validation";

describe("local authentication form validation", () => {
  it("rejects incomplete bootstrap credentials and a username password", () => {
    expect(validateBootstrap({ username: "ab", displayName: "", password: "short", confirmPassword: "different" })).toMatchObject({ username: expect.any(String), displayName: expect.any(String), password: expect.any(String), confirmPassword: expect.any(String) });
    expect(validateBootstrap({ username: "local-pharmacist", displayName: "Synthetic Pharmacist", password: "LOCAL-PHARMACIST", confirmPassword: "LOCAL-PHARMACIST" }).password).toBeDefined();
  });

  it("accepts a new offline credential and validates password changes", () => {
    expect(validateBootstrap({ username: "local.pharmacist", displayName: "เภสัชกรทดสอบ", password: "new-local-password-123", confirmPassword: "new-local-password-123" })).toEqual({});
    expect(validatePasswordChange({ currentPassword: "old-password", newPassword: "new-local-password-456", confirmPassword: "mismatch" }).confirmPassword).toBeDefined();
    expect(validatePasswordChange({ currentPassword: "old-password", newPassword: "new-local-password-456", confirmPassword: "new-local-password-456" })).toEqual({});
  });
});
