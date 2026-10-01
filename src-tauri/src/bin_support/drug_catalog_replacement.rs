//! Explicit maintenance-only replacement; historical data stays in the recovery backup.
use super::{convert, key, validate, Args};
use rusqlite::{params, params_from_iter, types::Value, Connection, OpenFlags};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    fs::OpenOptions,
    time::Duration,
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
const HEADERS: [&str; 28] = [
    "dcode",
    "dname",
    "unitcode",
    "unitdose",
    "dose/pack",
    "Volume per Pack (mL)",
    "package",
    "Detail",
    "theory (conc./mL)",
    "marker",
    "exp            (หลังผสม)      วัน",
    "exptime    (หลังผสม)                (ชั่วโมง)",
    "dfdiluent",
    "Default Diluent",
    "dfroute",
    "Default Route",
    "reg",
    "warning",
    "storage",
    "dfrate",
    "flag",
    "expstore >หลัง recon/เปิดใช้",
    "Maxdose",
    "MaxDilAlert",
    "MaxDilH",
    "CumAlert แจ้งเตือน doseสะสม",
    "CumAlertH",
    "Dil_Incompat",
];
const FIELDS: [(usize, &str, &str); 22] = [
    (0, "legacy_dcode", "text"),
    (1, "drug_name", "text"),
    (4, "dose_per_pack", "number"),
    (5, "volume_per_pack_ml", "number"),
    (6, "package", "text"),
    (7, "detail", "text"),
    (8, "theory", "text"),
    (9, "marker", "bool"),
    (10, "legacy_exp", "integer"),
    (11, "expiry_time", "time"),
    (16, "legacy_reg", "text"),
    (17, "warning", "text"),
    (18, "storage", "text"),
    (19, "default_rate", "text"),
    (20, "flag", "bool"),
    (21, "expiry_storage", "text"),
    (22, "max_dose", "number"),
    (23, "max_dilution_alert", "bool"),
    (24, "max_dilution_hard", "number"),
    (25, "cumulative_alert", "bool"),
    (26, "cumulative_alert_hard", "number"),
    (27, "dilution_incompatibility", "text"),
];
fn label(s: &str) -> String {
    s.trim().trim_end_matches('.').to_lowercase()
}

fn named_lookup(c: &Connection, table: &str, column: &str, name: &str) -> Result<Option<i64>> {
    if name.trim().is_empty() {
        return Ok(None);
    }
    let mut stmt = c.prepare(&format!("SELECT id,{column} FROM {table}"))?;
    let matches = stmt
        .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?
        .collect::<rusqlite::Result<Vec<_>>>()?
        .into_iter()
        .filter(|r| label(&r.1) == label(name))
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        return Err(format!("missing or ambiguous {table} label {name}").into());
    }
    Ok(Some(matches[0].0))
}

fn diluent(c: &Connection, code: &str, name: &str) -> Result<(Option<i64>, bool)> {
    if name.trim().is_empty() {
        return Ok((None, false));
    }
    let mut stmt = c.prepare("SELECT id,legacy_dilcode,diluent_name,volume_ml FROM diluents")?;
    let rows = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, Option<String>>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, Option<f64>>(3)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let coded = rows
        .iter()
        .filter(|r| !code.is_empty() && r.1.as_ref().is_some_and(|s| key(s) == key(code)))
        .collect::<Vec<_>>();
    if coded.len() > 1 {
        return Err("ambiguous diluent code".into());
    }
    if let Some(r) = coded.first().filter(|r| label(&r.2) == label(name)) {
        return Ok((Some(r.0), r.3.is_none()));
    }
    let unknown = rows
        .iter()
        .filter(|r| label(&r.2) == label(name) && r.3.is_none())
        .collect::<Vec<_>>();
    if unknown.len() > 1 {
        return Err("ambiguous diluent with unspecified volume".into());
    }
    if let Some(r) = unknown.first() {
        return Ok((Some(r.0), true));
    }
    c.execute(
        "INSERT INTO diluents(diluent_name,volume_ml) VALUES(?1,NULL)",
        [name.trim()],
    )?;
    Ok((Some(c.last_insert_rowid()), true))
}

