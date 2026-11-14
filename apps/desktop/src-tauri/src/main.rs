mod commands;

fn main() {
    tauri::Builder::default()
        .manage(commands::terminal::TerminalState::default())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::git::git_clone_repository,
            commands::git::git_snapshot,
            commands::git::git_init,
            commands::git::git_stage_all,
            commands::git::git_commit,
            commands::git::git_add_remote,
            commands::git::git_create_branch,
            commands::git::git_checkout_branch,
            commands::git::git_fetch_all,
            commands::git::git_pull_ff_only,
            commands::git::git_push,
            commands::workspace::app_version,
            commands::workspace::list_directory,
            commands::workspace::search_workspace,
            commands::workspace::read_text_file,
            commands::workspace::write_text_file,
            commands::workspace::resolve_terminal_directory,
            commands::workspace::run_terminal_command,
            commands::terminal::start_terminal,
            commands::terminal::write_terminal_input,
            commands::terminal::resize_terminal,
            commands::terminal::close_terminal,
            commands::os_builder::list_os_base_capabilities,
            commands::os_builder::check_os_build_preflight,
            commands::os_builder::validate_os_project,
            commands::os_builder::prepare_debian_live_project,
            commands::os_builder::prepare_os_build_project,
            commands::os_builder::validate_os_build_artifact
        ])
        .run(tauri::generate_context!())
        .expect("error while running Void Memingo");
}
