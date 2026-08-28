import { useCallback, useEffect, useRef, useState } from "react";

import { commandError, createUser, listUsers, updateUser } from "../api/commands";
import { PageDescription } from "../guidance/PageGuidance";
import type { ManagedUser, UserRole, UserType } from "../types/auth";

type UserFormValues = {
  username: string;
  displayName: string;
  password: string;
  confirmPassword: string;
  userType: UserType;
  role: UserRole;
  active: boolean;
};

type UserFormErrors = Partial<Record<keyof UserFormValues, string>>;

const EMPTY_USER: UserFormValues = {
  username: "",
  displayName: "",
  password: "",
  confirmPassword: "",
  userType: "pharmacist",
  role: "pharmacist",
  active: true,
};

export function UserManagement({ currentUserId }: { currentUserId: number }) {
  const [users, setUsers] = useState<ManagedUser[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [editing, setEditing] = useState<ManagedUser | null>(null);
  const [formOpen, setFormOpen] = useState(false);
  const [values, setValues] = useState<UserFormValues>(EMPTY_USER);
  const [errors, setErrors] = useState<UserFormErrors>({});
  const [message, setMessage] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const submissionLock = useRef(false);

  const load = useCallback(async () => {
    setLoading(true);
    setLoadError(null);
    try {
      setUsers(await listUsers());
    } catch (error) {
      setUsers([]);
      setLoadError(commandError(error).message ?? "Local users could not be loaded.");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { void load(); }, [load]);

  function beginCreate() {
    setEditing(null);
    setValues(EMPTY_USER);
    setErrors({});
    setMessage(null);
    setFormOpen(true);
  }

  function beginEdit(user: ManagedUser) {
    if (user.id === currentUserId) return;
    setEditing(user);
    setValues({
      username: user.username,
      displayName: user.displayName,
      password: "",
      confirmPassword: "",
      userType: user.userType,
      role: user.role,
      active: user.active,
    });
    setErrors({});
    setMessage(null);
    setFormOpen(true);
  }

  function field<K extends keyof UserFormValues>(name: K, value: UserFormValues[K]) {
    setValues((current) => ({ ...current, [name]: value }));
    setErrors((current) => ({ ...current, [name]: undefined }));
    setMessage(null);
  }

  async function submit(event: React.FormEvent) {
    event.preventDefault();
    const nextErrors = validateManagedUser(values, Boolean(editing));
    setErrors(nextErrors);
    if (Object.keys(nextErrors).length > 0 || submissionLock.current) return;
    submissionLock.current = true;
    setBusy(true);
    setMessage(null);
    try {
      if (editing) {
        await updateUser(editing.id, {
          username: values.username.trim(),
          displayName: values.displayName.trim(),
          userType: values.userType,
          role: values.role,
          active: values.active,
        });
        setMessage("User updated.");
      } else {
        await createUser({
          username: values.username.trim(),
          displayName: values.displayName.trim(),
          password: values.password,
          userType: values.userType,
        });
        setMessage("Local user created.");
      }
      setUsers(await listUsers());
      setFormOpen(false);
      setEditing(null);
      setValues(EMPTY_USER);
    } catch (error) {
      const parsed = commandError(error);
      if (parsed.field && parsed.field in values) {
        setErrors((current) => ({ ...current, [parsed.field!]: parsed.message ?? "Invalid value." }));
      } else {
        setMessage(parsed.message ?? "The local user could not be saved.");
      }
    } finally {
      submissionLock.current = false;
      setBusy(false);
    }
  }

  return <section className="workspace users-workspace" aria-labelledby="users-heading">
    <div className="page-heading">
      <div><p className="eyebrow">Settings</p><h1 id="users-heading">Users</h1><PageDescription pageKey="users" /></div>
      <button className="button button--primary" type="button" onClick={beginCreate} disabled={busy}>Add user</button>
    </div>
    <p className="users-boundary-note">User type distinguishes pharmacists from assistant pharmacists. Both may order and check preparation; only pharmacist accounts can be assigned as the preparation pharmacist.</p>
    {loadError && <div className="form-error-summary" role="alert">{loadError} <button className="button button--compact button--secondary" type="button" onClick={() => void load()}>Retry</button></div>}
    {message && <div className="auth-success" role="status">{message}</div>}
    {formOpen && <UserEditor
      editing={editing}
      values={values}
      errors={errors}
      busy={busy}
      onField={field}
      onCancel={() => { if (!busy) { setFormOpen(false); setEditing(null); setErrors({}); } }}
      onSubmit={(event) => void submit(event)}
    />}
    <UserTable users={users} currentUserId={currentUserId} loading={loading} onEdit={beginEdit} />
  </section>;
}

function UserEditor({ editing, values, errors, busy, onField, onCancel, onSubmit }: {
  editing: ManagedUser | null;
  values: UserFormValues;
  errors: UserFormErrors;
  busy: boolean;
  onField: <K extends keyof UserFormValues>(name: K, value: UserFormValues[K]) => void;
  onCancel: () => void;
  onSubmit: (event: React.FormEvent) => void;
}) {
  return <section className="surface user-editor" aria-labelledby="user-editor-heading">
    <div><p className="eyebrow">{editing ? "Manage account" : "New local account"}</p><h2 id="user-editor-heading">{editing ? editing.displayName : "Add user"}</h2></div>
    <form onSubmit={onSubmit} noValidate>
      <UserField label="Username" error={errors.username}><input autoComplete="off" value={values.username} disabled={busy} onChange={(event) => onField("username", event.target.value)} /></UserField>
      <UserField label="Display name" error={errors.displayName}><input autoComplete="off" value={values.displayName} disabled={busy} onChange={(event) => onField("displayName", event.target.value)} /></UserField>
      <UserField label="User type" error={errors.userType}><select value={values.userType} disabled={busy} onChange={(event) => onField("userType", event.target.value === "non_pharmacist" ? "non_pharmacist" : "pharmacist")}><option value="pharmacist">Pharmacist</option><option value="non_pharmacist">Assistant pharmacist</option></select></UserField>
      {editing && <UserField label="Access level" error={errors.role}><AccessLevelSelect value={values.role} disabled={busy} onChange={(role) => onField("role", role)} /></UserField>}
      {!editing && <UserField label="Initial password" error={errors.password}><input type="password" autoComplete="new-password" value={values.password} disabled={busy} onChange={(event) => onField("password", event.target.value)} /></UserField>}
      {!editing && <UserField label="Confirm password" error={errors.confirmPassword}><input type="password" autoComplete="new-password" value={values.confirmPassword} disabled={busy} onChange={(event) => onField("confirmPassword", event.target.value)} /></UserField>}
      {editing && <label className="checkbox-field is-wide"><input type="checkbox" checked={values.active} disabled={busy} onChange={(event) => onField("active", event.target.checked)} />Account active</label>}
      <div className="user-editor__actions"><button className="button button--secondary" type="button" disabled={busy} onClick={onCancel}>Cancel</button><button className="button button--primary" type="submit" disabled={busy}>{busy ? "Saving…" : editing ? "Save user" : "Create user"}</button></div>
    </form>
  </section>;
}

export function UserTable({ users, currentUserId, loading, onEdit }: {
  users: ManagedUser[];
  currentUserId: number;
  loading: boolean;
  onEdit: (user: ManagedUser) => void;
}) {
  if (loading) return <div className="detail-loading" aria-busy="true">Loading local users…</div>;
  if (users.length === 0) return <div className="empty-state"><h2>No manageable users</h2><p>Create the first additional local account.</p></div>;
  return <div className="list-card"><div className="table-scroll"><table className="patient-table users-table"><thead><tr><th>Name</th><th>Username</th><th>User type</th><th>Access</th><th>Status</th><th aria-label="Actions" /></tr></thead><tbody>{users.map((user) => <tr key={user.id}><td><strong>{user.displayName}</strong>{user.id === currentUserId && <span className="row-subtitle">Current account</span>}</td><td>@{user.username}</td><td><span className={`user-type user-type--${user.userType}`}>{user.userType === "pharmacist" ? "Pharmacist" : "Assistant pharmacist"}</span></td><td>{user.role === "admin" ? "Administrator" : "Standard"}</td><td><span className={user.active ? "status-badge status-badge--active" : "status-badge status-badge--inactive"}>{user.active ? "Active" : "Inactive"}</span></td><td><button className="row-action" type="button" disabled={user.id === currentUserId} title={user.id === currentUserId ? "Manage your password under Account" : "Edit user"} onClick={() => onEdit(user)}>Edit</button></td></tr>)}</tbody></table></div></div>;
}

function UserField({ label, error, children }: { label: string; error?: string; children: React.ReactNode }) {
  return <label className="form-field"><span className="field-label">{label}</span>{children}{error && <span className="field-error">{error}</span>}</label>;
}

export function AccessLevelSelect({ value, disabled, onChange }: { value: UserRole; disabled: boolean; onChange: (role: UserRole) => void }) {
  return <select value={value} disabled={disabled} onChange={(event) => onChange(event.target.value === "admin" ? "admin" : "pharmacist")}><option value="pharmacist">Standard</option><option value="admin">Administrator</option></select>;
}

export function validateManagedUser(values: UserFormValues, editing: boolean): UserFormErrors {
  const errors: UserFormErrors = {};
  const username = values.username.trim();
  const displayName = values.displayName.trim();
  if (username.length < 3 || username.length > 64 || /\s/.test(username)) errors.username = "Use 3–64 characters without spaces.";
  if (!displayName || [...displayName].length > 100) errors.displayName = "Display name is required and limited to 100 characters.";
  if (!editing) {
    if ([...values.password].length < 12 || [...values.password].length > 128) errors.password = "Password must be 12–128 characters.";
    else if (values.password.toLocaleLowerCase() === username.toLocaleLowerCase()) errors.password = "Password must differ from the username.";
    if (values.confirmPassword !== values.password) errors.confirmPassword = "Passwords do not match.";
  }
  return errors;
}
