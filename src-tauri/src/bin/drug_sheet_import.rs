//! Offline, explicit drug-master update from a reviewed Google Sheets values snapshot.
//! Never imports stock balances, deletes drugs, or changes existing identifiers.
use clap::Parser;
use rusqlite::{params_from_iter, types::Value, Connection, OpenFlags};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    time::Duration,
};

#[derive(Parser)]
struct Args {
    #[arg(long)]
    database: PathBuf,
    #[arg(long)]
    input: PathBuf,
    #[arg(long)]
    source_url: String,
    /// Reviewed identity conflicts to withhold, using source dcode values.
    #[arg(long, value_delimiter = ',')]
    skip_codes: Vec<String>,
    /// Explicit source-code=target-code decisions. Existing target IDs are preserved.
    #[arg(long)]
    map_code: Vec<String>,
    #[arg(long)]
    apply: bool,
    /// Required for apply; must not exist. A SQLite backup is verified before writing.
    #[arg(long)]
    backup: Option<PathBuf>,
}

const HEADERS: [&str; 32] = [
    "dcode",
    "dname",
    "unitcode",
    "dose/pack",
    "vol/pack",
    "package",
    "Detail",
    "price",
    "theory",
    "marker",
    "exp",
    "dfdiluent",
    "dfroute",
    "reg",
    "warning",
    "storage",
    "dfrate",
    "flag",
    "exptime",
    "expstore",
    "Maxdose",
    "MaxDilAlert",
    "MaxDilH",
    "CumAlert",
    "CumAlertH",
    "Dil_Incompat",
    "Inv",
    "InvCut",
    "InvMin",
    "InvMax",
    "InvUse",
    "HOMCCode",
];
// Inv is deliberately excluded: modern inventory comes from inventory_movements.
const FIELDS: [(usize, &str, &str); 30] = [
    (1, "drug_name", "text"),
    (2, "unit_id", "unit"),
    (3, "dose_per_pack", "number"),
    (4, "volume_per_pack_ml", "number"),
    (5, "package", "text"),
    (6, "detail", "text"),
    (7, "price", "number"),
    (8, "theory", "text"),
    (9, "marker", "bool"),
    (10, "legacy_exp", "integer"),
    (11, "default_diluent_id", "diluent"),
    (12, "default_route_id", "route"),
    (13, "legacy_reg", "text"),
    (14, "warning", "text"),
    (15, "storage", "text"),
    (16, "default_rate", "text"),
    (17, "flag", "bool"),
    (18, "expiry_time", "time"),
    (19, "expiry_storage", "text"),
    (20, "max_dose", "number"),
    (21, "max_dilution_alert", "bool"),
    (22, "max_dilution_hard", "number"),
    (23, "cumulative_alert", "bool"),
    (24, "cumulative_alert_hard", "number"),
    (25, "dilution_incompatibility", "text"),
    (27, "inventory_cut", "bool"),
    (28, "inventory_min", "number"),
    (29, "inventory_max", "number"),
    (30, "inventory_enabled", "bool"),
    (31, "homc_code", "text"),
];

fn key(s: &str) -> String {
    let s = s.trim();
    if !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()) {
        let trimmed = s.trim_start_matches('0');
        if trimmed.is_empty() {
            "0".into()
        } else {
            trimmed.into()
        }
    } else {
        s.to_lowercase()
    }
}

fn lookup(
    c: &Connection,
    table: &str,
    column: &str,
) -> Result<HashMap<String, i64>, Box<dyn std::error::Error>> {
    let mut result = HashMap::new();
    let mut stmt = c.prepare(&format!(
        "SELECT id,{column} FROM {table} WHERE {column} IS NOT NULL"
    ))?;
    for row in stmt.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))? {
        let (id, code) = row?;
        if result.insert(key(&code), id).is_some() {
            return Err(format!("ambiguous normalized code in {table}").into());
        }
    }
    Ok(result)
}

