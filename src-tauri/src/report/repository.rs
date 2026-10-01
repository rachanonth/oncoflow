use rusqlite::{params, Connection};

use crate::inventory::stock_state;

use super::{InventoryUsageReportRow, PreparationCountReportRow, ReportInterval};

pub(super) fn diluent_usage(
    connection: &Connection,
    date: &str,
) -> rusqlite::Result<Vec<super::DiluentUsageRow>> {
    connection
        .prepare(
            "SELECT NULLIF(trim(snapshot_diluent_name),''),snapshot_diluent_volume_ml,
                SUM(final_container_count),COUNT(*)
         FROM preparation_tasks
         WHERE preparation_date=?1 AND state='verified' AND cancellation_id IS NULL
         GROUP BY NULLIF(trim(snapshot_diluent_name),''),snapshot_diluent_volume_ml
         ORDER BY NULLIF(trim(snapshot_diluent_name),'') IS NULL,
                  NULLIF(trim(snapshot_diluent_name),'') COLLATE NOCASE,
                  snapshot_diluent_volume_ml IS NULL,snapshot_diluent_volume_ml",
        )?
        .query_map([date], |row| {
            Ok(super::DiluentUsageRow {
                diluent_name: row.get(0)?,
                volume_ml: row.get(1)?,
                bottle_count: row.get(2)?,
                dose_count: row.get(3)?,
            })
        })?
        .collect()
}

pub(super) fn ward_delivery(
    connection: &Connection,
    date: &str,
) -> rusqlite::Result<Vec<super::WardDeliveryRow>> {
    connection.prepare(
        "SELECT t.id,o.ward_id,COALESCE(NULLIF(trim(w.ward_name),''),'ไม่ระบุหอผู้ป่วย'),
                p.id,p.legacy_hn,
                COALESCE(NULLIF(trim(COALESCE(p.title,'') || ' ' || COALESCE(p.first_name,'') || ' ' || COALESCE(p.last_name,'')),''),'ไม่ระบุชื่อ'),
                d.drug_name,t.snapshot_ordered_dose_text,t.snapshot_dose_unit_text,
                t.snapshot_diluent_name,t.snapshot_diluent_volume_ml
         FROM preparation_tasks t
         JOIN orders o ON o.id=t.source_order_id
         JOIN patients p ON p.id=o.patient_id
         JOIN drugs d ON d.id=t.drug_id
         LEFT JOIN wards w ON w.id=o.ward_id
         WHERE t.state='verified' AND t.cancellation_id IS NULL AND t.preparation_date=?1
         ORDER BY w.ward_name COLLATE NOCASE,o.ward_id,p.legacy_hn,p.id,
                  o.id,t.snapshot_sequence_no,t.id"
    )?.query_map([date], |row| Ok(super::WardDeliveryRow {
        task_id: row.get(0)?, ward_id: row.get(1)?, ward_name: row.get(2)?,
        patient_id: row.get(3)?, patient_hn: row.get(4)?, patient_name: row.get(5)?,
        drug_name: row.get(6)?, ordered_dose_text: row.get(7)?, dose_unit_text: row.get(8)?,
        diluent_name: row.get(9)?, diluent_volume_ml: row.get(10)?,
    }))?.collect()
}

fn period_expression(interval: ReportInterval) -> &'static str {
    match interval {
        ReportInterval::Daily => "t.preparation_date",
        ReportInterval::Weekly => "date(t.preparation_date, printf('-%d days', (CAST(strftime('%w', t.preparation_date) AS INTEGER) + 6) % 7))",
        ReportInterval::Monthly => "substr(t.preparation_date, 1, 7) || '-01'",
    }
}

