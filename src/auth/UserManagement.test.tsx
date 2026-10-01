import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import type { ManagedUser } from "../types/auth";
import { AccessLevelSelect, PasswordResetEditor, UserTable, validateManagedUser, validatePasswordReset } from "./UserManagement";

const users: ManagedUser[] = [
  {
    id: 1,
    username: "local.admin",
    displayName: "ผู้ดูแลระบบ",
    role: "admin",
    userType: "pharmacist",
    active: true,
    createdAt: "2026-08-25 09:00:00",
    updatedAt: "2026-08-25 09:00:00",
  },
  {
    id: 2,
    username: "local.support",
    displayName: "เจ้าหน้าที่ทดสอบ",
    role: "pharmacist",
    userType: "non_pharmacist",
    active: false,
    createdAt: "2026-08-25 09:05:00",
    updatedAt: "2026-08-25 09:06:00",
  },
];

describe("UserManagement", () => {
  it("renders pharmacist and non-pharmacist accounts without credential data", () => {
    const html = renderToStaticMarkup(<UserTable users={users} currentUserId={1} loading={false} onEdit={() => undefined} />);
    expect(html).toContain("ผู้ดูแลระบบ");
    expect(html).toContain("เจ้าหน้าที่ทดสอบ");
    expect(html).toContain("Pharmacist");
    expect(html).toContain("Assistant pharmacist");
    expect(html).toContain("Inactive");
    expect(html).not.toContain("passwordHash");
    expect(html).not.toContain("$argon2");
  });

  it("requires a strong matching initial password only when creating", () => {
    const invalid = validateManagedUser({ username: "a b", displayName: "", password: "short", confirmPassword: "different", userType: "pharmacist", role: "pharmacist", active: true }, false);
    expect(invalid.username).toBeTruthy();
    expect(invalid.displayName).toBeTruthy();
    expect(invalid.password).toBeTruthy();
    expect(invalid.confirmPassword).toBeTruthy();

    const editing = validateManagedUser({ username: "local.support", displayName: "เจ้าหน้าที่ทดสอบ", password: "", confirmPassword: "", userType: "non_pharmacist", role: "admin", active: false }, true);
    expect(editing).toEqual({});
  });

  it("shows administrator access separately from pharmacist identity type", () => {
    const html = renderToStaticMarkup(<UserTable users={users} currentUserId={99} loading={false} onEdit={() => undefined} />);
    expect(html).toContain("Administrator");
    expect(html).toContain("Standard");
    expect(html).toContain("Pharmacist");
    expect(html).toContain("Assistant pharmacist");
  });

  it("offers standard and administrator access levels when editing", () => {
    const html = renderToStaticMarkup(<AccessLevelSelect value="admin" disabled={false} onChange={() => undefined} />);
    expect(html).toContain("Standard");
    expect(html).toContain("Administrator");
    expect(html).toContain('value="admin" selected=""');
  });
});

describe("Admin password reset", () => {
  it("requires a strong confirmed replacement", () => {
    expect(validatePasswordReset("admin", "", "")).toBeTruthy();
    expect(validatePasswordReset("long.username", "LONG.USERNAME", "LONG.USERNAME")).toBeTruthy();
    expect(validatePasswordReset("admin", "new-password-123", "different")).toBeTruthy();
    expect(validatePasswordReset("admin", "new-password-123", "new-password-123")).toBeNull();
  });
  it("identifies the target and consequences without asking for the old password", () => {
    const html = renderToStaticMarkup(<PasswordResetEditor user={users[1]} onBusy={() => undefined} onCancel={() => undefined} onDone={() => undefined} />);
    expect(html).toContain("local.support");
    expect(html).toContain("existing sessions will end");
    expect(html).toContain("Confirm password reset");
    expect(html).not.toContain("current-password");
  });
  it("disables reset for the signed-in administrator", () => {
    const html = renderToStaticMarkup(<UserTable users={[users[0]]} currentUserId={1} loading={false} onEdit={() => undefined} onReset={() => undefined} />);
    expect(html).toMatch(/disabled=""[^>]*>Reset password/);
  });
});