fn convert(
    s: &str,
    kind: &str,
    maps: &HashMap<&str, HashMap<String, i64>>,
) -> Result<Value, String> {
    let s = s.trim();
    if s.is_empty() {
        return Ok(Value::Null);
    }
    match kind {
        "text" => Ok(Value::Text(s.into())),
        "bool" => match s {
            "Yes" => Ok(Value::Integer(1)),
            "No" => Ok(Value::Integer(0)),
            _ => Err("expected Yes or No".into()),
        },
        "number" | "integer" => {
            let cleaned = s.strip_prefix('$').unwrap_or(s).replace(',', "");
            let n = cleaned.parse::<f64>().map_err(|_| "invalid number")?;
            if !n.is_finite() || n < 0.0 {
                return Err("expected finite nonnegative number".into());
            }
            if kind == "integer" {
                if n.fract() != 0.0 || n >= i64::MAX as f64 {
                    return Err("invalid integer".into());
                }
                Ok(Value::Integer(n as i64))
            } else {
                Ok(Value::Real(n))
            }
        }
        "time" => {
            let parts = s
                .split(':')
                .map(str::parse::<u32>)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| "invalid time")?;
            if !(2..=3).contains(&parts.len())
                || parts[0] > 23
                || parts[1] > 59
                || parts.get(2).is_some_and(|s| *s > 59)
            {
                return Err("invalid time".into());
            }
            Ok(Value::Text(format!(
                "{:02}:{:02}:{:02}",
                parts[0],
                parts[1],
                parts.get(2).unwrap_or(&0)
            )))
        }
        kind => maps
            .get(kind)
            .and_then(|m| m.get(&key(s)))
            .copied()
            .map(Value::Integer)
            .ok_or_else(|| format!("unknown {kind} code {s}")),
    }
}

fn validate(c: &Connection) -> Result<(), Box<dyn std::error::Error>> {
    let integrity: String = c.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
    let violations: i64 =
        c.query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |r| {
            r.get(0)
        })?;
    if integrity != "ok" || violations != 0 {
        return Err("database validation failed".into());
    }
    Ok(())
}

