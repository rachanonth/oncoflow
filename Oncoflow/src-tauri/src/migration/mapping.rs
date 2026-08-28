use rusqlite::{params, params_from_iter, types::Value, OptionalExtension, Transaction};

use super::{ImportError, MigrationIssue, SourceDatabase, SourceRow, SourceValue, TableReport};

#[derive(Clone, Copy)]
enum Transform {
    Text,
    TextPlaceholder(&'static str),
    Integer,
    Real,
    Boolean,
    BooleanText,
    IntegerText,
    Date,
    DateTime,
    Time,
    OptionalParsedInteger,
    OptionalParsedReal,
    Lookup {
        table: &'static str,
        key: &'static str,
        required: bool,
        identifier_is_sensitive: bool,
    },
    ConstantText(&'static str),
    ConstantInteger(i64),
}

#[derive(Clone, Copy)]
struct ColumnMapping {
    source: Option<&'static str>,
    destination: &'static str,
    transform: Transform,
}

#[derive(Clone, Copy)]
enum InsertMode {
    Insert,
    Replace,
}

pub(crate) struct TableMapping {
    pub source: &'static str,
    pub destination: &'static str,
    columns: &'static [ColumnMapping],
    mode: InsertMode,
    skip_if_null: Option<&'static str>,
}

const fn column(
    source: &'static str,
    destination: &'static str,
    transform: Transform,
) -> ColumnMapping {
    ColumnMapping {
        source: Some(source),
        destination,
        transform,
    }
}

const fn constant(destination: &'static str, transform: Transform) -> ColumnMapping {
    ColumnMapping {
        source: None,
        destination,
        transform,
    }
}

const fn lookup(
    source: &'static str,
    destination: &'static str,
    table: &'static str,
    key: &'static str,
    required: bool,
    identifier_is_sensitive: bool,
) -> ColumnMapping {
    column(
        source,
        destination,
        Transform::Lookup {
            table,
            key,
            required,
            identifier_is_sensitive,
        },
    )
}

macro_rules! mapping {
    ($source:literal => $destination:literal, [$($column:expr),* $(,)?]) => {
        TableMapping {
            source: $source,
            destination: $destination,
            columns: &[$($column),*],
            mode: InsertMode::Insert,
            skip_if_null: None,
        }
    };
    ($source:literal => $destination:literal, replace, [$($column:expr),* $(,)?]) => {
        TableMapping {
            source: $source,
            destination: $destination,
            columns: &[$($column),*],
            mode: InsertMode::Replace,
            skip_if_null: None,
        }
    };
    ($source:literal => $destination:literal, skip_if_null = $skip:literal, [$($column:expr),* $(,)?]) => {
        TableMapping {
            source: $source,
            destination: $destination,
            columns: &[$($column),*],
            mode: InsertMode::Insert,
            skip_if_null: Some($skip),
        }
    };
}

pub(crate) static TABLE_MAPPINGS: &[TableMapping] = &[
    mapping!("TblUser" => "users", [
        column("user", "legacy_user", Transform::Text),
        column("user", "username", Transform::Text),
        column("name", "display_name", Transform::Text),
        constant("password_hash", Transform::ConstantText("LEGACY_CREDENTIAL_DISABLED")),
        constant("role", Transform::ConstantText("user")),
        constant("active", Transform::ConstantInteger(0)),
    ]),
    mapping!("Tblunit" => "units", [
        column("unitcode", "legacy_unitcode", Transform::Text),
        column("unitname", "unit_name", Transform::Text),
    ]),
    mapping!("Tblroute" => "routes", [
        column("rcode", "legacy_rcode", Transform::Text),
        column("rname", "route_name", Transform::Text),
    ]),
    mapping!("Tbldiluent" => "diluents", [
        column("dilcode", "legacy_dilcode", Transform::Text),
        column("dilname", "diluent_name", Transform::TextPlaceholder("[Legacy unnamed diluent]")),
        column("vol", "volume_ml", Transform::Real),
    ]),
    mapping!("Tbldoctor" => "doctors", [
        column("doccode", "legacy_doccode", Transform::Text),
        column("docname", "doctor_name", Transform::Text),
    ]),
    mapping!("Tblward" => "wards", [
        column("wcode", "legacy_wcode", Transform::Text),
        column("wname", "ward_name", Transform::Text),
        column("tel", "telephone", Transform::Text),
    ]),
    mapping!("TblDiagnosis" => "diagnoses", [
        column("diagcode", "id", Transform::Integer),
        column("diagcode", "legacy_diagcode", Transform::IntegerText),
        column("diagnosis", "diagnosis", Transform::Text),
        column("warning1", "warning1", Transform::BooleanText),
        column("warning2", "warning2", Transform::BooleanText),
    ]),
    mapping!("TblRegimen" => "regimens", [
        column("regcode", "id", Transform::Integer),
        column("regcode", "legacy_regcode", Transform::IntegerText),
        column("Regimen", "regimen_name", Transform::Text),
        column("marker", "marker", Transform::Boolean),
        column("flag", "flag", Transform::Boolean),
        column("cyccheck", "cycle_check", Transform::Boolean),
        column("auto", "auto_mode", Transform::Boolean),
        column("DAlert", "drug_alert", Transform::Boolean),
        column("AppAlert", "appointment_alert", Transform::Boolean),
        column("CounselAlert", "counsel_alert", Transform::Boolean),
    ]),
    mapping!("Tbldrug" => "drugs", [
        column("dcode", "legacy_dcode", Transform::Text),
        column("dname", "drug_name", Transform::Text),
        lookup("unitcode", "unit_id", "units", "legacy_unitcode", false, false),
        column("dose/pack", "dose_per_pack", Transform::Real),
        column("vol/pack", "volume_per_pack_ml", Transform::Real),
        column("package", "package", Transform::Text),
        column("Detail", "detail", Transform::Text),
        column("price", "price", Transform::Real),
        column("theory", "theory", Transform::Text),
        column("marker", "marker", Transform::Boolean),
        lookup("dfdiluent", "default_diluent_id", "diluents", "legacy_dilcode", false, false),
        lookup("dfroute", "default_route_id", "routes", "legacy_rcode", false, false),
        column("dfrate", "default_rate", Transform::Text),
        column("warning", "warning", Transform::Text),
        column("storage", "storage", Transform::Text),
        column("flag", "flag", Transform::Boolean),
        column("exptime", "expiry_time", Transform::Time),
        column("expstore", "expiry_storage", Transform::Text),
        column("Maxdose", "max_dose", Transform::Real),
        column("MaxDilAlert", "max_dilution_alert", Transform::Boolean),
        column("MaxDilH", "max_dilution_hard", Transform::Real),
        column("CumAlert", "cumulative_alert", Transform::Boolean),
        column("CumAlertH", "cumulative_alert_hard", Transform::Real),
        column("Dil_Incompat", "dilution_incompatibility", Transform::Text),
        column("InvCut", "inventory_cut", Transform::Boolean),
        column("InvMin", "inventory_min", Transform::Real),
        column("InvMax", "inventory_max", Transform::Real),
        column("Inv", "inventory_qty", Transform::Real),
        column("InvUse", "inventory_enabled", Transform::Boolean),
        column("HOMCCode", "homc_code", Transform::Text),
        column("exp", "legacy_exp", Transform::Integer),
        column("reg", "legacy_reg", Transform::Text),
    ]),
    mapping!("Tblpatient" => "patients", [
        column("hn", "legacy_hn", Transform::Text),
        column("xn", "legacy_xn", Transform::Text),
        column("CAno", "cancer_no", Transform::Text),
        column("bname", "title", Transform::Text),
        column("fname", "first_name", Transform::Text),
        column("lname", "last_name", Transform::Text),
        column("weight", "weight_kg", Transform::Real),
        column("high", "height_cm", Transform::Real),
        column("birthdate", "birth_date", Transform::Date),
        column("occup", "occupation", Transform::Text),
        column("address", "address", Transform::Text),
        lookup("diagcode", "diagnosis_id", "diagnoses", "legacy_diagcode", false, false),
        lookup("regcode", "regimen_id", "regimens", "legacy_regcode", false, false),
        column("appcard", "appointment_card", Transform::Boolean),
        column("counselling", "counselling", Transform::Boolean),
        column("PHistory", "patient_history", Transform::Text),
        column("Stage", "stage", Transform::Text),
        column("Her2", "her2", Transform::Text),
        column("ERPR", "erpr", Transform::Text),
        column("CD", "cd", Transform::Text),
        column("MH", "mh", Transform::Text),
        column("Allergy", "allergy", Transform::Text),
        column("record", "record_by", Transform::Text),
        column("recordtime", "record_time", Transform::DateTime),
        column("endtreat", "treatment_ended", Transform::Boolean),
        column("age", "legacy_age", Transform::Real),
        column("bsa", "legacy_bsa", Transform::Real),
        column("sex", "sex", Transform::Text),
        column("tel", "telephone", Transform::Text),
    ]),
    mapping!("TblDrug Details1" => "drug_detail_groups", [
        column("code", "id", Transform::Integer),
        column("code", "legacy_code", Transform::IntegerText),
        lookup("dcode", "drug_id", "drugs", "legacy_dcode", true, false),
        column("note", "note", Transform::Text),
    ]),
    mapping!("TblDrug Details2" => "drug_detail_items", [
        lookup("code", "detail_group_id", "drug_detail_groups", "id", true, false),
        column("detail", "detail", Transform::TextPlaceholder("[Legacy detail missing]")),
    ]),
    mapping!("Tblregimen details1" => "regimen_groups", [
        column("code", "id", Transform::Integer),
        column("code", "legacy_code", Transform::IntegerText),
        lookup("regcode", "regimen_id", "regimens", "legacy_regcode", true, false),
        column("note", "note", Transform::Text),
        column("cday", "cycle_day", Transform::Integer),
        column("ncycle", "cycle_count", Transform::Integer),
    ]),
    mapping!("Tblregimen details2" => "regimen_items", [
        lookup("code", "regimen_group_id", "regimen_groups", "id", true, false),
        lookup("drug", "drug_id", "drugs", "legacy_dcode", false, false),
        column("dose", "dose", Transform::OptionalParsedReal),
        column("dose", "legacy_dose_text", Transform::Text),
        column("unit", "unit_text", Transform::Text),
        column("route", "route_text", Transform::Text),
        column("details", "details", Transform::Text),
        column("group", "item_group", Transform::Text),
        column("duration", "duration", Transform::Text),
        column("StartD", "start_day", Transform::Integer),
        column("ordering", "ordering_no", Transform::Integer),
        lookup("dfdiluent", "default_diluent_id", "diluents", "legacy_dilcode", false, false),
        lookup("dfroute", "default_route_id", "routes", "legacy_rcode", false, false),
        column("dfrate", "default_rate", Transform::Text),
    ]),
    mapping!("Appoint" => "appointments", [
        column("appid", "id", Transform::Integer),
        column("appid", "legacy_appid", Transform::IntegerText),
        lookup("hn", "patient_id", "patients", "legacy_hn", true, true),
        column("appdate", "appointment_date", Transform::DateTime),
        lookup("diagcode", "diagnosis_id", "diagnoses", "legacy_diagcode", false, false),
        lookup("Regcode", "regimen_id", "regimens", "legacy_regcode", false, false),
        column("user", "legacy_user", Transform::Text),
        column("timerecord", "recorded_at", Transform::DateTime),
    ]),
    mapping!("Appoint details" => "appointment_items", [
        lookup("appid", "appointment_id", "appointments", "id", true, false),
        lookup("adcode", "drug_id", "drugs", "legacy_dcode", false, false),
        column("adose", "dose", Transform::Real),
        column("adetail", "detail", Transform::Text),
    ]),
    mapping!("TblAppCard" => "appointment_cards", [
        column("ccode", "legacy_ccode", Transform::IntegerText),
        lookup("Regimen", "regimen_id", "regimens", "legacy_regcode", false, false),
        column("aCyc", "cycle_no", Transform::Integer),
        column("aDay", "day_no", Transform::Integer),
        column("Day_no", "appointment_day", Transform::Integer),
    ]),
    mapping!("order" => "orders", [
        column("orderid", "id", Transform::Integer),
        column("orderid", "legacy_orderid", Transform::IntegerText),
        lookup("hn", "patient_id", "patients", "legacy_hn", true, true),
        lookup("wcode", "ward_id", "wards", "legacy_wcode", false, false),
        lookup("doccode", "doctor_id", "doctors", "legacy_doccode", false, false),
        constant("worker", Transform::ConstantInteger(0)),
        column("worker", "legacy_worker", Transform::Text),
        column("edit worker", "edit_worker", Transform::Text),
        column("note", "note", Transform::Text),
        column("ordertime", "order_time", Transform::DateTime),
        constant("side_effect_flag", Transform::ConstantInteger(0)),
        column("SideEffect", "side_effect_text", Transform::Text),
        column("SErecorder", "side_effect_recorder", Transform::Text),
        column("SErecordtime", "side_effect_record_time", Transform::DateTime),
        column("ME", "medication_error_text", Transform::Text),
        lookup("regcode", "regimen_id", "regimens", "legacy_regcode", false, false),
        column("Type", "order_type", Transform::Text),
        column("app", "appointment_flag", Transform::Boolean),
    ]),
    mapping!("order details" => "order_items", [
        lookup("orderid", "order_id", "orders", "id", true, false),
        lookup("dcode", "drug_id", "drugs", "legacy_dcode", true, false),
        lookup("dilcode", "diluent_id", "diluents", "legacy_dilcode", false, false),
        column("start", "start_date", Transform::Date),
        column("stop", "stop_date", Transform::Date),
        column("dose", "dose", Transform::Real),
        lookup("rcode", "route_id", "routes", "legacy_rcode", false, false),
        column("time", "schedule_time", Transform::Time),
        column("noofdrug", "number_of_drug", Transform::Real),
        column("missing", "missing", Transform::Boolean),
        column("print", "printed", Transform::Boolean),
        column("rate", "rate", Transform::Text),
        column("ordering", "ordering_no", Transform::Integer),
        column("running", "running_no", Transform::Integer),
        column("runsum", "running_sum", Transform::Integer),
        column("InvDate", "inventory_date", Transform::Date),
    ]),
    mapping!("InvIN" => "inventory_events", [
        column("Incode", "legacy_incode", Transform::IntegerText),
        lookup("dcode", "drug_id", "drugs", "legacy_dcode", true, false),
        column("In_no", "quantity", Transform::Real),
        column("Indate", "event_date", Transform::DateTime),
        column("InvOK", "inventory_ok", Transform::Boolean),
        column("SendOrder", "send_order", Transform::Boolean),
        column("user", "legacy_user", Transform::Text),
    ]),
    mapping!("TblSideEffect" => "side_effect_catalog", [
        column("SEcode", "id", Transform::Integer),
        column("SEcode", "legacy_secode", Transform::IntegerText),
        column("sideE", "side_effect_name", Transform::Text),
    ]),
    mapping!("TblGradingSE" => "side_effect_grades", [
        column("ID", "id", Transform::Integer),
        column("Adverse Event", "adverse_event", Transform::Text),
        column("Short Name", "short_name", Transform::Text),
        column("1", "grade1", Transform::Text),
        column("2", "grade2", Transform::Text),
        column("3", "grade3", Transform::Text),
        column("4", "grade4", Transform::Text),
        column("5", "grade5", Transform::Text),
        column("REMARK", "remark", Transform::Text),
        column("ALSO_CONSIDER", "also_consider", Transform::Text),
    ]),
    mapping!("SideEffect" => "side_effect_records", [
        column("orderid", "id", Transform::Integer),
        lookup("orderid", "order_id", "orders", "id", false, false),
        lookup("hn", "patient_id", "patients", "legacy_hn", true, true),
        lookup("SideEffect", "side_effect_id", "side_effect_catalog", "id", false, false),
        column("SEDate", "side_effect_date", Transform::Date),
        column("DrugAdminDate", "drug_admin_date", Transform::Date),
        column("SEmanage", "management", Transform::Text),
        column("DrugCause", "suspected_drug", Transform::Text),
        column("Grade", "grade", Transform::Text),
        column("SErecorder", "recorder", Transform::Text),
        column("SErecordtime", "record_time", Transform::DateTime),
    ]),
    mapping!("Drug Administration" => "drug_administration", skip_if_null = "hn", [
        lookup("hn", "patient_id", "patients", "legacy_hn", true, true),
        column("cycle", "cycle", Transform::OptionalParsedInteger),
        column("admindate", "administration_date", Transform::Date),
        column("sideeffect", "side_effect_flag", Transform::Boolean),
        column("details", "details", Transform::Text),
        column("recorder", "recorder", Transform::Text),
        column("recordtime", "record_time", Transform::DateTime),
    ]),
    mapping!("Pharmcare" => "pharmcare_soap", [
        column("SOAPcode", "id", Transform::Integer),
        column("SOAPcode", "legacy_soapcode", Transform::IntegerText),
        lookup("hn", "patient_id", "patients", "legacy_hn", false, true),
        column("Problem", "problem", Transform::Text),
        column("SOAPdate", "soap_date", Transform::DateTime),
        column("SOAPrec", "recorder", Transform::Text),
        column("SOAPnote", "note", Transform::Text),
        column("PBType", "problem_type", Transform::Text),
        column("S", "subjective", Transform::Text),
        column("O", "objective", Transform::Text),
        column("A", "assessment", Transform::Text),
        column("P", "plan_text", Transform::Text),
    ]),
    mapping!("PharmCareRec" => "pharmcare_records", [
        column("PRcode", "id", Transform::Integer),
        column("PRcode", "legacy_prcode", Transform::IntegerText),
        lookup("orderID", "order_id", "orders", "id", false, false),
        lookup("hn", "patient_id", "patients", "legacy_hn", false, true),
        column("visitdate", "visit_date", Transform::DateTime),
        column("P1", "p1", Transform::Boolean),
        column("P2", "p2", Transform::Boolean),
        column("P3", "p3", Transform::Boolean),
        column("P4", "p4", Transform::Boolean),
        column("P5", "p5", Transform::Boolean),
        column("P6", "p6", Transform::Boolean),
        column("P7", "p7", Transform::Boolean),
        column("P8", "p8", Transform::Boolean),
        column("P9", "p9", Transform::Boolean),
        column("PCareNote", "note", Transform::Text),
        column("UserPrac", "user_practice", Transform::Text),
        column("EditPrac", "edit_practice", Transform::Text),
    ]),
    mapping!("TblProblem" => "problem_catalog", [
        column("Problemcode", "id", Transform::Integer),
        column("Problemcode", "legacy_problemcode", Transform::IntegerText),
        column("problemname", "problem_name", Transform::Text),
    ]),
    mapping!("Problem" => "problems", [
        column("procode", "id", Transform::Integer),
        column("procode", "legacy_procode", Transform::IntegerText),
        lookup("HN", "patient_id", "patients", "legacy_hn", false, true),
        column("problem", "problem_code", Transform::IntegerText),
        column("pdate", "problem_date", Transform::Date),
        column("ptime", "problem_time", Transform::Time),
        column("pnote", "note", Transform::Text),
        column("Clear", "cleared", Transform::Boolean),
        column("clearby", "cleared_by", Transform::Text),
        column("pby", "problem_by", Transform::Text),
    ]),
    mapping!("Planning" => "plans", [
        column("planID", "id", Transform::Integer),
        column("planID", "legacy_planid", Transform::IntegerText),
        lookup("hn", "patient_id", "patients", "legacy_hn", false, true),
        column("Topic", "topic", Transform::Text),
        column("Planning", "plan_text", Transform::Text),
        column("plandate", "plan_date", Transform::DateTime),
        column("planby", "plan_by", Transform::Text),
        column("editby", "edit_by", Transform::Text),
        column("editdate", "edit_date", Transform::DateTime),
        column("Unhold", "inactive", Transform::Boolean),
        column("UnholdBy", "inactive_by", Transform::Text),
    ]),
    mapping!("TblDTPCat" => "dtp_categories", [
        column("ID", "id", Transform::Integer),
        column("ID", "legacy_id", Transform::IntegerText),
        column("Categories", "category", Transform::Text),
        column("Subcat1", "subcategory1", Transform::Text),
        column("Subcat2", "subcategory2", Transform::Text),
    ]),
    mapping!("Intervention" => "interventions", [
        column("IntCode", "id", Transform::Integer),
        column("IntCode", "legacy_intcode", Transform::IntegerText),
        lookup("hn", "patient_id", "patients", "legacy_hn", false, true),
        column("Intdate", "intervention_date", Transform::DateTime),
        lookup("DTP", "dtp_id", "dtp_categories", "id", false, false),
        column("DTPDetail", "dtp_detail", Transform::Text),
        column("IntTo", "intervention_to", Transform::Text),
        column("IntType", "intervention_type", Transform::Text),
        column("IntDetail", "intervention_detail", Transform::Text),
        column("Response", "response", Transform::Text),
        column("IntNote", "note", Transform::Text),
        column("IntBy", "intervention_by", Transform::Text),
        column("Int", "intervention_performed", Transform::Boolean),
    ]),
    mapping!("PNote" => "pharmacist_notes", [
        column("ncode", "id", Transform::Integer),
        column("ncode", "legacy_ncode", Transform::IntegerText),
        lookup("HN", "patient_id", "patients", "legacy_hn", false, true),
        column("NoteDate", "note_date", Transform::Date),
        column("NoteTime", "note_time", Transform::Time),
        column("PNote", "note", Transform::Text),
        column("Hold", "hold", Transform::Boolean),
        column("NoteBy", "note_by", Transform::Text),
        column("UnholdBy", "unhold_by", Transform::Text),
    ]),
    mapping!("MinMax" => "alert_settings", replace, [
        constant("id", Transform::ConstantInteger(1)),
        column("NoteAlert", "note_alert", Transform::Boolean),
        column("SEAlert", "side_effect_alert", Transform::Boolean),
        column("SOAPAlert", "soap_alert", Transform::Boolean),
        column("NewOrderAlert", "new_order_alert", Transform::Boolean),
        column("CycleAlert", "cycle_alert", Transform::Boolean),
        column("Plan", "plan_alert", Transform::Boolean),
        column("Platelet", "platelet_threshold", Transform::Real),
        column("Bilirubin", "bilirubin_threshold", Transform::Real),
        column("Creatinine", "creatinine_threshold", Transform::Real),
        column("Hosp", "hospital", Transform::Text),
        column("LabelNo", "label_number", Transform::Integer),
        column("WBC", "wbc_threshold", Transform::Real),
        column("ANC", "anc_threshold", Transform::Real),
        column("Hb", "haemoglobin_threshold", Transform::Real),
        column("AST", "ast_threshold", Transform::Real),
    ]),
    mapping!("AlertRec" => "alert_records", [
        lookup("HN", "patient_id", "patients", "legacy_hn", false, true),
        column("AlertCode", "alert_code", Transform::IntegerText),
        column("AlertDate", "alert_date", Transform::DateTime),
        column("AlertType", "alert_type", Transform::IntegerText),
        column("Manage", "management", Transform::Text),
        column("CurrUser", "current_user", Transform::Text),
        column("ViewNote", "view_note", Transform::Boolean),
        column("LabResDate", "lab_result_date", Transform::Text),
        column("Labtype", "lab_type", Transform::Text),
    ]),
];

pub(crate) fn import_mapped_table(
    source: &mut dyn SourceDatabase,
    transaction: &Transaction<'_>,
    mapping: &TableMapping,
    issues: &mut Vec<MigrationIssue>,
) -> Result<TableReport, ImportError> {
    let rows = source.read_table(mapping.source)?;
    let source_count = rows.len() as u64;
    let column_names = mapping
        .columns
        .iter()
        .map(|column| quote_identifier(column.destination))
        .collect::<Vec<_>>()
        .join(", ");
    let placeholders = (1..=mapping.columns.len())
        .map(|index| format!("?{index}"))
        .collect::<Vec<_>>()
        .join(", ");
    let verb = match mapping.mode {
        InsertMode::Insert => "INSERT",
        InsertMode::Replace => "INSERT OR REPLACE",
    };
    let sql = format!(
        "{verb} INTO {} ({column_names}) VALUES ({placeholders})",
        quote_identifier(mapping.destination)
    );
    let mut statement = transaction.prepare(&sql)?;
    let mut imported = 0;
    let mut skipped = 0;

    for (row_index, row) in rows.iter().enumerate() {
        if let Some(skip_column) = mapping.skip_if_null {
            if row.get(skip_column).is_none_or(SourceValue::is_null) {
                skipped += 1;
                issues.push(MigrationIssue {
                    severity: "warning".to_owned(),
                    category: "source_null_skipped".to_owned(),
                    source_table: Some(mapping.source.to_owned()),
                    destination_table: Some(mapping.destination.to_owned()),
                    row_number: Some((row_index + 1) as u64),
                    identifier: None,
                    message: format!(
                        "row was skipped because required source column '{skip_column}' is NULL"
                    ),
                });
                continue;
            }
        }
        let values = mapping
            .columns
            .iter()
            .map(|column| transform_value(transaction, mapping, column, row, row_index, issues))
            .collect::<Result<Vec<_>, _>>()?;
        statement
            .execute(params_from_iter(values.iter()))
            .map_err(|error| ImportError::row(mapping.source, row_index, error.to_string()))?;
        imported += 1;
    }

    Ok(TableReport {
        source_table: mapping.source.to_owned(),
        destination_table: Some(mapping.destination.to_owned()),
        source_row_count: source_count,
        imported_row_count: imported,
        skipped_row_count: skipped,
        error_count: skipped,
        synthetic_row_count: 0,
        status: if skipped == 0 {
            "imported"
        } else {
            "imported_with_skips"
        }
        .to_owned(),
        notes: Vec::new(),
    })
}

fn transform_value(
    transaction: &Transaction<'_>,
    table: &TableMapping,
    column: &ColumnMapping,
    row: &SourceRow,
    row_index: usize,
    issues: &mut Vec<MigrationIssue>,
) -> Result<Value, ImportError> {
    let source_value = match column.source {
        Some(source_column) => row.get(source_column).ok_or_else(|| {
            ImportError::row(
                table.source,
                row_index,
                format!("source column '{source_column}' was not returned by Access"),
            )
        })?,
        None => &SourceValue::Null,
    };

    let converted = match column.transform {
        Transform::Text => source_value.to_text().map(Value::Text),
        Transform::TextPlaceholder(placeholder) => match source_value.to_text() {
            Some(value) => Some(Value::Text(value)),
            None => {
                issues.push(MigrationIssue {
                    severity: "warning".to_owned(),
                    category: "required_text_placeholder".to_owned(),
                    source_table: Some(table.source.to_owned()),
                    destination_table: Some(table.destination.to_owned()),
                    row_number: Some((row_index + 1) as u64),
                    identifier: None,
                    message: format!(
                        "NULL source value for '{}' was replaced with a labeled compatibility placeholder",
                        column.destination
                    ),
                });
                Some(Value::Text(placeholder.to_owned()))
            }
        },
        Transform::Integer => source_value.to_integer().map(Value::Integer),
        Transform::Real => source_value.to_real().map(Value::Real),
        Transform::Boolean => source_value
            .to_boolean()
            .map(|value| Value::Integer(value.into())),
        Transform::BooleanText => source_value
            .to_boolean()
            .map(|value| Value::Text(if value { "1" } else { "0" }.to_owned())),
        Transform::IntegerText => source_value
            .to_integer()
            .map(|value| Value::Text(value.to_string())),
        Transform::Date => source_value
            .to_text()
            .map(|value| normalize_date(&value))
            .transpose()?
            .map(Value::Text),
        Transform::DateTime => source_value
            .to_text()
            .map(|value| normalize_datetime(&value))
            .transpose()?
            .map(Value::Text),
        Transform::Time => source_value
            .to_text()
            .map(|value| normalize_time(&value))
            .transpose()?
            .map(Value::Text),
        Transform::OptionalParsedInteger => source_value
            .to_text()
            .and_then(|value| value.trim().parse::<i64>().ok())
            .map(Value::Integer),
        Transform::OptionalParsedReal => source_value
            .to_text()
            .and_then(|value| value.trim().parse::<f64>().ok())
            .map(Value::Real),
        Transform::ConstantText(value) => Some(Value::Text(value.to_owned())),
        Transform::ConstantInteger(value) => Some(Value::Integer(value)),
        Transform::Lookup {
            table: lookup_table,
            key,
            required,
            identifier_is_sensitive,
        } => {
            let Some(lookup_value) = source_value.to_lookup_value() else {
                return if required {
                    Err(ImportError::row(
                        table.source,
                        row_index,
                        format!("required relationship '{}' is NULL", column.destination),
                    ))
                } else {
                    Ok(Value::Null)
                };
            };
            let sql = format!(
                "SELECT id FROM {} WHERE {} = ?1",
                quote_identifier(lookup_table),
                quote_identifier(key)
            );
            let result = transaction
                .query_row(&sql, params![lookup_value], |lookup_row| {
                    lookup_row.get::<_, i64>(0)
                })
                .optional()?;
            match result {
                Some(id) => Some(Value::Integer(id)),
                None if required => {
                    return Err(ImportError::row(
                        table.source,
                        row_index,
                        format!(
                            "required relationship '{}' has no destination row",
                            column.destination
                        ),
                    ));
                }
                None => {
                    issues.push(MigrationIssue {
                        severity: "warning".to_owned(),
                        category: "orphan_reference".to_owned(),
                        source_table: Some(table.source.to_owned()),
                        destination_table: Some(table.destination.to_owned()),
                        row_number: Some((row_index + 1) as u64),
                        identifier: if identifier_is_sensitive {
                            None
                        } else {
                            Some(lookup_value)
                        },
                        message: format!(
                            "optional relationship '{}' was stored as NULL because no destination row exists",
                            column.destination
                        ),
                    });
                    None
                }
            }
        }
    };

    Ok(converted.unwrap_or(Value::Null))
}

pub(crate) fn create_regimen_placeholders(
    transaction: &Transaction<'_>,
    regimen_group_rows: &[SourceRow],
    issues: &mut Vec<MigrationIssue>,
) -> Result<u64, ImportError> {
    let mut synthetic_regimens = 0;
    for row in regimen_group_rows {
        let Some(code) = row.get("regcode").and_then(SourceValue::to_integer) else {
            continue;
        };
        let legacy_code = code.to_string();
        let exists: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM regimens WHERE legacy_regcode = ?1)",
            params![legacy_code],
            |result| result.get(0),
        )?;
        if !exists {
            transaction.execute(
                "INSERT INTO regimens (id, legacy_regcode, regimen_name) VALUES (?1, ?2, ?3)",
                params![code, code.to_string(), "[Legacy missing regimen master]"],
            )?;
            synthetic_regimens += 1;
            issues.push(MigrationIssue::resolved_orphan(
                "Tblregimen details1",
                "regimens",
                code.to_string(),
                "created a synthetic regimen because the referenced legacy master row is missing",
            ));
        }
    }

