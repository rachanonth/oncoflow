import { useEffect, useState } from "react";

import { commandError, getDrug } from "../api/commands";
import type { DrugDetail as DrugDetailType } from "../types/drug";
import { displayDuration } from "../shared/duration";
import { displayDrugValue, displayFlag, numberWithUnit } from "./format";

interface DrugDetailProps {
  drugId: number;
  onBack: () => void;
  onEdit: (drug: DrugDetailType) => void;
}

type DetailState =
  | { kind: "loading" }
  | { kind: "ready"; drug: DrugDetailType }
  | { kind: "error"; message: string };

export function DrugDetail({ drugId, onBack, onEdit }: DrugDetailProps) {
  const [reloadKey, setReloadKey] = useState(0);
  const [state, setState] = useState<DetailState>({ kind: "loading" });

  useEffect(() => {
    let active = true;
    setState({ kind: "loading" });
    void getDrug(drugId)
      .then((drug) => active && setState({ kind: "ready", drug }))
      .catch((error: unknown) => {
        if (active) {
          setState({
            kind: "error",
            message: commandError(error).message ?? "Unable to load this drug.",
          });
        }
      });
    return () => {
      active = false;
    };
  }, [drugId, reloadKey]);

  if (state.kind === "loading") {
    return (
      <section className="workspace">
        <button className="back-button" type="button" onClick={onBack}>← Drugs</button>
        <div className="detail-loading" aria-label="Loading drug details">
          <div className="skeleton-block skeleton-block--hero" />
          <div className="detail-grid drug-detail-grid"><div className="skeleton-block" /><div className="skeleton-block" /></div>
        </div>
      </section>
    );
  }

  if (state.kind === "error") {
    return (
      <section className="workspace">
        <button className="back-button" type="button" onClick={onBack}>← Drugs</button>
        <div className="state-panel state-panel--error surface" role="alert">
          <span className="state-icon" aria-hidden="true">!</span>
          <h1>Drug details unavailable</h1>
          <p>{state.message}</p>
          <button className="button button--secondary" type="button" onClick={() => setReloadKey((value) => value + 1)}>Try again</button>
        </div>
      </section>
    );
  }

  const { drug } = state;
  return (
    <section className="workspace" aria-labelledby="drug-name-heading">
      <button className="back-button" type="button" onClick={onBack}>← Drugs</button>
      <header className="patient-hero drug-hero">
        <div className="patient-avatar drug-avatar" aria-hidden="true">Rx</div>
        <div className="patient-hero__identity">
          <p className="eyebrow">Drug master record</p>
          <h1 id="drug-name-heading">{drug.name}</h1>
          <div className="identity-chips">
            {drug.unit && <span className="identity-chip"><b>Unit</b> {drug.unit}</span>}
            <span className={`identity-chip ${drug.inventoryEnabled ? "" : "identity-chip--neutral"}`}>
              Inventory {drug.inventoryEnabled ? "enabled" : "disabled"}
            </span>
          </div>
        </div>
        <button className="button button--primary" type="button" onClick={() => onEdit(drug)}>Edit drug</button>
      </header>

      <div className="safety-boundary-note">
        Legacy configuration is displayed as stored. OncoFlow does not calculate or reinterpret these values in this milestone.
      </div>

      <div className="detail-grid drug-detail-grid">
        <DrugSection title="Identity">
          <DrugField label="Drug name" value={drug.name} />
          <DrugField label="Unit" value={drug.unit} />
          <DrugField label="Package" value={drug.package} />
          <DrugField label="Price" value={drug.price} />
        </DrugSection>

        <DrugSection title="Preparation">
          <DrugField label="Dose per pack" value={drug.dosePerPack} />
          <DrugField label="Volume per pack" value={numberWithUnit(drug.volumePerPackMl, "mL")} />
          <DrugField label="Default diluent" value={drug.defaultDiluent} />
          <DrugField label="Default route" value={drug.defaultRoute} />
          <DrugField label="Default rate" value={displayDuration(drug.defaultRate)} />
          <DrugField label="Expiry time" value={displayDuration(drug.expiryTime, true)} />
          <DrugField label="Preparation detail" value={drug.detail} wide preserveLines />
          <DrugField label="Storage" value={drug.storage} wide preserveLines />
          <DrugField label="Expiry storage" value={drug.expiryStorage} wide preserveLines />
        </DrugSection>

        <DrugSection title="Safety configuration">
          <DrugField label="Maximum dose" value={drug.maxDose} />
          <DrugField label="Maximum dilution alert" value={displayFlag(drug.maxDilutionAlert)} />
          <DrugField label="Maximum dilution threshold" value={drug.maxDilutionHard} />
          <DrugField label="Cumulative alert" value={displayFlag(drug.cumulativeAlert)} />
          <DrugField label="Cumulative threshold" value={drug.cumulativeAlertHard} />
          <DrugField label="Drug record" value={drug.marker ? "Enabled" : "Disabled"} />
          <DrugField label="Warning" value={drug.warning} wide preserveLines alert />
          <DrugField label="Dilution incompatibility" value={drug.dilutionIncompatibility} wide preserveLines />
        </DrugSection>

        <DrugSection title="Inventory configuration">
          <DrugField label="Inventory" value={drug.inventoryEnabled ? "Enabled" : "Disabled"} />
          <DrugField label="Cut-off flag" value={displayFlag(drug.inventoryCut)} />
          <DrugField label="Minimum" value={drug.inventoryMin} />
          <DrugField label="Maximum" value={drug.inventoryMax} />
          <DrugField label="Current quantity" value={drug.inventoryQuantity} />
        </DrugSection>

        <DrugSection title="Legacy / metadata">
          <DrugField label="Local record ID" value={drug.id} />
          <DrugField label="Legacy mapping code" value={drug.legacyMappingCode} />
          <DrugField label="Legacy expiry value" value={drug.legacyExp} />
          <DrugField label="Legacy regimen value" value={drug.legacyReg} />
        </DrugSection>
      </div>
    </section>
  );
}

function DrugSection({ title, children }: { title: string; children: React.ReactNode }) {
  return <section className="detail-section"><h2>{title}</h2><dl className="detail-fields">{children}</dl></section>;
}

function DrugField({ label, value, wide = false, emphasized = false, preserveLines = false, alert = false }: { label: string; value: string | number | null; wide?: boolean; emphasized?: boolean; preserveLines?: boolean; alert?: boolean }) {
  return (
    <div className={`${wide ? "is-wide" : ""} ${preserveLines ? "preserve-lines" : ""} ${alert && value ? "detail-alert" : ""}`}>
      <dt>{label}</dt>
      <dd className={emphasized ? "is-emphasized" : undefined}>{displayDrugValue(value)}</dd>
    </div>
  );
}
