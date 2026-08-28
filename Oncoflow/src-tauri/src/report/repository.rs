use rusqlite::{params, Connection};

use super::{PreparationCountReportRow, ReportInterval};

pub(super) fn preparation_counts(
    connection: &Connection,
    interval: ReportInterval,
    date_from: &str,
    date_to: &str,
) -> rusqlite::Result<Vec<PreparationCountReportRow>> {
    let period = match interval {
        ReportInterval::Daily => "t.preparation_date",
        ReportInterval::Weekly => "date(t.preparation_date, printf('-%d days', (CAST(strftime('%w', t.preparation_date) AS INTEGER) + 6) % 7))",
        ReportInterval::Monthly => "substr(t.preparation_date, 1, 7) || '-01'",
    };
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
}