    Ok(synthetic_regimens)
}

pub(crate) fn create_regimen_group_placeholders(
    transaction: &Transaction<'_>,
    regimen_item_rows: &[SourceRow],
    issues: &mut Vec<MigrationIssue>,
) -> Result<(u64, u64), ImportError> {
    let unresolved_regimen_id = -1_i64;
    let mut synthetic_groups = 0;
    for row in regimen_item_rows {
        let Some(code) = row.get("code").and_then(SourceValue::to_integer) else {
            continue;
        };
        let exists: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM regimen_groups WHERE id = ?1)",
            params![code],
            |result| result.get(0),
        )?;
        if !exists {
            transaction.execute(
                "INSERT OR IGNORE INTO regimens (id, legacy_regcode, regimen_name) VALUES (?1, ?2, ?3)",
                params![unresolved_regimen_id, "__unresolved_group_parent__", "[Legacy unresolved regimen group parent]"],
            )?;
            transaction.execute(
                "INSERT INTO regimen_groups (id, legacy_code, regimen_id, note) VALUES (?1, ?2, ?3, ?4)",
                params![code, code.to_string(), unresolved_regimen_id, "[Legacy missing regimen group]"],
            )?;
            synthetic_groups += 1;
            issues.push(MigrationIssue::resolved_orphan(
                "Tblregimen details2",
                "regimen_groups",
                code.to_string(),
                "created a synthetic regimen group because the referenced legacy group row is missing",
            ));
        }
    }

    Ok(((synthetic_groups > 0).into(), synthetic_groups))
}

