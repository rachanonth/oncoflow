import { useEffect, useId, useMemo, useRef, useState } from "react";
import { createPortal } from "react-dom";

import { currentBangkokDateTimeValue, displayDate, displayLocalDateTime } from "../shared/dateTime";

type CommonProps = {
  value: string;
  onChange: (value: string) => void;
  disabled?: boolean;
  invalid?: boolean;
  describedBy?: string;
  placeholder?: string;
  autoFocus?: boolean;
};

export function BuddhistDateInput(props: CommonProps) {
  return <BuddhistCalendarInput {...props} mode="date" />;
}

export function BuddhistDateTimeInput(props: CommonProps) {
  return <BuddhistCalendarInput {...props} mode="datetime" />;
}

function BuddhistCalendarInput({ value, onChange, disabled = false, invalid = false, describedBy, placeholder, autoFocus, mode }: CommonProps & { mode: "date" | "datetime" }) {
  const inputRef = useRef<HTMLInputElement>(null);
  const dialogRef = useRef<HTMLDivElement>(null);
  const titleId = useId();
  const [open, setOpen] = useState(false);
  const [year, setYear] = useState(0);
  const [month, setMonth] = useState(0);
  const [day, setDay] = useState(0);
  const [hour, setHour] = useState("00");
  const [minute, setMinute] = useState("00");

  const current = useMemo(() => parseBangkokValue(currentBangkokDateTimeValue()), []);
  const selected = parseBangkokValue(value);
  const displayValue = value
    ? mode === "date" ? displayDate(value) : displayLocalDateTime(value, value)
    : "";

  const years = useMemo(() => {
    const start = Math.min(current.year - 150, year || current.year);
    const end = Math.max(current.year + 100, year || current.year);
    return Array.from({ length: end - start + 1 }, (_, index) => start + index);
  }, [current.year, year]);

  useEffect(() => {
    if (!open) return;
    dialogRef.current?.focus();
    function escape(event: KeyboardEvent) {
      if (event.key === "Escape") close();
    }
    window.addEventListener("keydown", escape);
    return () => window.removeEventListener("keydown", escape);
  }, [open]);

  function openCalendar() {
    if (disabled) return;
    const initial = selected.valid ? selected : current;
    setYear(initial.year);
    setMonth(initial.month);
    setDay(initial.day);
    setHour(initial.hour);
    setMinute(initial.minute);
    setOpen(true);
  }

  function close() {
    setOpen(false);
    window.requestAnimationFrame(() => inputRef.current?.focus());
  }

  function moveMonth(offset: number) {
    const next = new Date(Date.UTC(year, month - 1 + offset, 1));
    setYear(next.getUTCFullYear());
    setMonth(next.getUTCMonth() + 1);
    setDay(Math.min(day, daysInMonth(next.getUTCFullYear(), next.getUTCMonth() + 1)));
  }

  function selectDay(nextDay: number) {
    setDay(nextDay);
    if (mode === "date") {
      onChange(isoDate(year, month, nextDay));
      close();
    }
  }

  function selectToday() {
    setYear(current.year); setMonth(current.month); setDay(current.day);
    if (mode === "date") {
      onChange(isoDate(current.year, current.month, current.day));
      close();
    } else {
      setHour(current.hour); setMinute(current.minute);
    }
  }

  function applyDateTime() {
    onChange(`${isoDate(year, month, day)}T${hour}:${minute}`);
    close();
  }

  const calendar = open && typeof document !== "undefined" ? createPortal(
    <div className="buddhist-calendar-backdrop" onMouseDown={(event) => { if (event.target === event.currentTarget) close(); }}>
      <div className="buddhist-calendar" role="dialog" aria-modal="true" aria-labelledby={titleId} ref={dialogRef} tabIndex={-1}>
        <div className="buddhist-calendar__heading"><div><p className="eyebrow">Buddhist calendar</p><h2 id={titleId}>{mode === "date" ? "Select date" : "Select date and time"}</h2></div><button type="button" aria-label="Close calendar" onClick={close}>×</button></div>
        <div className="buddhist-calendar__navigation">
          <button type="button" aria-label="Previous month" onClick={() => moveMonth(-1)}>‹</button>
          <select aria-label="Month" value={month} onChange={(event) => { const next = Number(event.target.value); setMonth(next); setDay(Math.min(day, daysInMonth(year, next))); }}>{Array.from({ length: 12 }, (_, index) => <option value={index + 1} key={index + 1}>{String(index + 1).padStart(2, "0")}</option>)}</select>
          <select aria-label="Buddhist year" value={year} onChange={(event) => { const next = Number(event.target.value); setYear(next); setDay(Math.min(day, daysInMonth(next, month))); }}>{years.map((candidate) => <option value={candidate} key={candidate}>{candidate + 543}</option>)}</select>
          <button type="button" aria-label="Next month" onClick={() => moveMonth(1)}>›</button>
        </div>
        <div className="buddhist-calendar__weekdays" aria-hidden="true">{["SUN", "MON", "TUE", "WED", "THU", "FRI", "SAT"].map((label) => <span key={label}>{label}</span>)}</div>
        <div className="buddhist-calendar__days">{calendarCells(year, month).map((candidate, index) => candidate === null ? <span key={`blank-${index}`} /> : <button className={candidate === day ? "is-selected" : undefined} type="button" aria-label={`${String(candidate).padStart(2, "0")}/${String(month).padStart(2, "0")}/${year + 543}`} aria-pressed={candidate === day} onClick={() => selectDay(candidate)} key={candidate}>{candidate}</button>)}</div>
        {mode === "datetime" && <div className="buddhist-calendar__time"><span>Bangkok time (GMT+7)</span><div><select aria-label="Hour, 24-hour format" value={hour} onChange={(event) => setHour(event.target.value)}>{Array.from({ length: 24 }, (_, index) => <option key={index} value={String(index).padStart(2, "0")}>{String(index).padStart(2, "0")}</option>)}</select><b>:</b><select aria-label="Minute" value={minute} onChange={(event) => setMinute(event.target.value)}>{Array.from({ length: 60 }, (_, index) => <option key={index} value={String(index).padStart(2, "0")}>{String(index).padStart(2, "0")}</option>)}</select></div></div>}
        <div className="buddhist-calendar__actions"><button className="button button--secondary button--compact" type="button" onClick={() => { onChange(""); close(); }}>Clear</button><button className="button button--secondary button--compact" type="button" onClick={selectToday}>Today</button><span /><button className="button button--secondary button--compact" type="button" onClick={close}>Cancel</button>{mode === "datetime" && <button className="button button--primary button--compact" type="button" onClick={applyDateTime}>Apply</button>}</div>
      </div>
    </div>,
    document.body,
  ) : null;

  return <span className="buddhist-date-control"><input ref={inputRef} type="text" readOnly autoFocus={autoFocus} value={displayValue} placeholder={placeholder ?? (mode === "date" ? "DD/MM/YYYY" : "DD/MM/YYYY HH:mm")} disabled={disabled} aria-invalid={invalid} aria-describedby={describedBy} aria-haspopup="dialog" aria-expanded={open} onClick={openCalendar} onKeyDown={(event) => { if (event.key === "Enter" || event.key === " ") { event.preventDefault(); openCalendar(); } }} /><span className="buddhist-date-control__icon" aria-hidden="true">▦</span>{calendar}</span>;
}