pub(super) fn preparation_counts(
    connection: &Connection,
    interval: ReportInterval,
    date_from: &str,
    date_to: &str,
) -> rusqlite::Result<Vec<PreparationCountReportRow>> {
    let period = period_expression(interval);
    let sql = format!(
        "SELECT {period} AS period_start,
                t.drug_id,d.drug_name,t.prepared_by_user_id,
                COALESCE(NULLIF(TRIM(u.display_name),''),u.username,'ไม่ระบุผู้เตรียม') AS preparer_name,
                COUNT(*),SUM(t.final_container_count),t.cancellation_id IS NOT NULL
         FROM preparation_tasks t
         JOIN drugs d ON d.id=t.drug_id
         LEFT JOIN users u ON u.id=t.prepared_by_user_id
         WHERE (t.state IN ('prepared','verified') AND t.cancellation_id IS NULL OR t.cancelled_after_preparation=1)
           AND t.preparation_date BETWEEN ?1 AND ?2
         GROUP BY period_start,t.drug_id,d.drug_name,t.prepared_by_user_id,
                  COALESCE(NULLIF(TRIM(u.display_name),''),u.username,'ไม่ระบุผู้เตรียม'),t.cancellation_id IS NOT NULL
         ORDER BY period_start ASC,d.drug_name COLLATE NOCASE ASC,
                  preparer_name COLLATE NOCASE ASC,t.drug_id ASC"
    );

    connection
        .prepare(&sql)?
        .query_map(params![date_from, date_to], |row| {
            Ok(PreparationCountReportRow {
                cancelled: row.get(7)?,
                period_start: row.get(0)?,
                drug_id: row.get(1)?,
                drug_name: row.get(2)?,
                preparer_user_id: row.get(3)?,
                preparer_name: row.get(4)?,
                prescription_count: row.get(5)?,
                bottle_count: row.get(6)?,
            })
        })?
        .collect()
}

