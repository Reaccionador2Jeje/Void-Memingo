mod commands;

fn main() {
    tauri::Builder::default()
        .manage(commands::terminal::TerminalState::default())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::workspace::app_version,
            commands::workspace::list_directory,
            commands::workspace::read_text_file,
            commands::workspace::write_text_file,
            commands::workspace::resolve_terminal_directory,
            commands::workspace::run_terminal_command,
            commands::terminal::start_terminal,
            commands::terminal::write_terminal_input,
            commands::terminal::resize_terminal,
            commands::terminal::close_terminal
        ])
        .run(tauri::generate_context!())
        .expect("error while running Void Memingo");
}
