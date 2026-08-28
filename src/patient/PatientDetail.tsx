import { useEffect, useState } from "react";

import { commandError, getPatient } from "../api/commands";
import type { PatientDetail as PatientDetailType } from "../types/patient";
import { PatientOrderHistory } from "../order/PatientOrderHistory";
import { displayDate } from "../shared/dateTime";
import {
  detailInitials,
  displayDateTime,
  displayValue,
  patientName,
} from "./format";
import { calculateAgeYears } from "./age";

interface PatientDetailProps {
  patientId: number;
  onBack: () => void;
  onEdit: (patient: PatientDetailType) => void;
  onOpenOrder: (orderId: number) => void;
  onCreateOrder: (patientId: number) => void;
}

type DetailState =
  | { kind: "loading" }
  | { kind: "ready"; patient: PatientDetailType }
  | { kind: "error"; message: string };

export function PatientDetail({ patientId, onBack, onEdit, onOpenOrder, onCreateOrder }: PatientDetailProps) {
  const [reloadKey, setReloadKey] = useState(0);
  const [state, setState] = useState<DetailState>({ kind: "loading" });

  useEffect(() => {
    let active = true;
    setState({ kind: "loading" });
    void getPatient(patientId)
      .then((patient) => active && setState({ kind: "ready", patient }))
      .catch((error: unknown) => {
        if (active) {
          setState({
            kind: "error",
            message: commandError(error).message ?? "Unable to load this patient.",
          });
        }
      });
    return () => {
      active = false;
    };
  }, [patientId, reloadKey]);

  if (state.kind === "loading") {
    return (
      <section className="workspace">
        <button className="back-button" type="button" onClick={onBack}>← Patients</button>
        <div className="detail-loading" aria-label="Loading patient details">
          <div className="skeleton-block skeleton-block--hero" />
          <div className="detail-grid">
            <div className="skeleton-block" />
            <div className="skeleton-block" />
          </div>
        </div>
      </section>
    );
  }

  if (state.kind === "error") {
    return (
      <section className="workspace">
        <button className="back-button" type="button" onClick={onBack}>← Patients</button>
        <div className="state-panel state-panel--error surface" role="alert">
          <span className="state-icon" aria-hidden="true">!</span>
          <h1>Patient details unavailable</h1>
          <p>{state.message}</p>
          <button
            className="button button--secondary"
            type="button"
            onClick={() => setReloadKey((value) => value + 1)}
          >
            Try again
          </button>
        </div>
      </section>
    );
  }

  const { patient } = state;
  const ageYears = calculateAgeYears(patient.birthDate) ?? patient.ageYears;
  return (
    <section className="workspace" aria-labelledby="patient-name-heading">
      <button className="back-button" type="button" onClick={onBack}>← Patients</button>

      <header className="patient-hero patient-detail-hero">
        <div className="patient-avatar" aria-hidden="true">{detailInitials(patient)}</div>
        <div className="patient-hero__identity">
          <p className="eyebrow">Patient record</p>
          <h1 id="patient-name-heading">{patientName(patient)}</h1>
          <div className="identity-chips">
            <span className="identity-chip"><b>HN</b> {patient.hn}</span>
            {patient.cancerNo && <span className="identity-chip"><b>CA</b> {patient.cancerNo}</span>}
            {patient.treatmentEnded === true && (
              <span className="identity-chip identity-chip--neutral">Treatment ended</span>
            )}
          </div>
        </div>
        <button className="button button--primary" type="button" onClick={() => onEdit(patient)}>
          Edit patient
        </button>
      </header>

      <div className="detail-grid patient-detail-grid">
        <div className="patient-detail-column">
          <DetailSection title="Identity">
            <DetailField label="HN" value={patient.hn} emphasized />
            <DetailField label="CA number" value={patient.cancerNo} />
            <DetailField label="Title" value={patient.title} />
            <DetailField label="First name" value={patient.firstName} />
            <DetailField label="Last name" value={patient.lastName} />
            <DetailField label="Sex" value={patient.sex ?? "Not specified"} />
            <DetailField label="Birth date" value={patient.birthDate ? displayDate(patient.birthDate) : null} />
            <DetailField label="Age" value={ageYears === null ? null : `${ageYears} years`} />
            <DetailField label="Telephone" value={patient.telephone} />
          </DetailSection>

          <DetailSection title="Measurements">
            <DetailField
              label="Weight"
              value={patient.weightKg === null ? null : `${patient.weightKg} kg`}
            />
            <DetailField
              label="Height"
              value={patient.heightCm === null ? null : `${patient.heightCm} cm`}
            />
          </DetailSection>
        </div>

        <div className="patient-detail-column">
          <DetailSection title="Clinical">
            <DetailField label="Diagnosis" value={patient.diagnosis} wide />
            <DetailField label="Regimen" value={patient.regimen} wide />
            <DetailField label="Stage" value={patient.stage} />
            <DetailField label="HER2" value={patient.her2} />
            <DetailField label="ER/PR" value={patient.erpr} />
            <DetailField
              label="Counselling"
              value={patient.counselling ? "Recorded" : "Not recorded"}
            />
            <DetailField label="Allergy" value={patient.allergy} wide preserveLines />
          </DetailSection>

          <DetailSection title="Other information">
            <DetailField label="Occupation" value={patient.occupation} />
            <DetailField label="Address" value={patient.address} wide preserveLines />
            <DetailField label="Patient history" value={patient.patientHistory} wide preserveLines />
            <DetailField
              label="Treatment end date"
              value={patient.treatmentEndDate ? displayDate(patient.treatmentEndDate) : null}
            />
          </DetailSection>
        </div>
      </div>

      <PatientOrderHistory
        patientId={patient.id}
        onOpen={onOpenOrder}
        onCreate={() => onCreateOrder(patient.id)}
      />

      <footer className="record-metadata">
        <span>Last recorded {displayDateTime(patient.recordTime)}</span>
        {patient.recordBy && <span>Recorded by {patient.recordBy}</span>}
        <span>Local record ID {patient.id}</span>
      </footer>
    </section>
  );
}

function DetailSection({
  title,
  children,
}: {
  title: string;
  children: React.ReactNode;
}) {
  return (
    <section className="detail-section">
      <h2>{title}</h2>
      <dl className="detail-fields">{children}</dl>
    </section>
  );
}

function DetailField({
  label,
  value,
  wide = false,
  emphasized = false,
  preserveLines = false,
}: {
  label: string;
  value: string | number | null;
  wide?: boolean;
  emphasized?: boolean;
  preserveLines?: boolean;
}) {
  const isEmpty = value === null || value === "";
  return (
    <div className={`${wide ? "is-wide" : ""} ${preserveLines ? "preserve-lines" : ""} ${isEmpty ? "is-empty" : ""}`}>
      <dt>{label}</dt>
      <dd className={emphasized ? "is-emphasized" : undefined}>{displayValue(value)}</dd>
    </div>
  );
}