fn replace(
    c: &mut Connection,
    a: &Args,
    rows: &[Vec<String>],
    digest: &str,
) -> Result<serde_json::Value> {
    c.pragma_update(None, "foreign_keys", true)?;
    let tx = c.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    if a.apply {
        let path = a.backup.as_ref().ok_or("--backup required")?;
        OpenOptions::new().create_new(true).write(true).open(path)?;
        let mut dest = Connection::open(path)?;
        let source = Connection::open_with_flags(&a.database, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        rusqlite::backup::Backup::new(&source, &mut dest)?.run_to_completion(
            64,
            Duration::from_millis(10),
            None,
        )?;
        validate(&dest)?;
    }
    let ids = super::lookup(&tx, "drugs", "legacy_dcode")?;
    let overrides = a
        .map_code
        .iter()
        .map(|s| {
            s.split_once('=')
                .map(|(s, t)| (key(s), key(t)))
                .ok_or("invalid map-code")
        })
        .collect::<std::result::Result<HashMap<_, _>, _>>()?;
    let mut next_id: i64 =
        tx.query_row("SELECT COALESCE(MAX(id),0)+1 FROM drugs", [], |r| r.get(0))?;
    let mut seen = HashSet::new();
    let mut kept = HashSet::new();
    let mut plans = Vec::new();
    let mut unresolved_volumes = Vec::new();
    let mut updated = 0;
    for row in rows.iter().skip(1) {
        if row.iter().all(|s| s.trim().is_empty()) {
            continue;
        }
        let cell = |i: usize| row.get(i).map(|s| s.trim()).unwrap_or("");
        if row.len() > 28 || cell(0).is_empty() || cell(1).is_empty() || !seen.insert(key(cell(0)))
        {
            return Err("invalid, empty or duplicate drug identity".into());
        }
        for i in [9, 20, 23, 25] {
            if cell(i).is_empty() {
                return Err(format!("missing Yes/No for drug {}", cell(0)).into());
            }
        }
        let old = overrides
            .get(&key(cell(0)))
            .cloned()
            .unwrap_or_else(|| key(cell(0)));
        let id = if let Some(id) = ids.get(&old) {
            updated += 1;
            *id
        } else {
            let id = next_id;
            next_id += 1;
            id
        };
        if !kept.insert(id) {
            return Err("multiple rows target one existing drug".into());
        }
        let mut values = FIELDS
            .iter()
            .map(|(i, _, kind)| {
                convert(cell(*i), kind, &HashMap::new())
                    .map_err(|e| format!("drug {} field {}: {e}", cell(0), HEADERS[*i]).into())
            })
            .collect::<Result<Vec<_>>>()?;
        values.push(
            named_lookup(&tx, "units", "unit_name", cell(3))?
                .map(Value::Integer)
                .unwrap_or(Value::Null),
        );
        values.push(
            named_lookup(&tx, "routes", "route_name", cell(15))?
                .map(Value::Integer)
                .unwrap_or(Value::Null),
        );
        let (dil, unknown) = diluent(&tx, cell(12), cell(13))?;
        values.push(dil.map(Value::Integer).unwrap_or(Value::Null));
        if unknown && cell(13) != "--" {
            unresolved_volumes.push(cell(0).to_owned());
        }
        plans.push((id, ids.values().any(|v| *v == id), values));
    }
    if plans.is_empty() {
        return Err("refusing empty catalog replacement".into());
    }
    tx.execute_batch("CREATE TEMP TABLE retained_drugs(id INTEGER PRIMARY KEY); CREATE TEMP TABLE removed_drugs(id INTEGER PRIMARY KEY);")?;
    for id in kept {
        tx.execute("INSERT INTO retained_drugs VALUES(?1)", [id])?;
    }
    tx.execute("INSERT INTO removed_drugs SELECT id FROM drugs WHERE id NOT IN (SELECT id FROM retained_drugs)",[])?;
    let removed: i64 = tx.query_row("SELECT COUNT(*) FROM removed_drugs", [], |r| r.get(0))?;
    let groups:i64=tx.query_row("SELECT COUNT(DISTINCT regimen_group_id) FROM regimen_items WHERE drug_id IN (SELECT id FROM removed_drugs)",[],|r|r.get(0))?;
    if groups > 0 && !a.remove_affected_regimen_groups {
        return Err(format!("{groups} regimen groups reference removed drugs; an explicit regimen decision is required").into());
    }
    // Do not bypass immutable preparation/output history. Unexpected dependencies abort.
    let unsupported:i64=tx.query_row("SELECT
        (SELECT COUNT(*) FROM preparation_tasks WHERE drug_id IN (SELECT id FROM removed_drugs)
          OR source_order_item_id IN (SELECT id FROM order_items WHERE drug_id IN (SELECT id FROM removed_drugs))) +
        (SELECT COUNT(*) FROM preparation_inventory_postings WHERE inventory_movement_id IN
          (SELECT id FROM inventory_movements WHERE drug_id IN (SELECT id FROM removed_drugs))) +
        (SELECT COUNT(*) FROM preparation_output_snapshots WHERE inventory_movement_id IN
          (SELECT id FROM inventory_movements WHERE drug_id IN (SELECT id FROM removed_drugs)))",[],|r|r.get(0))?;
    if unsupported != 0 {
        return Err("removed drugs have preparation history that requires separate review".into());
    }
    let mut deleted = serde_json::Map::new();
    if groups > 0 {
        let n=tx.execute("DELETE FROM regimen_groups WHERE id IN (SELECT regimen_group_id FROM regimen_items WHERE drug_id IN (SELECT id FROM removed_drugs))",[])?;
        deleted.insert("regimen_groups".into(), json!(n));
    }
    for table in [
        "order_items",
        "appointment_items",
        "inventory_events",
        "drug_administration",
        "drug_detail_groups",
    ] {
        let n = tx.execute(
            &format!("DELETE FROM {table} WHERE drug_id IN (SELECT id FROM removed_drugs)"),
            [],
        )?;
        deleted.insert(table.into(), json!(n));
    }
    // User-authorized obsolete-drug history removal, with exact trigger restoration.
    let trigger:String=tx.query_row("SELECT sql FROM sqlite_master WHERE type='trigger' AND name='inventory_movements_reject_delete'",[],|r|r.get(0))?;
    tx.execute_batch("DROP TRIGGER inventory_movements_reject_delete")?;
    let n = tx.execute(
        "DELETE FROM inventory_movements WHERE drug_id IN (SELECT id FROM removed_drugs)",
        [],
    )?;
    deleted.insert("inventory_movements".into(), json!(n));
    tx.execute_batch(&trigger)?;
    tx.execute(
        "DELETE FROM drugs WHERE id IN (SELECT id FROM removed_drugs)",
        [],
    )?;
    // Free old code spellings before assigning source codes, allowing reviewed code swaps.
    for (id, existing, _) in &plans {
        if *existing {
            tx.execute(
                "UPDATE drugs SET legacy_dcode=?1 WHERE id=?2",
                params![format!("__replacement_{digest}_{id}"), id],
            )?;
        }
    }
    let columns = FIELDS
        .iter()
        .map(|(_, col, _)| *col)
        .chain(["unit_id", "default_route_id", "default_diluent_id"])
        .collect::<Vec<_>>();
    for (id, existing, values) in &plans {
        let mut params = values.clone();
        params.push(Value::Integer(*id));
        if *existing {
            tx.execute(
                &format!(
                    "UPDATE drugs SET {} WHERE id=?",
                    columns
                        .iter()
                        .map(|col| format!("{col}=?"))
                        .collect::<Vec<_>>()
                        .join(",")
                ),
                params_from_iter(params.iter()),
            )?;
        } else {
            tx.execute(
                &format!(
                    "INSERT INTO drugs ({},id) VALUES ({})",
                    columns.join(","),
                    vec!["?"; params.len()].join(",")
                ),
                params_from_iter(params.iter()),
            )?;
        }
    }
    let report = json!({"applied":a.apply,"drugs":plans.len(),"updated":updated,"inserted":plans.len()-updated,"removed":removed,"deleted_dependents":deleted,"unspecified_diluent_volume_codes":unresolved_volumes,"source_sha256":digest,"source_url":a.source_url,"backup":a.backup});
    tx.execute("INSERT INTO audit_events(event_type,entity_type,entity_id,metadata_json) VALUES('drug_catalog_replacement','drug_catalog','all',?1)",[report.to_string()])?;
    validate(&tx)?;
    if a.apply {
        tx.commit()?;
    }
    Ok(report)
}

pub(super) fn run(a: &Args) -> Result<()> {
    if !a.skip_codes.is_empty() {
        return Err("replacement cannot skip source rows".into());
    }
    if a.database.to_string_lossy().starts_with("\\\\") {
        return Err("network database paths are prohibited".into());
    }
    let parent = a.database.parent().ok_or("database directory missing")?;
    if parent.join("connection.json").exists() {
        let settings: serde_json::Value =
            serde_json::from_slice(&std::fs::read(parent.join("connection.json"))?)?;
        if settings
            .get("host")
            .and_then(|v| v.as_str())
            .is_some_and(|s| !s.is_empty())
        {
            return Err(
                "client mode: run maintenance on the stopped server's local database".into(),
            );
        }
    }
    let _lock = if a.apply {
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(parent.join("oncoflow-server.lock"))?;
        file.try_lock()
            .map_err(|_| "close OncoFlow/server before replacement")?;
        Some(file)
    } else {
        None
    };
    let bytes = std::fs::read(&a.input)?;
    let rows: Vec<Vec<Option<String>>> = serde_json::from_slice(&bytes)?;
    let rows = rows
        .into_iter()
        .map(|r| {
            r.into_iter()
                .map(Option::unwrap_or_default)
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    if rows
        .first()
        .map(|r| r.iter().map(String::as_str).collect::<Vec<_>>())
        != Some(HEADERS.to_vec())
    {
        return Err("unexpected replacement sheet headers".into());
    }
    let digest = format!("{:x}", Sha256::digest(&bytes));
    let mut c = if a.apply {
        Connection::open_with_flags(&a.database, OpenFlags::SQLITE_OPEN_READ_WRITE)?
    } else {
        let source = Connection::open_with_flags(&a.database, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        let mut memory = Connection::open_in_memory()?;
        rusqlite::backup::Backup::new(&source, &mut memory)?.run_to_completion(
            64,
            Duration::from_millis(10),
            None,
        )?;
        memory
    };
    c.busy_timeout(Duration::from_secs(5))?;
    validate(&c)?;
    let report = replace(&mut c, a, &rows, &digest)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (tempfile::TempDir, Connection, Args, Vec<Vec<String>>) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("oncoflow.db");
        let c = Connection::open(&path).unwrap();
        let mut migrations = std::fs::read_dir(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../migrations"),
        )
        .unwrap()
        .map(|p| p.unwrap().path())
        .filter(|p| p.extension().is_some_and(|s| s == "sql"))
        .collect::<Vec<_>>();
        migrations.sort();
        for migration in migrations {
            c.execute_batch(&std::fs::read_to_string(migration).unwrap())
                .unwrap();
        }
        c.execute_batch("INSERT INTO units VALUES(1,'1','mg.'),(3,'3','unit');
            INSERT INTO routes VALUES(1,'1','IV Infusion');
            INSERT INTO diluents VALUES(1,'001','D-5-W',100);
            INSERT INTO drugs(id,legacy_dcode,drug_name,inventory_qty) VALUES(1,'01','Synthetic A',9),(2,'45','Synthetic obsolete',8),(3,'OF-D000051','Synthetic B',7);
            INSERT INTO regimens(id,legacy_regcode,regimen_name) VALUES(1,'SYN','Synthetic regimen');
            INSERT INTO regimen_groups(id,regimen_id) VALUES(1,1),(2,1);
            INSERT INTO regimen_items(id,regimen_group_id,drug_id) VALUES(1,1,1),(2,1,2),(3,2,1);
            INSERT INTO inventory_movements(drug_id,movement_type,quantity_delta) VALUES(1,'opening_balance',9),(2,'opening_balance',8);
            INSERT INTO patients(id,legacy_hn) VALUES(1,'SYNTHETIC');
            INSERT INTO orders(id,legacy_orderid,patient_id) VALUES(1,'SYNTHETIC',1);
            INSERT INTO order_items(id,order_id,drug_id) VALUES(1,1,2);").unwrap();
        let mut row = vec![String::new(); 28];
        row[0] = "1".into();
        row[1] = "Synthetic A revised".into();
        row[2] = "1".into();
        row[3] = "unit".into();
        row[4] = "15".into();
        row[5] = "5".into();
        row[6] = "Vial".into();
        row[12] = "1".into();
        row[13] = "NSS".into();
        row[15] = "IV Infusion".into();
        for i in [9, 20, 23, 25] {
            row[i] = "No".into();
        }
        let mut second = row.clone();
        second[0] = "45".into();
        second[1] = "Synthetic B revised".into();
        let mut third = row.clone();
        third[0] = "96".into();
        third[1] = "Synthetic new".into();
        third[3] = "mg".into();
        let rows = vec![
            HEADERS.iter().map(|s| s.to_string()).collect(),
            row,
            second,
            third,
        ];
        let input = dir.path().join("source.json");
        std::fs::write(&input, serde_json::to_vec(&rows).unwrap()).unwrap();
        let args = Args {
            database: path,
            input,
            source_url: "synthetic-source".into(),
            skip_codes: vec![],
            map_code: vec!["45=OF-D000051".into()],
            apply: true,
            backup: Some(dir.path().join("backup.db")),
            replace_catalog: true,
            remove_affected_regimen_groups: true,
        };
        (dir, c, args, rows)
    }
    #[test]
    fn replaces_exact_codes_and_labels_and_removes_whole_affected_groups() {
        let (_dir, mut c, a, rows) = fixture();
        let report = replace(&mut c, &a, &rows, "synthetic-digest").unwrap();
        assert_eq!(report["drugs"], 3);
        assert_eq!(report["removed"], 1);
        assert_eq!(
            c.query_row("SELECT legacy_dcode FROM drugs WHERE id=1", [], |r| r
                .get::<_, String>(0))
                .unwrap(),
            "1"
        );
        assert_eq!(
            c.query_row("SELECT legacy_dcode FROM drugs WHERE id=3", [], |r| r
                .get::<_, String>(0))
                .unwrap(),
            "45"
        );
        assert_eq!(
            c.query_row("SELECT unit_id FROM drugs WHERE id=1", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            3
        );
        assert_eq!(c.query_row("SELECT dl.diluent_name,dl.volume_ml FROM drugs d JOIN diluents dl ON dl.id=d.default_diluent_id WHERE d.id=1",[],|r|Ok((r.get::<_,String>(0)?,r.get::<_,Option<f64>>(1)?))).unwrap(),("NSS".into(),None));
        assert_eq!(
            c.query_row("SELECT COUNT(*) FROM regimen_groups", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert_eq!(
            c.query_row("SELECT id FROM regimen_groups", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            2
        );
        assert_eq!(
            c.query_row("SELECT COUNT(*) FROM order_items", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
        assert_eq!(
            c.query_row(
                "SELECT SUM(quantity_delta) FROM inventory_movements",
                [],
                |r| r.get::<_, f64>(0)
            )
            .unwrap(),
            9.0
        );
        assert!(c.execute("DELETE FROM inventory_movements", []).is_err());
        assert_eq!(
            c.query_row(
                "SELECT COUNT(*) FROM audit_events WHERE event_type='drug_catalog_replacement'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            1
        );
        let b = Connection::open(a.backup.unwrap()).unwrap();
        validate(&b).unwrap();
        assert_eq!(
            b.query_row("SELECT COUNT(*) FROM regimen_groups", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            2
        );
        validate(&c).unwrap();
    }
    #[test]
    fn later_failure_restores_deletes_and_immutable_trigger() {
        let (_dir, mut c, a, rows) = fixture();
        c.execute_batch("CREATE TRIGGER synthetic_failure BEFORE INSERT ON drugs BEGIN SELECT RAISE(ABORT,'synthetic failure'); END;").unwrap();
        assert!(replace(&mut c, &a, &rows, "synthetic-digest").is_err());
        assert_eq!(
            c.query_row("SELECT COUNT(*) FROM regimen_groups", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            2
        );
        assert_eq!(
            c.query_row(
                "SELECT COUNT(*) FROM drugs WHERE legacy_dcode='01'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            1
        );
        assert_eq!(
            c.query_row("SELECT COUNT(*) FROM inventory_movements", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            2
        );
        assert!(c.execute("DELETE FROM inventory_movements", []).is_err());
        assert_eq!(
            c.query_row("SELECT COUNT(*) FROM audit_events", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
    #[test]
    fn preview_never_changes_source_and_regimen_decision_is_required() {
        let (_dir, mut c, mut a, rows) = fixture();
        a.apply = false;
        a.backup = None;
        run(&a).unwrap();
        assert_eq!(
            c.query_row(
                "SELECT COUNT(*) FROM drugs WHERE legacy_dcode='01'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            1
        );
        a.remove_affected_regimen_groups = false;
        assert!(replace(&mut c, &a, &rows, "synthetic-digest").is_err());
        assert_eq!(
            c.query_row("SELECT COUNT(*) FROM regimen_groups", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            2
        );
    }
}