fn run(a: Args) -> Result<(), Box<dyn std::error::Error>> {
    let bytes = std::fs::read(&a.input)?;
    let rows: Vec<Vec<String>> = serde_json::from_slice(&bytes)?;
    if rows
        .first()
        .map(|r| r.iter().map(String::as_str).collect::<Vec<_>>())
        != Some(HEADERS.to_vec())
    {
        return Err("unexpected sheet headers".into());
    }
    let flags = if a.apply {
        OpenFlags::SQLITE_OPEN_READ_WRITE
    } else {
        OpenFlags::SQLITE_OPEN_READ_ONLY
    };
    let mut c = Connection::open_with_flags(&a.database, flags)?;
    c.busy_timeout(Duration::from_secs(5))?;
    c.pragma_update(None, "foreign_keys", true)?;
    validate(&c)?;
    let tx = c.transaction_with_behavior(if a.apply {
        rusqlite::TransactionBehavior::Immediate
    } else {
        rusqlite::TransactionBehavior::Deferred
    })?;
    let drug_ids = lookup(&tx, "drugs", "legacy_dcode")?;
    let maps = HashMap::from([
        ("unit", lookup(&tx, "units", "legacy_unitcode")?),
        ("route", lookup(&tx, "routes", "legacy_rcode")?),
        ("diluent", lookup(&tx, "diluents", "legacy_dilcode")?),
    ]);
    let overrides = a
        .map_code
        .iter()
        .map(|s| {
            s.split_once('=')
                .map(|(s, t)| (key(s), t.to_owned()))
                .ok_or("map-code requires source=target")
        })
        .collect::<Result<HashMap<_, _>, _>>()?;
    let skips = a.skip_codes.iter().map(|s| key(s)).collect::<HashSet<_>>();
    let mut seen = HashSet::new();
    let mut targets = HashSet::new();
    let mut skipped = Vec::new();
    let mut plans = Vec::new();
    for (index, row) in rows.iter().enumerate().skip(1) {
        if row.iter().all(|s| s.trim().is_empty()) {
            continue;
        }
        if row.len() > 32 {
            return Err(format!("row {} has extra columns", index + 1).into());
        }
        let cell = |i: usize| row.get(i).map(|s| s.trim()).unwrap_or("");
        let source = cell(0);
        if source.is_empty() || !seen.insert(key(source)) {
            return Err("missing or duplicate source drug code".into());
        }
        let reason = if cell(1).is_empty() {
            Some("missing drug name")
        } else if skips.contains(&key(source)) {
            Some("withheld for identity review")
        } else {
            None
        };
        if let Some(reason) = reason {
            skipped.push(json!({"row":index+1,"code":source,"reason":reason}));
            continue;
        }
        let target = overrides
            .get(&key(source))
            .map(String::as_str)
            .unwrap_or(source);
        if !targets.insert(key(target)) {
            return Err("multiple source rows target the same drug".into());
        }
        let mut values = Vec::new();
        let mut errors = Vec::new();
        for (i, _, kind) in FIELDS {
            match convert(cell(i), kind, &maps) {
                Ok(v) => values.push(v),
                Err(e) => errors.push(format!("{}: {e}", HEADERS[i])),
            }
        }
        if !errors.is_empty() {
            skipped.push(json!({"row":index+1,"code":source,"reason":errors}));
            continue;
        }
        if let (Value::Real(min), Value::Real(max)) = (&values[26], &values[27]) {
            if min > max {
                return Err(format!("inventory minimum exceeds maximum for {source}").into());
            }
        }
        plans.push((
            source.to_owned(),
            target.to_owned(),
            drug_ids.get(&key(target)).copied(),
            values,
        ));
    }
    let digest = format!("{:x}", Sha256::digest(&bytes));
    if a.apply && !plans.is_empty() {
        let path = a
            .backup
            .as_ref()
            .ok_or("--backup is required with --apply")?;
        // Reserve a new file, never overwrite an existing recovery snapshot.
        std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(path)?;
        let mut backup = Connection::open(path)?;
        // Separate read connection sees the same committed state protected by BEGIN IMMEDIATE.
        let source = Connection::open_with_flags(&a.database, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        rusqlite::backup::Backup::new(&source, &mut backup)?.run_to_completion(
            64,
            Duration::from_millis(10),
            None,
        )?;
        validate(&backup)?;
        for (source, target, id, values) in &plans {
            let mut params = values.clone();
            let id = if let Some(id) = id {
                params.push(Value::Integer(*id));
                let setters = FIELDS
                    .iter()
                    .map(|(_, col, _)| format!("{col}=?"))
                    .collect::<Vec<_>>()
                    .join(",");
                tx.execute(
                    &format!("UPDATE drugs SET {setters} WHERE id=?"),
                    params_from_iter(params.iter()),
                )?;
                *id
            } else {
                params.push(Value::Text(target.clone()));
                let cols = FIELDS
                    .iter()
                    .map(|(_, col, _)| *col)
                    .chain(["legacy_dcode"])
                    .collect::<Vec<_>>()
                    .join(",");
                let placeholders = vec!["?"; params.len()].join(",");
                tx.execute(
                    &format!("INSERT INTO drugs ({cols}) VALUES ({placeholders})"),
                    params_from_iter(params.iter()),
                )?;
                tx.last_insert_rowid()
            };
            tx.execute("INSERT INTO audit_events(event_type,entity_type,entity_id,metadata_json) VALUES('drug_sheet_import','drug',?1,?2)",rusqlite::params![id.to_string(),json!({"source_url":a.source_url,"source_sha256":digest,"source_code":source}).to_string()])?;
        }
        validate(&tx)?;
        tx.commit()?;
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "applied":a.apply,"source_sha256":digest,"source_url":a.source_url,
            "updates":plans.iter().filter(|p|p.2.is_some()).count(),
            "inserts":plans.iter().filter(|p|p.2.is_none()).count(),
            "skipped":skipped,"backup":a.backup,
            "inventory":"existing balances and movements preserved; no sheet Inv values imported"
        }))?
    );
    Ok(())
}

