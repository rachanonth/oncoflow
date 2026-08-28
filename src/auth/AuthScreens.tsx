import { useState } from "react";

import appIcon from "../../src-tauri/icons/icon.png";
import { bootstrapUser, commandError, loginUser } from "../api/commands";
import type { AuthState } from "../types/auth";
import { validateBootstrap, type BootstrapFormValues } from "./validation";

export function FirstRunSetup({ onAuthenticated }: { onAuthenticated: (state: AuthState) => void }) {
  const [values, setValues] = useState<BootstrapFormValues>({ username: "", displayName: "", password: "", confirmPassword: "" });
  const [errors, setErrors] = useState<ReturnType<typeof validateBootstrap>>({});
  const [submitError, setSubmitError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  function field(name: keyof BootstrapFormValues, value: string) { setValues((current) => ({ ...current, [name]: value })); setErrors((current) => ({ ...current, [name]: undefined })); }
  async function submit(event: React.FormEvent) {
    event.preventDefault();
    const nextErrors = validateBootstrap(values); setErrors(nextErrors);
    if (Object.keys(nextErrors).length > 0) return;
    setBusy(true); setSubmitError(null);
    try { onAuthenticated(await bootstrapUser({ username: values.username.trim(), displayName: values.displayName.trim(), password: values.password })); }
    catch (error) { setSubmitError(commandError(error).message ?? "First-run setup could not be completed."); }
    finally { setBusy(false); }
  }
  return <AuthFrame eyebrow="First-run setup" title="Create the first local account" summary="Establish a new offline OncoFlow administrator credential. Legacy Access passwords are never accepted.">
    <form className="auth-form" onSubmit={(event) => void submit(event)} noValidate>
      {submitError && <div className="auth-error" role="alert">{submitError}</div>}
      <AuthField label="Username" error={errors.username}><input autoFocus autoComplete="username" value={values.username} onChange={(event) => field("username", event.target.value)} /></AuthField>
      <AuthField label="Display name" error={errors.displayName}><input autoComplete="name" value={values.displayName} onChange={(event) => field("displayName", event.target.value)} /></AuthField>
      <AuthField label="New password" hint="12–128 characters" error={errors.password}><input type="password" autoComplete="new-password" value={values.password} onChange={(event) => field("password", event.target.value)} /></AuthField>
      <AuthField label="Confirm password" error={errors.confirmPassword}><input type="password" autoComplete="new-password" value={values.confirmPassword} onChange={(event) => field("confirmPassword", event.target.value)} /></AuthField>
      <button className="button button--primary auth-submit" type="submit" disabled={busy}>{busy ? "Creating local account…" : "Create local account"}</button>
      <p className="auth-privacy">No factory password is created. Only a salted password hash is stored in this device's <code>oncoflow.db</code>.</p>
    </form>
  </AuthFrame>;
}

export function LoginScreen({ onAuthenticated, initialError = null }: { onAuthenticated: (state: AuthState) => void; initialError?: string | null }) {
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(initialError);
  const [busy, setBusy] = useState(false);
  async function submit(event: React.FormEvent) {
    event.preventDefault();
    if (!username.trim() || !password) { setError("Enter the local username and password."); return; }
    setBusy(true); setError(null);
    try { onAuthenticated(await loginUser({ username: username.trim(), password })); }
    catch (cause) { setError(commandError(cause).message ?? "The local login was not accepted."); }
    finally { setBusy(false); }
  }
  return <AuthFrame>
    <form className="auth-form" onSubmit={(event) => void submit(event)} noValidate>
      {error && <div className="auth-error" role="alert">{error}</div>}
      <AuthField label="Username"><input autoFocus autoComplete="username" value={username} onChange={(event) => setUsername(event.target.value)} /></AuthField>
      <AuthField label="Password"><input type="password" autoComplete="current-password" value={password} onChange={(event) => setPassword(event.target.value)} /></AuthField>
      <button className="button button--primary auth-submit" type="submit" disabled={busy}>{busy ? "Signing in…" : "Sign in"}</button>
    </form>
  </AuthFrame>;
}

export function AuthFrame({ eyebrow, title, summary, children }: { eyebrow?: string; title?: string; summary?: string; children: React.ReactNode }) {
  const hasHeading = eyebrow || title || summary;
  return <main className="auth-shell"><section className="auth-card"><div className="auth-brand"><div className="brand-mark" aria-hidden="true"><img src={appIcon} alt="" /></div><div><strong>OncoFlow</strong><span>Chemotherapy preparation</span></div></div>{hasHeading && <div className="auth-heading">{eyebrow && <p className="eyebrow">{eyebrow}</p>}{title && <h1>{title}</h1>}{summary && <p>{summary}</p>}</div>}{children}<footer><span className="local-badge__dot" aria-hidden="true"/> Local-only · SQLite · no external identity provider</footer></section></main>;
}

function AuthField({ label, hint, error, children }: { label: string; hint?: string; error?: string; children: React.ReactNode }) {
  return <label className="auth-field"><span>{label}{hint && <small>{hint}</small>}</span>{children}{error && <b role="alert">{error}</b>}</label>;
}