pub(crate) fn create_drug_detail_placeholders(
    transaction: &Transaction<'_>,
    detail_rows: &[SourceRow],
    issues: &mut Vec<MigrationIssue>,
) -> Result<(u64, u64), ImportError> {
    let mut missing_codes = Vec::new();
    for row in detail_rows {
        let Some(code) = row.get("code").and_then(SourceValue::to_integer) else {
            continue;
        };
        let exists: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM drug_detail_groups WHERE id = ?1)",
            params![code],
            |result| result.get(0),
        )?;
        if !exists && !missing_codes.contains(&code) {
            missing_codes.push(code);
        }
    }
    if missing_codes.is_empty() {
        return Ok((0, 0));
    }

    transaction.execute(
        "INSERT INTO drugs (legacy_dcode, drug_name) VALUES (?1, ?2)",
        params![
            "__unresolved_detail_parent__",
            "[Legacy unresolved drug-detail parent]"
        ],
    )?;
    let drug_id = transaction.last_insert_rowid();
    for code in &missing_codes {
        transaction.execute(
            "INSERT INTO drug_detail_groups (id, legacy_code, drug_id, note) VALUES (?1, ?2, ?3, ?4)",
            params![
                code,
                code.to_string(),
                drug_id,
                "[Legacy missing drug-detail group]"
            ],
        )?;
        issues.push(MigrationIssue::resolved_orphan(
            "TblDrug Details2",
            "drug_detail_groups",
            code.to_string(),
            "created a synthetic drug-detail group because the referenced legacy group row is missing",
        ));
    }
    Ok((1, missing_codes.len() as u64))
}

