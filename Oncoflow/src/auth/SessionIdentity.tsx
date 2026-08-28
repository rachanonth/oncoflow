import type { CurrentUser } from "../types/auth";

export function SessionIdentity({ user, busy = false, error = null, onLogout }: { user: CurrentUser; busy?: boolean; error?: string | null; onLogout: () => void }) {
  return <div className="session-identity">
    <div><strong>{user.displayName}</strong><span>{user.userType === "pharmacist" ? "Pharmacist" : "Assistant pharmacist"}{user.role === "admin" ? " · Admin" : ""}</span></div>
    <button type="button" disabled={busy} aria-label="Sign out" title="Sign out" onClick={onLogout}>{busy ? "Signing out…" : "Sign out"}</button>
    {error && <small role="alert">{error}</small>}
  </div>;
}
