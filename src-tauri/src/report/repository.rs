use rusqlite::{params, Connection};

use crate::inventory::stock_state;

use super::{InventoryUsageReportRow, PreparationCountReportRow, ReportInterval};

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
                COUNT(*),SUM(t.final_container_count)
         FROM preparation_tasks t
         JOIN drugs d ON d.id=t.drug_id
         LEFT JOIN users u ON u.id=t.prepared_by_user_id
         WHERE t.state IN ('prepared','verified')
           AND t.preparation_date BETWEEN ?1 AND ?2
         GROUP BY period_start,t.drug_id,d.drug_name,t.prepared_by_user_id,
                  COALESCE(NULLIF(TRIM(u.display_name),''),u.username,'ไม่ระบุผู้เตรียม')
         ORDER BY period_start ASC,d.drug_name COLLATE NOCASE ASC,
                  preparer_name COLLATE NOCASE ASC,t.drug_id ASC"
    );

    connection
        .prepare(&sql)?
        .query_map(params![date_from, date_to], |row| {
            Ok(PreparationCountReportRow {
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
                COUNT(*),SUM(t.final_container_count),
                COALESCE(SUM(CASE WHEN p.status='posted' THEN p.containers_required ELSE 0 END),0),
                SUM(CASE WHEN t.state='prepared' THEN 1 ELSE 0 END),
                SUM(CASE WHEN p.status='manual_reconciliation_required' THEN 1 ELSE 0 END),
                SUM(CASE WHEN p.status='tracking_disabled' THEN 1 ELSE 0 END),
                SUM(CASE WHEN t.state='verified' AND p.id IS NULL THEN 1 ELSE 0 END),
                b.current_stock,d.inventory_min,d.inventory_enabled
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