pub(crate) fn quote_identifier(identifier: &str) -> String {
    format!("\"{}\"", identifier.replace('"', "\"\""))
}

pub(crate) fn normalize_boolean(value: &str) -> Result<bool, ImportError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "-1" | "1" | "true" | "yes" => Ok(true),
        "0" | "false" | "no" => Ok(false),
        other => Err(ImportError::Conversion(format!(
            "'{other}' is not a recognized Access Yes/No value"
        ))),
    }
}

pub(crate) fn normalize_date(value: &str) -> Result<String, ImportError> {
    let normalized = clean_timestamp(value);
    let date = normalized
        .get(0..10)
        .filter(|candidate| valid_date(candidate))
        .ok_or_else(|| {
            ImportError::Conversion("Access date is not in a supported format".into())
        })?;
    Ok(date.to_owned())
}

pub(crate) fn normalize_datetime(value: &str) -> Result<String, ImportError> {
    let normalized = clean_timestamp(value);
    let date = normalized
        .get(0..10)
        .filter(|candidate| valid_date(candidate))
        .ok_or_else(|| ImportError::Conversion("Access timestamp has an invalid date".into()))?;
    let time = normalized.get(11..19).unwrap_or("00:00:00");
    if !valid_time(time) {
        return Err(ImportError::Conversion(
            "Access timestamp has an invalid time".into(),
        ));
    }
    Ok(format!("{date}T{time}"))
}