function parseBangkokValue(value: string): { valid: boolean; year: number; month: number; day: number; hour: string; minute: string } {
  const match = /^(\d{4})-(\d{2})-(\d{2})(?:[T ](\d{2}):(\d{2}))?/.exec(value);
  if (!match) return { valid: false, year: 0, month: 0, day: 0, hour: "00", minute: "00" };
  const year = Number(match[1]); const month = Number(match[2]); const day = Number(match[3]);
  const probe = new Date(Date.UTC(year, month - 1, day));
  const valid = probe.getUTCFullYear() === year && probe.getUTCMonth() === month - 1 && probe.getUTCDate() === day;
  return { valid, year, month, day, hour: match[4] ?? "00", minute: match[5] ?? "00" };
}

function calendarCells(year: number, month: number): Array<number | null> {
  const blanks = new Date(Date.UTC(year, month - 1, 1)).getUTCDay();
  return [...Array.from({ length: blanks }, () => null), ...Array.from({ length: daysInMonth(year, month) }, (_, index) => index + 1)];
}

function daysInMonth(year: number, month: number): number { return new Date(Date.UTC(year, month, 0)).getUTCDate(); }
function isoDate(year: number, month: number, day: number): string { return `${year}-${String(month).padStart(2, "0")}-${String(day).padStart(2, "0")}`; }
