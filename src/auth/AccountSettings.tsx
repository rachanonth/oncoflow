import { useState } from "react";

import { changePassword, commandError } from "../api/commands";
import { PageDescription } from "../guidance/PageGuidance";
import type { CurrentUser } from "../types/auth";
import { validatePasswordChange, type PasswordFormValues } from "./validation";

export function AccountSettings({ user }: { user: CurrentUser }) {
  const [values, setValues] = useState<PasswordFormValues>({ currentPassword: "", newPassword: "", confirmPassword: "" });
  const [errors, setErrors] = useState<ReturnType<typeof validatePasswordChange>>({});
  const [message, setMessage] = useState<{ tone: "error" | "success"; text: string } | null>(null);
  const [busy, setBusy] = useState(false);
  function field(name: keyof PasswordFormValues, value: string) { setValues((current) => ({ ...current, [name]: value })); setErrors((current) => ({ ...current, [name]: undefined })); setMessage(null); }
  async function submit(event: React.FormEvent) {
    event.preventDefault();
    const nextErrors = validatePasswordChange(values); setErrors(nextErrors);
    if (Object.keys(nextErrors).length > 0) return;
    setBusy(true); setMessage(null);
    try { await changePassword({ currentPassword: values.currentPassword, newPassword: values.newPassword }); setValues({ currentPassword: "", newPassword: "", confirmPassword: "" }); setMessage({ tone: "success", text: "Password changed. Use the new password at the next sign-in." }); }
    catch (error) { setMessage({ tone: "error", text: commandError(error).message ?? "Password could not be changed." }); }
    finally { setBusy(false); }
  }
  return <section className="workspace account-workspace" aria-labelledby="account-heading"><div className="page-heading"><div><p className="eyebrow">Settings</p><h1 id="account-heading">Account</h1><PageDescription pageKey="account" /></div></div><div className="account-grid"><section className="surface account-card"><p className="eyebrow">Current identity</p><div className="account-identity"><span>{initials(user.displayName)}</span><div><h2>{user.displayName}</h2><p>@{user.username}</p><b>{userTypeLabel(user.userType)}{user.role === "admin" ? " · Local administrator" : ""}</b></div></div><p className="account-boundary">User type records local identity metadata. It is not a digital signature, clinical competency designation, or automatic clinical permission.</p></section><section className="surface account-card"><p className="eyebrow">Credential</p><h2>Change password</h2><form className="account-password-form" onSubmit={(event) => void submit(event)} noValidate>{message && <div className={message.tone === "error" ? "auth-error" : "auth-success"} role={message.tone === "error" ? "alert" : "status"}>{message.text}</div>}<PasswordField label="Current password" error={errors.currentPassword}><input type="password" autoComplete="current-password" value={values.currentPassword} onChange={(event) => field("currentPassword", event.target.value)}/></PasswordField><PasswordField label="New password" error={errors.newPassword}><input type="password" autoComplete="new-password" value={values.newPassword} onChange={(event) => field("newPassword", event.target.value)}/></PasswordField><PasswordField label="Confirm new password" error={errors.confirmPassword}><input type="password" autoComplete="new-password" value={values.confirmPassword} onChange={(event) => field("confirmPassword", event.target.value)}/></PasswordField><button className="button button--primary" type="submit" disabled={busy}>{busy ? "Changing password…" : "Change password"}</button></form></section></div></section>;
}

function PasswordField({ label, error, children }: { label: string; error?: string; children: React.ReactNode }) { return <label><span>{label}</span>{children}{error && <b className="field-error">{error}</b>}</label>; }
function initials(value: string): string { return value.trim().split(/\s+/).slice(0, 2).map((part) => part[0] ?? "").join("").toLocaleUpperCase() || "OF"; }
function userTypeLabel(value: CurrentUser["userType"]): string { return value === "pharmacist" ? "Pharmacist" : "Assistant pharmacist"; }