fn main() {
    if let Err(e) = run(Args::parse()) {
        eprintln!("Drug import failed: {e}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn import_preserves_identity_stock_and_links_and_rolls_back_on_failure() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("oncoflow.db");
        let c = Connection::open(&db).unwrap();
        c.execute_batch(include_str!("../../../migrations/001_initial.sql"))
            .unwrap();
        c.execute_batch(include_str!(
            "../../../migrations/002_import_compatibility.sql"
        ))
        .unwrap();
        c.execute_batch("CREATE TABLE audit_events(event_type TEXT,entity_type TEXT,entity_id TEXT,metadata_json TEXT);
            INSERT INTO units VALUES(7,'001','synthetic unit');
            INSERT INTO drugs(id,legacy_dcode,drug_name,inventory_qty) VALUES(42,'01','Synthetic old',12);
            INSERT INTO drug_detail_groups(drug_id,note) VALUES(42,'synthetic link');").unwrap();
        let mut row = vec![String::new(); 32];
        row[0] = "1".into();
        row[1] = "Synthetic updated".into();
        row[2] = "1".into();
        row[26] = "-999".into();
        let mut new = row.clone();
        new[0] = "2".into();
        new[1] = "Synthetic new".into();
        let input = dir.path().join("input.json");
        std::fs::write(
            &input,
            serde_json::to_vec(&vec![
                HEADERS.iter().map(|s| s.to_string()).collect(),
                row,
                new,
            ])
            .unwrap(),
        )
        .unwrap();
        let backup = dir.path().join("backup.db");
        let args = |apply, backup| Args {
            database: db.clone(),
            input: input.clone(),
            source_url: "synthetic-source".into(),
            skip_codes: vec![],
            map_code: vec![],
            apply,
            backup,
        };
        run(args(false, None)).unwrap();
        assert_eq!(
            c.query_row("SELECT drug_name FROM drugs WHERE id=42", [], |r| r
                .get::<_, String>(0))
                .unwrap(),
            "Synthetic old"
        );
        // Failure after the first update must roll back the entire transaction.
        c.execute_batch("CREATE TRIGGER synthetic_failure BEFORE INSERT ON drugs BEGIN SELECT RAISE(ABORT,'synthetic failure'); END;").unwrap();
        assert!(run(args(true, Some(backup.clone()))).is_err());
        assert_eq!(
            c.query_row("SELECT drug_name FROM drugs WHERE id=42", [], |r| r
                .get::<_, String>(0))
                .unwrap(),
            "Synthetic old"
        );
        assert_eq!(
            c.query_row("SELECT COUNT(*) FROM audit_events", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
        let b = Connection::open(&backup).unwrap();
        validate(&b).unwrap();
        c.execute_batch("DROP TRIGGER synthetic_failure").unwrap();
        run(args(true, Some(dir.path().join("success.db")))).unwrap();
        let actual = c
            .query_row(
                "SELECT legacy_dcode,drug_name,unit_id,inventory_qty FROM drugs WHERE id=42",
                [],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, i64>(2)?,
                        r.get::<_, f64>(3)?,
                    ))
                },
            )
            .unwrap();
        assert_eq!(actual, ("01".into(), "Synthetic updated".into(), 7, 12.0));
        assert_eq!(
            c.query_row("SELECT drug_id FROM drug_detail_groups", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            42
        );
        assert_eq!(
            c.query_row(
                "SELECT inventory_qty FROM drugs WHERE legacy_dcode='2'",
                [],
                |r| r.get::<_, Option<f64>>(0)
            )
            .unwrap(),
            None
        );
        run(args(true, Some(dir.path().join("repeat.db")))).unwrap();
        assert_eq!(
            c.query_row("SELECT COUNT(*) FROM drugs", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            2
        );
    }
    #[test]
    fn normalize_codes_without_confusing_internal_ids() {
        assert_eq!(key("001"), key("1"));
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch("CREATE TABLE diluents(id INTEGER,legacy_dilcode TEXT); INSERT INTO diluents VALUES(14,'016');").unwrap();
        let maps = HashMap::from([("diluent", lookup(&c, "diluents", "legacy_dilcode").unwrap())]);
        assert_eq!(convert("16", "diluent", &maps).unwrap(), Value::Integer(14));
        assert!(convert("59", "diluent", &maps).is_err());
    }
    #[test]
    fn preserves_source_values_without_recalculating() {
        let maps = HashMap::new();
        assert_eq!(
            convert("$1,200.00", "number", &maps).unwrap(),
            Value::Real(1200.0)
        );
        assert_eq!(
            convert("8:00", "time", &maps).unwrap(),
            Value::Text("08:00:00".into())
        );
        assert_eq!(convert("", "number", &maps).unwrap(), Value::Null);
        assert_eq!(convert("Yes", "bool", &maps).unwrap(), Value::Integer(1));
        assert!(convert("NaN", "number", &maps).is_err());
        assert!(convert("-1", "number", &maps).is_err());
        assert!(convert("25:00", "time", &maps).is_err());
        assert!(FIELDS.iter().all(|(i, _, _)| *i != 26));
    }
    #[test]
    fn ambiguous_legacy_codes_fail() {
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch("CREATE TABLE units(id INTEGER,legacy_unitcode TEXT); INSERT INTO units VALUES(1,'01'),(2,'1');").unwrap();
        assert!(lookup(&c, "units", "legacy_unitcode").is_err());
    }
}
