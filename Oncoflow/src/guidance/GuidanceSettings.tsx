import { useEffect, useRef, useState } from "react";

import { commandError } from "../api/commands";
import { PageDescription, usePageGuidance } from "./PageGuidance";
import { PAGE_DESCRIPTIONS, pageDescription, type PageKey } from "./pageDescriptions";

export function GuidanceSettings() {
  const state = usePageGuidance();
  const [selected, setSelected] = useState<PageKey>("patients");
  const [value, setValue] = useState("");
  const [message, setMessage] = useState<{ tone: "success" | "error"; text: string } | null>(null);
  const [busy, setBusy] = useState(false);
  const submissionLock = useRef(false);
  const standard = pageDescription(selected);

  useEffect(() => {
    setValue(state.guidance[selected] ?? "");
    setMessage(null);
  }, [selected, state.guidance]);

  async function persist(next: string | null) {
    if (submissionLock.current) return;
    if (next !== null && [...next.trim()].length > 500) {
      setMessage({ tone: "error", text: "Guidance is limited to 500 characters." });
      return;
    }
    submissionLock.current = true;
    setBusy(true);
    setMessage(null);
    try {
      const saved = await state.save(selected, next);
      setValue(saved.guidance ?? "");
      setMessage({ tone: "success", text: saved.guidance ? "Guidance saved." : "Guidance reset." });
    } catch (error) {
      setMessage({ tone: "error", text: commandError(error).message ?? "Guidance could not be saved." });
    } finally {
      submissionLock.current = false;
      setBusy(false);
    }
  }

  return <section className="workspace guidance-workspace" aria-labelledby="guidance-heading">
    <div className="page-heading"><div><p className="eyebrow">Settings</p><h1 id="guidance-heading">Guidance</h1><PageDescription pageKey="guidance" /></div></div>
    <div className="surface guidance-editor">
      <div className="guidance-editor__heading"><div><p className="eyebrow">Page copy</p><h2>Optional workstation guidance</h2></div><span>{value.trim().length}/500</span></div>
      {state.error && <div className="auth-error" role="alert">{state.error} <button className="button button--compact button--secondary" type="button" onClick={() => void state.reload()}>Retry</button></div>}
      {message && <div className={message.tone === "error" ? "auth-error" : "auth-success"} role={message.tone === "error" ? "alert" : "status"}>{message.text}</div>}
      <label className="form-field"><span className="field-label">Page</span><select value={selected} disabled={busy || state.loading} onChange={(event) => setSelected(event.target.value as PageKey)}>{PAGE_DESCRIPTIONS.map((page) => <option key={page.key} value={page.key}>{page.title}</option>)}</select></label>
      <div className="guidance-standard"><span>Standard description</span><p lang="th">{standard.description}</p></div>
      <label className="form-field"><span className="field-label">Guidance</span><textarea rows={4} maxLength={500} value={value} disabled={busy || state.loading} placeholder="Optional instructions for this workstation" onChange={(event) => { setValue(event.target.value); setMessage(null); }} /><small>Do not enter patient-identifying information or use Guidance to define clinical rules.</small></label>
      <div className="guidance-editor__actions"><button className="button button--secondary" type="button" disabled={busy || !state.guidance[selected]} onClick={() => void persist(null)}>Reset</button><button className="button button--primary" type="button" disabled={busy || state.loading} onClick={() => void persist(value)}>{busy ? "Saving…" : "Save Guidance"}</button></div>
    </div>
  </section>;
}

export function validateGuidance(value: string) {
  return [...value.trim()].length <= 500;
}
