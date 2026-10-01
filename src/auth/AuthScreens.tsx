import { useContext, useRef, useState } from "react";

import appIcon from "../../src-tauri/icons/icon.png";
import { bootstrapUser, commandError, loginUser } from "../api/commands";
import type { AuthState } from "../types/auth";
import { validateBootstrap, type BootstrapFormValues } from "./validation";
import { clearUsernameHistory, readUsernameHistory, rememberSuccessfulUsername, UsernameHistoryScope } from "./usernameHistory";

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
  return <AuthFrame eyebrow="First-run setup" title="Create the first administrator account" summary="Establish an OncoFlow administrator credential. Legacy Access passwords are never accepted.">
    <form className="auth-form" onSubmit={(event) => void submit(event)} noValidate>
      {submitError && <div className="auth-error" role="alert">{submitError}</div>}
      <AuthField label="Username" error={errors.username}><input autoFocus autoComplete="username" value={values.username} onChange={(event) => field("username", event.target.value)} /></AuthField>
      <AuthField label="Display name" error={errors.displayName}><input autoComplete="name" value={values.displayName} onChange={(event) => field("displayName", event.target.value)} /></AuthField>
      <AuthField label="New password" hint="12–128 characters" error={errors.password}><input type="password" autoComplete="new-password" value={values.password} onChange={(event) => field("password", event.target.value)} /></AuthField>
      <AuthField label="Confirm password" error={errors.confirmPassword}><input type="password" autoComplete="new-password" value={values.confirmPassword} onChange={(event) => field("confirmPassword", event.target.value)} /></AuthField>
      <button className="button button--primary auth-submit" type="submit" disabled={busy}>{busy ? "Creating account…" : "Create account"}</button>
      <p className="auth-privacy">No factory password is created. Only a salted password hash is stored in the configured <code>oncoflow.db</code>.</p>
    </form>
  </AuthFrame>;
}

export function LoginScreen({ onAuthenticated, initialError = null }: { onAuthenticated: (state: AuthState) => void; initialError?: string | null }) {
  const historyKey = useContext(UsernameHistoryScope);
  const [usernames, setUsernames] = useState(() => readUsernameHistory(historyKey));
  const [username, setUsername] = useState(() => usernames[0] ?? "");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(initialError);
  const [busy, setBusy] = useState(false);
  const passwordInput = useRef<HTMLInputElement>(null);
  const submissionLock = useRef(false);
  function chooseUsername(value: string) {
    setUsername(value); setPassword(""); setError(null);
  }
  function forgetUsernames() {
    if (!clearUsernameHistory(historyKey)) { setError("Saved usernames could not be cleared. Please try again."); return; }
    setUsernames([]); chooseUsername("");
  }
  async function submit(event: React.FormEvent) {
    event.preventDefault();
    if (submissionLock.current) return;
    if (!username.trim() || !password) { setError("Enter the local username and password."); return; }
    submissionLock.current = true;
    setBusy(true); setError(null);
    try {
      const state = await loginUser({ username: username.trim(), password });
      rememberSuccessfulUsername(historyKey, state);
      setPassword("");
      onAuthenticated(state);
    }
    catch (cause) { setError(commandError(cause).message ?? "The local login was not accepted."); }
    finally { submissionLock.current = false; setBusy(false); }
  }
  return <AuthFrame>
    <form className="auth-form" onSubmit={(event) => void submit(event)} noValidate>
      {error && <div className="auth-error" role="alert">{error}</div>}
      {usernames.length > 0 && <>
        <AuthField label="Saved usernames"><select disabled={busy} value={usernames.includes(username) ? username : ""} onChange={(event) => { chooseUsername(event.target.value); if (event.target.value) passwordInput.current?.focus(); }}><option value="">Use another username</option>{usernames.map((saved) => <option key={saved} value={saved}>{saved}</option>)}</select></AuthField>
        <button className="button button--secondary" type="button" disabled={busy} onClick={forgetUsernames}>Clear saved usernames</button>
      </>}
      <AuthField label="Username"><input autoFocus={usernames.length === 0} autoComplete="username" value={username} disabled={busy} onChange={(event) => chooseUsername(event.target.value)} /></AuthField>
      <AuthField label="Password"><input ref={passwordInput} autoFocus={usernames.length > 0} type="password" autoComplete="current-password" value={password} disabled={busy} onChange={(event) => setPassword(event.target.value)} /></AuthField>
      <button className="button button--primary auth-submit" type="submit" disabled={busy}>{busy ? "Signing in…" : "Sign in"}</button>
      <p className="auth-privacy">Successful usernames are remembered on this device for this connection. Passwords are not saved in this list.</p>
    </form>
  </AuthFrame>;
}

export function AuthFrame({ eyebrow, title, summary, children }: { eyebrow?: string; title?: string; summary?: string; children: React.ReactNode }) {
  const hasHeading = eyebrow || title || summary;
  return <main className="auth-shell"><section className="auth-card"><div className="auth-brand"><div className="brand-mark" aria-hidden="true"><img src={appIcon} alt="" /></div><div><strong>OncoFlow</strong><span>Chemotherapy preparation</span></div></div>{hasHeading && <div className="auth-heading">{eyebrow && <p className="eyebrow">{eyebrow}</p>}{title && <h1>{title}</h1>}{summary && <p>{summary}</p>}</div>}{children}<footer><span className="local-badge__dot" aria-hidden="true"/> Local / LAN · no internet required</footer></section></main>;
}

function AuthField({ label, hint, error, children }: { label: string; hint?: string; error?: string; children: React.ReactNode }) {
  return <label className="auth-field"><span>{label}{hint && <small>{hint}</small>}</span>{children}{error && <b role="alert">{error}</b>}</label>;
}
