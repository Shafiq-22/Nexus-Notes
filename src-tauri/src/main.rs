// Hide the extra console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod error;
mod events;
mod fs_watch;
mod index;
mod model;
mod paths;
mod state;
mod vault;
mod write_guard;

use state::AppState;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::open_vault,
            commands::close_vault,
            commands::startup_vault,
            commands::reindex,
            commands::get_vault_tree,
            commands::read_note,
            commands::write_note,
            commands::create_note,
            commands::create_folder,
            commands::rename_path,
            commands::move_path,
            commands::delete_path,
            commands::search,
            commands::search_suggest,
            commands::get_backlinks,
            commands::get_outline,
            commands::list_tags,
            commands::notes_with_tag,
            commands::graph_global,
            commands::graph_local,
            commands::list_databases,
            commands::get_database,
            commands::query_database,
            commands::upsert_record,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Nexus Notes");
}