pub(crate) fn normalize_time(value: &str) -> Result<String, ImportError> {
    let normalized = clean_timestamp(value);
    let candidate = if normalized.len() >= 19 {
        &normalized[11..19]
    } else if normalized.len() >= 8 {
        &normalized[0..8]
    } else {
        return Err(ImportError::Conversion(
            "Access time is not in a supported format".into(),
        ));
    };
    if !valid_time(candidate) {
        return Err(ImportError::Conversion("Access time is invalid".into()));
    }
    Ok(candidate.to_owned())
}

fn clean_timestamp(value: &str) -> String {
    value
        .trim()
        .trim_start_matches("{ts '")
        .trim_start_matches("{d '")
        .trim_end_matches("'}")
        .replace('T', " ")
}

fn valid_date(value: &str) -> bool {
    let parts = value
        .split('-')
        .map(str::parse::<u32>)
        .collect::<Result<Vec<_>, _>>();
    let Ok(parts) = parts else { return false };
    if parts.len() != 3 {
        return false;
    }
    let (year, month, day) = (parts[0], parts[1], parts[2]);
    if year == 0 || !(1..=12).contains(&month) {
        return false;
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days = match month {
        2 if leap => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    (1..=days).contains(&day)
}

fn valid_time(value: &str) -> bool {
    let parts = value
        .split(':')
        .map(str::parse::<u32>)
        .collect::<Result<Vec<_>, _>>();
    matches!(parts, Ok(parts) if parts.len() == 3 && parts[0] < 24 && parts[1] < 60 && parts[2] < 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_access_booleans() {
        for truthy in ["-1", "1", "True", "YES"] {
            assert!(normalize_boolean(truthy).unwrap());
        }
        for falsey in ["0", "False", "no"] {
            assert!(!normalize_boolean(falsey).unwrap());
        }
        assert!(normalize_boolean("maybe").is_err());
    }

    #[test]
    fn normalizes_access_dates_and_times() {
        assert_eq!(normalize_date("2009-02-10 14:12:04").unwrap(), "2009-02-10");
        assert_eq!(
            normalize_datetime("{ts '2009-02-10 14:12:04'}").unwrap(),
            "2009-02-10T14:12:04"
        );
        assert_eq!(normalize_time("1899-12-30 08:30:00").unwrap(), "08:30:00");
        assert!(normalize_date("2009-02-30").is_err());
    }

    #[test]
    fn preserves_thai_unicode_text() {
        let value = SourceValue::Text("ข้อมูลผู้ป่วย".to_owned());
        assert_eq!(value.to_text().as_deref(), Some("ข้อมูลผู้ป่วย"));
        assert!(!value.to_text().unwrap().contains('\u{fffd}'));
    }
}