pub(super) fn inventory_usage(
    connection: &Connection,
    interval: ReportInterval,
    date_from: &str,
    date_to: &str,
) -> rusqlite::Result<Vec<InventoryUsageReportRow>> {
    let period = period_expression(interval);
    let sql = format!(
        "WITH balances AS (
           SELECT drug_id,SUM(quantity_delta) AS current_stock
           FROM inventory_movements
           GROUP BY drug_id
         )
         SELECT {period} AS period_start,
                t.drug_id,d.legacy_dcode,d.drug_name,
                COALESCE(NULLIF(TRIM(d.package),''),NULLIF(TRIM(u.unit_name),''),'ภาชนะ'),
                SUM(CASE WHEN t.cancellation_id IS NULL OR t.cancelled_after_preparation=1 THEN 1 ELSE 0 END),
                SUM(CASE WHEN t.cancellation_id IS NULL OR t.cancelled_after_preparation=1 THEN t.final_container_count ELSE 0 END),
                COALESCE(SUM(CASE WHEN p.status='posted' THEN p.containers_required ELSE 0 END),0),
                SUM(CASE WHEN t.state='prepared' AND t.cancellation_id IS NULL THEN 1 ELSE 0 END),
                SUM(CASE WHEN p.status='manual_reconciliation_required' THEN 1 ELSE 0 END),
                SUM(CASE WHEN p.status='tracking_disabled' THEN 1 ELSE 0 END),
                SUM(CASE WHEN t.state='verified' AND p.id IS NULL THEN 1 ELSE 0 END),
                b.current_stock,d.inventory_min,d.inventory_enabled,
                SUM(CASE WHEN t.cancellation_id IS NOT NULL AND p.id IS NOT NULL THEN 1 ELSE 0 END)
         FROM preparation_tasks t
         JOIN drugs d ON d.id=t.drug_id
         LEFT JOIN units u ON u.id=d.unit_id
         LEFT JOIN preparation_inventory_postings p ON p.preparation_task_id=t.id
         LEFT JOIN balances b ON b.drug_id=t.drug_id
         WHERE t.state IN ('prepared','verified')
           AND t.preparation_date BETWEEN ?1 AND ?2
         GROUP BY period_start,t.drug_id,d.legacy_dcode,d.drug_name,
                  COALESCE(NULLIF(TRIM(d.package),''),NULLIF(TRIM(u.unit_name),''),'ภาชนะ'),
                  b.current_stock,d.inventory_min,d.inventory_enabled
         ORDER BY period_start ASC,d.drug_name COLLATE NOCASE ASC,t.drug_id ASC"
    );

    connection
        .prepare(&sql)?
        .query_map(params![date_from, date_to], |row| {
            let current_stock = row.get(12)?;
            let minimum_stock = row.get(13)?;
            let tracking_enabled = row.get::<_, i64>(14)? != 0;
            Ok(InventoryUsageReportRow {
                cancelled_review_count: row.get(15)?,
                period_start: row.get(0)?,
                drug_id: row.get(1)?,
                drug_code: row.get(2)?,
                drug_name: row.get(3)?,
                source_package: row.get(4)?,
                prescription_count: row.get(5)?,
                prepared_bottle_count: row.get(6)?,
                issued_source_container_count: row.get(7)?,
                awaiting_verification_count: row.get(8)?,
                manual_reconciliation_count: row.get(9)?,
                tracking_disabled_count: row.get(10)?,
                unrecorded_inventory_count: row.get(11)?,
                current_stock,
                minimum_stock,
                stock_state: stock_state(tracking_enabled, current_stock, minimum_stock),
            })
        })?
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diluent_counts_checked_bottles_by_snapshot_name_and_volume() {
        let connection = Connection::open_in_memory().unwrap();
        connection.execute_batch("PRAGMA foreign_keys=ON;
            CREATE TABLE preparation_tasks(preparation_date TEXT,state TEXT,cancellation_id INTEGER,snapshot_diluent_name TEXT,snapshot_diluent_volume_ml REAL,final_container_count INTEGER);
            INSERT INTO preparation_tasks VALUES
            ('2026-10-01','verified',NULL,'D-5-W',250,3),
            ('2026-10-01','verified',NULL,'NSS',100,2),
            ('2026-10-01','verified',NULL,' NSS ',100,3),
            ('2026-10-01','verified',NULL,'NSS',1000,2),
            ('2026-10-01','verified',NULL,'NSS',250,5),
            ('2026-10-01','verified',NULL,'NSS',50,1),
            ('2026-10-01','verified',NULL,'NSS',500,3),
            ('2026-10-01','prepared',NULL,'NSS',100,9),
            ('2026-10-01','pending',NULL,'NSS',100,9),
            ('2026-10-01','verified',1,'NSS',100,9),
            ('2026-10-02','verified',NULL,'NSS',100,9);").unwrap();
        let before = connection.total_changes();
        let rows = diluent_usage(&connection, "2026-10-01").unwrap();
        assert_eq!(rows.len(), 6);
        assert_eq!(rows.iter().map(|row| row.bottle_count).sum::<i64>(), 19);
        assert_eq!(rows.iter().map(|row| row.dose_count).sum::<i64>(), 7);
        let nss = rows
            .iter()
            .find(|row| row.diluent_name.as_deref() == Some("NSS") && row.volume_ml == Some(100.0))
            .unwrap();
        assert_eq!(nss.bottle_count, 5);
        assert_eq!(nss.dose_count, 2);
        assert!(diluent_usage(&connection, "2026-09-30").unwrap().is_empty());
        assert_eq!(connection.total_changes(), before);
        connection
            .execute_batch(
                "INSERT INTO preparation_tasks VALUES
            ('2026-10-01','verified',NULL,NULL,NULL,2),
            ('2026-10-01','verified',NULL,'  ',NULL,1),
            ('2026-10-01','verified',NULL,'NSS',0,1);",
            )
            .unwrap();
        let rows = diluent_usage(&connection, "2026-10-01").unwrap();
        let missing = rows.iter().find(|row| row.diluent_name.is_none()).unwrap();
        assert_eq!(missing.bottle_count, 3);
        assert_eq!(missing.volume_ml, None);
        assert!(rows.iter().any(|row| row.volume_ml == Some(0.0)));
    }

    #[test]
    fn delivery_uses_checked_snapshots_for_exact_date_without_collapsing_duplicates() {
        let directory = tempfile::tempdir().unwrap();
        let database =
            crate::db::Database::initialize(directory.path().join("oncoflow.db")).unwrap();
        let connection = database.open().unwrap();
        connection.execute_batch(
            "INSERT INTO wards(id,legacy_wcode,ward_name) VALUES(1,'SYN-W','หอทดสอบ');
             INSERT INTO patients(id,legacy_hn,first_name,last_name) VALUES(1,'SYN-001','ผู้ป่วย','ทดสอบ');
             INSERT INTO drugs(id,legacy_dcode,drug_name) VALUES(1,'SYN-D','ยาทดสอบ');
             INSERT INTO orders(id,legacy_orderid,patient_id,ward_id,oncoflow_created) VALUES(1,'SYN-O1',1,1,1),(2,'SYN-O2',1,NULL,1);
             INSERT INTO order_items(id,order_id,drug_id,dose) VALUES(1,1,1,999),(2,1,1,999),(3,1,1,999),(4,2,1,999);
             INSERT INTO preparation_tasks(id,source_order_id,source_order_item_id,drug_id,preparation_date,state,prepared_at,verified_at,snapshot_ordered_dose_text,snapshot_dose_unit_text,snapshot_diluent_name,snapshot_diluent_volume_ml,final_container_count)
             VALUES(1,1,1,1,'2026-09-29','verified','2026-09-29 01:00','2026-09-29 02:00','2300','mg','D-5-W',1000,2),
                   (2,1,2,1,'2026-09-29','verified','2026-09-29 01:00','2026-09-29 02:00','2300','mg','D-5-W',1000,1),
                   (3,1,3,1,'2026-09-29','prepared','2026-09-29 01:00',NULL,'2300','mg',NULL,NULL,1),
                   (4,1,1,1,'2026-09-30','verified','2026-09-30 01:00','2026-09-30 02:00','2400','mg','D-5-W',1000,1),
                   (5,2,4,1,'2026-09-29','verified','2026-09-29 01:00','2026-09-29 02:00',NULL,NULL,NULL,NULL,1),
                   (6,1,2,1,'2026-09-30','pending',NULL,NULL,'2400','mg',NULL,NULL,1);"
        ).unwrap();
        let before = connection.total_changes();
        let rows = ward_delivery(&connection, "2026-09-29").unwrap();
        assert_eq!(rows.len(), 3);
        let known_ward: Vec<_> = rows.iter().filter(|row| row.ward_id == Some(1)).collect();
        assert_eq!(known_ward.len(), 2);
        assert_ne!(known_ward[0].task_id, known_ward[1].task_id);
        for row in known_ward {
            assert_eq!(row.ordered_dose_text.as_deref(), Some("2300"));
            assert_eq!(row.diluent_volume_ml, Some(1000.0));
        }
        let unknown = rows.iter().find(|row| row.ward_id.is_none()).unwrap();
        assert_eq!(unknown.ward_name, "ไม่ระบุหอผู้ป่วย");
        assert!(unknown.ordered_dose_text.is_none());
        assert_eq!(ward_delivery(&connection, "2026-09-30").unwrap().len(), 1);
        assert!(ward_delivery(&connection, "2026-10-01").unwrap().is_empty());
        assert_eq!(connection.total_changes(), before);
    }

    fn fixture() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch(
                "CREATE TABLE drugs(id INTEGER PRIMARY KEY,drug_name TEXT NOT NULL);
             CREATE TABLE users(id INTEGER PRIMARY KEY,username TEXT NOT NULL,display_name TEXT);
             CREATE TABLE preparation_tasks(
               id INTEGER PRIMARY KEY,preparation_date TEXT NOT NULL,drug_id INTEGER NOT NULL,
               state TEXT NOT NULL,prepared_by_user_id INTEGER,final_container_count INTEGER NOT NULL
             );
             INSERT INTO drugs VALUES(1,'Paclitaxel'),(2,'Carboplatin');
             INSERT INTO users VALUES(7,'prepare.one','เภสัชกร หนึ่ง'),(8,'prepare.two','');
             INSERT INTO preparation_tasks VALUES
               (1,'2026-08-03',1,'prepared',7,1),
               (2,'2026-08-03',1,'verified',7,2),
               (3,'2026-08-09',2,'verified',8,3),
               (4,'2026-08-10',1,'pending',7,4),
               (5,'2026-08-10',1,'prepared',NULL,1);",
            )
            .unwrap();
        connection.execute_batch("ALTER TABLE preparation_tasks ADD COLUMN cancellation_id INTEGER; ALTER TABLE preparation_tasks ADD COLUMN cancelled_after_preparation INTEGER NOT NULL DEFAULT 0;").unwrap();
        connection
    }

    #[test]
    fn counts_prepared_lines_by_drug_and_preparer_only() {
        let rows = preparation_counts(
            &fixture(),
            ReportInterval::Daily,
            "2026-08-01",
            "2026-08-31",
        )
        .unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].prescription_count, 2);
        assert_eq!(rows[0].bottle_count, 3);
        assert_eq!(
            rows.iter().map(|row| row.prescription_count).sum::<i64>(),
            4
        );
        assert_eq!(rows.iter().map(|row| row.bottle_count).sum::<i64>(), 7);
        assert!(rows.iter().any(|row| row.preparer_name == "ไม่ระบุผู้เตรียม"));
    }

    #[test]
    fn weekly_period_starts_on_monday() {
        let rows = preparation_counts(
            &fixture(),
            ReportInterval::Weekly,
            "2026-08-01",
            "2026-08-31",
        )
        .unwrap();
        assert_eq!(rows[0].period_start, "2026-08-03");
        assert!(rows.iter().any(|row| row.period_start == "2026-08-10"));
    }

    #[test]
    fn inventory_usage_separates_prepared_bottles_from_issued_source_containers() {
        let connection = Connection::open_in_memory().unwrap();
        connection.execute_batch(
            "CREATE TABLE units(id INTEGER PRIMARY KEY,unit_name TEXT);
             CREATE TABLE drugs(id INTEGER PRIMARY KEY,legacy_dcode TEXT NOT NULL,drug_name TEXT NOT NULL,package TEXT,unit_id INTEGER,inventory_min REAL,inventory_enabled INTEGER NOT NULL);
             CREATE TABLE preparation_tasks(id INTEGER PRIMARY KEY,preparation_date TEXT NOT NULL,drug_id INTEGER NOT NULL,state TEXT NOT NULL,final_container_count INTEGER NOT NULL);
             CREATE TABLE preparation_inventory_postings(id INTEGER PRIMARY KEY,preparation_task_id INTEGER NOT NULL,status TEXT NOT NULL,containers_required INTEGER);
             CREATE TABLE inventory_movements(id INTEGER PRIMARY KEY,drug_id INTEGER NOT NULL,quantity_delta REAL NOT NULL);
             INSERT INTO units VALUES(1,'vial');
             INSERT INTO drugs VALUES(1,'D001','Paclitaxel','vial',1,3,1);
             INSERT INTO preparation_tasks VALUES(1,'2026-08-03',1,'verified',2),(2,'2026-08-03',1,'prepared',1),(3,'2026-08-03',1,'verified',2);
             INSERT INTO preparation_inventory_postings VALUES(1,1,'posted',3),(2,3,'manual_reconciliation_required',NULL);
             INSERT INTO inventory_movements VALUES(1,1,10),(2,1,-3);"
        ).unwrap();
        connection.execute_batch("ALTER TABLE preparation_tasks ADD COLUMN cancellation_id INTEGER; ALTER TABLE preparation_tasks ADD COLUMN cancelled_after_preparation INTEGER NOT NULL DEFAULT 0;").unwrap();
        let rows = inventory_usage(
            &connection,
            ReportInterval::Daily,
            "2026-08-01",
            "2026-08-31",
        )
        .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].prescription_count, 3);
        assert_eq!(rows[0].prepared_bottle_count, 5);
        assert_eq!(rows[0].issued_source_container_count, 3);
        assert_eq!(rows[0].awaiting_verification_count, 1);
        assert_eq!(rows[0].manual_reconciliation_count, 1);
        assert_eq!(rows[0].current_stock, Some(7.0));
    }
}
