mod commands;
mod state;

use commands::{
    apply_fields_mapping, apply_mapping, apply_species_resolutions, apply_wide_mapping,
    derive_status, export, export_diagnostic_report, get_map_points, get_status_vocab, load_file,
    preview_file, resolve_species_tnrs, run_validation, save_text_file, use_raw_as_gfb3,
};
use state::AppState;

pub fn run() {
    tauri::Builder::default()
        .manage(AppState::default())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            load_file,
            preview_file,
            apply_mapping,
            apply_wide_mapping,
            apply_fields_mapping,
            derive_status,
            use_raw_as_gfb3,
            get_status_vocab,
            resolve_species_tnrs,
            apply_species_resolutions,
            run_validation,
            export_diagnostic_report,
            export,
            get_map_points,
            save_text_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
