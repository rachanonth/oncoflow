mod auth;
mod clinical;
mod command_state;
mod db;
mod drug;
mod guidance;
mod hardware;
mod inventory;
pub mod lan;
mod master_data;
#[cfg(any(test, feature = "migration-cli"))]
pub mod migration;
mod order;
mod organization;
mod output;
mod patient;
mod preparation;
mod preparation_calc;
#[cfg(test)]
mod rc1_validation;
mod recovery;
mod regimen;
mod report;
mod safety;

use serde::Serialize;
use tauri::{Emitter, Manager, State};

use auth::AuthSession;
use db::Database;
use recovery::StartupState;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct HealthStatus {
    backend_running: bool,
    database_connected: bool,
    schema_version: i64,
}

#[tauri::command]
fn health_status(database: State<'_, Database>) -> Result<HealthStatus, String> {
    let schema_version = database
        .schema_version()
        .map_err(|_| "The local database health check failed.".to_string())?;

    Ok(HealthStatus {
        backend_running: true,
        database_connected: true,
        schema_version,
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let directory = app.path().app_data_dir()?;
            let connection = lan::ConnectionState::load(&directory)?;
            let client = connection.is_client();
            let host = lan::host::HostManager::new(&connection);
            if connection.is_host() { let _ = host.start(); }
            app.manage(host);
            app.manage(connection);
            if !client { app.manage(lan::lock_database(&directory)?); }
            let database_path = directory.join(db::DATABASE_FILENAME);
            let (database, startup) = if client {
                (Database::at_path(database_path), StartupState::ready())
            } else { match Database::initialize(&database_path) {
                Ok(database) => (database, StartupState::ready()),
                Err(error) => (
                    Database::at_path(database_path),
                    StartupState::failed(&error),
                ),
            } };
            app.manage(database);
            app.manage(startup);
            app.manage(AuthSession::default());
            Ok(())
        })
        .invoke_handler({
          let handler: fn(tauri::ipc::Invoke<tauri::Wry>) -> bool = tauri::generate_handler![
            lan::host::server_status,
            lan::host::start_host_server,
            lan::host::stop_host_server,
            lan::host::stop_server_and_exit,
            lan::host::export_server_certificate,
            lan::client::connection_settings,
            lan::client::save_connection_settings,
            lan::client::lan_invoke,
            auth::commands::get_auth_state,
            auth::commands::bootstrap_user,
            auth::commands::login,
            auth::commands::logout,
            auth::commands::get_current_user,
            auth::commands::change_password,
            auth::commands::reset_user_password,
            auth::commands::list_users,
            auth::commands::create_user,
            auth::commands::update_user,
            master_data::commands::list_doctors,
            master_data::commands::create_doctor,
            master_data::commands::update_doctor,
            master_data::commands::list_wards,
            master_data::commands::create_ward,
            master_data::commands::update_ward,
            master_data::commands::list_routes,
            master_data::commands::create_route,
            master_data::commands::update_route,
            master_data::commands::list_diluents,
            master_data::commands::create_diluent,
            master_data::commands::update_diluent,
            master_data::commands::list_diagnoses,
            master_data::commands::create_diagnosis,
            master_data::commands::update_diagnosis,
            guidance::commands::list_page_guidance,
            guidance::commands::update_page_guidance,
            organization::commands::get_application_settings,
            organization::commands::update_application_settings,
            health_status,
            patient::commands::list_patients,
            patient::commands::get_patient,
            patient::commands::create_patient,
            patient::commands::update_patient,
            patient::commands::patient_form_options,
            drug::commands::list_drugs,
            drug::commands::get_drug,
            drug::commands::create_drug,
            drug::commands::update_drug,
            drug::commands::drug_form_options,
            inventory::commands::list_inventory,
            inventory::commands::get_low_stock_items,
            inventory::commands::get_inventory_item,
            inventory::commands::list_inventory_movements,
            inventory::commands::record_inventory_receipt,
            inventory::commands::record_inventory_adjustment,
            inventory::commands::record_inventory_manual_issue,
            regimen::commands::list_regimens,
            regimen::commands::get_regimen,
            regimen::commands::create_regimen,
            regimen::commands::update_regimen,
            regimen::commands::add_regimen_group,
            regimen::commands::update_regimen_group,
            regimen::commands::delete_regimen_group,
            regimen::commands::add_regimen_item,
            regimen::commands::update_regimen_item,
            regimen::commands::delete_regimen_item,
            regimen::commands::reorder_regimen_items,
            regimen::commands::get_regimen_lookups,
            order::commands::list_orders,
            order::commands::list_patient_orders,
            order::commands::get_order,
            order::commands::get_order_cancellation_preview,
            order::commands::cancel_order,
            order::commands::create_order,
            order::commands::create_order_from_regimen,
            order::commands::update_order,
            order::commands::update_order_weight,
            order::commands::add_order_item,
            order::commands::update_order_item,
            order::commands::remove_order_item,
            order::commands::reorder_order_items,
            order::commands::get_order_lookups,
            order::commands::record_order_no_show,
            order::commands::reschedule_order,
            clinical::commands::clinical_standard_dose,
            clinical::commands::clinical_anc_cal,
            clinical::commands::clinical_anc_grade,
            clinical::commands::clinical_platelet,
            clinical::commands::clinical_lab_min_max,
            clinical::commands::clinical_fix_number,
            safety::commands::evaluate_order_safety,
            preparation::commands::list_preparation_queue,
            preparation::commands::get_preparation_workspace,
            preparation::commands::initialize_preparation,
            preparation::commands::update_preparation_task,
            preparation::commands::mark_preparation_prepared,
            preparation::commands::verify_preparation_task,
            preparation::commands::check_preparation_task,
            preparation::commands::check_preparation_tasks,
            preparation::commands::acknowledge_preparation_safety_finding,
            report::commands::get_preparation_count_report,
            report::commands::get_inventory_usage_report,
            report::commands::get_ward_delivery_report,
            report::commands::get_diluent_usage_report,
            output::commands::get_preparation_output,
            hardware::commands::list_system_printers,
            hardware::commands::list_system_label_fonts,
            hardware::commands::print_test_label,
            hardware::commands::print_preparation_label,
            hardware::commands::print_order_preparation_labels,
            hardware::commands::validate_printer_queue,
            recovery::commands::get_startup_status,
            recovery::commands::retry_database_initialization,
            recovery::commands::create_database_backup,
            recovery::commands::preflight_database_restore,
            recovery::commands::restore_database,
            recovery::commands::get_diagnostics,
            recovery::commands::open_data_folder,
          ];
          move |invoke: tauri::ipc::Invoke<tauri::Wry>| {
            let command = invoke.message.command();
            let client = invoke.message.state_ref().get::<lan::ConnectionState>().is_client();
            if client && !matches!(command, "connection_settings" | "save_connection_settings" | "lan_invoke" | "server_status" | "start_host_server" | "stop_host_server" | "stop_server_and_exit" | "export_server_certificate") {
                invoke.resolver.reject(serde_json::json!({"code":"client_mode", "message":"Use the configured LAN server for this operation."}));
                true
            } else { handler(invoke) }
          }
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.try_state::<lan::host::HostManager>().is_some_and(|host| host.running()) {
                    api.prevent_close();
                    let _ = window.emit("oncoflow-host-close-request", ());
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running OncoFlow");
}
