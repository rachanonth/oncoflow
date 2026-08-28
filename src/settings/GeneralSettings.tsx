import { useEffect, useState } from "react";

import { commandError, getApplicationSettings, updateApplicationSettings } from "../api/commands";
import { PageDescription } from "../guidance/PageGuidance";

export function GeneralSettings() {
  const [value, setValue] = useState("");
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<{ tone: "success" | "error"; text: string } | null>(null);

  useEffect(() => {
    let active = true;
    void getApplicationSettings()
      .then((settings) => { if (active) setValue(settings.hospitalName ?? ""); })
      .catch((error: unknown) => { if (active) setMessage({ tone: "error", text: commandError(error).message ?? "Application settings could not be loaded." }); })
      .finally(() => { if (active) setLoading(false); });
    return () => { active = false; };
  }, []);

  async function save() {
    const hospitalName = value.trim();
    if (hospitalName.length > 160) {
      setMessage({ tone: "error", text: "Hospital name is limited to 160 characters." });
      return;
    }
    setBusy(true);
    setMessage(null);
    try {
      const saved = await updateApplicationSettings({ hospitalName: hospitalName || null });
      setValue(saved.hospitalName ?? "");
      setMessage({ tone: "success", text: saved.hospitalName ? "Hospital name saved." : "Hospital name cleared." });
    } catch (error) {
      setMessage({ tone: "error", text: commandError(error).message ?? "Application settings could not be saved." });
    } finally {
      setBusy(false);
    }
  }

  return <section className="workspace guidance-workspace" aria-labelledby="general-settings-heading">
    <div className="page-heading"><div><p className="eyebrow">Settings</p><h1 id="general-settings-heading">General</h1><PageDescription pageKey="general" /></div></div>
    <section className="surface guidance-editor">
      <div className="guidance-editor__heading"><div><p className="eyebrow">Organization identity</p><h2>Hospital name</h2></div><span>{value.trim().length} / 160</span></div>
      <label className="form-field"><span className="field-label">Hospital name</span><input value={value} maxLength={160} disabled={loading || busy} placeholder="e.g. โรงพยาบาลตัวอย่าง" onChange={(event) => { setValue(event.target.value); setMessage(null); }} /><small>Displayed after the OncoFlow application name on the Working Formula. Leave blank to show only OncoFlow.</small></label>
      {message && <div className={message.tone === "success" ? "auth-success" : "auth-error"} role={message.tone === "error" ? "alert" : "status"}>{message.text}</div>}
      <div className="guidance-editor__actions"><button className="button button--secondary" type="button" disabled={loading || busy || !value} onClick={() => setValue("")}>Clear</button><button className="button button--primary" type="button" disabled={loading || busy} onClick={() => void save()}>{busy ? "Saving…" : "Save settings"}</button></div>
    </section>
  </section>;
}

export function formatWorkingFormulaAppName(hospitalName: string | null): string {
  const value = hospitalName?.trim();
  return value ? `OncoFlow · ${value}` : "OncoFlow";
}
