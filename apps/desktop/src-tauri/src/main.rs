mod commands;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::workspace::app_version,
            commands::workspace::list_directory,
            commands::workspace::read_text_file,
            commands::workspace::write_text_file
        ])
        .run(tauri::generate_context!())
        .expect("error while running Void Memingo");
}
