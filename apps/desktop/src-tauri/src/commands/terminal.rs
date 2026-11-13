use portable_pty::{native_pty_system, Child, ChildKiller, CommandBuilder, MasterPty, PtySize, PtySystem};
use serde::Serialize;
use std::{
    collections::HashMap,
    io::{Read, Write},
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex,
    },
    thread,
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Emitter, Manager, State};

static NEXT_SESSION_ID: AtomicU64 = AtomicU64::new(1);

struct TerminalSession {
    writer: Box<dyn Write + Send>,
    killer: Box<dyn ChildKiller + Send + Sync>,
    master: Box<dyn MasterPty + Send>,
}

#[derive(Default)]
pub struct TerminalState {
    sessions: Mutex<HashMap<String, TerminalSession>>,
}

impl Drop for TerminalState {
    fn drop(&mut self) {
        if let Ok(sessions) = self.sessions.get_mut() {
            for (_, mut session) in sessions.drain() {
                let _ = session.killer.kill();
            }
        }
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TerminalOutput {
    session_id: String,
    data: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TerminalExit {
    session_id: String,
    code: Option<u32>,
}

fn make_session_id() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let sequence = NEXT_SESSION_ID.fetch_add(1, Ordering::Relaxed);
    format!("vm-{timestamp}-{sequence}")
}

#[tauri::command]
pub fn start_terminal(
    app: AppHandle,
    state: State<'_, TerminalState>,
    root: String,
    current_directory: String,
    cols: u16,
    rows: u16,
) -> Result<String, String> {
    let root = super::workspace::canonical_root(&root)?;
    let cwd = super::workspace::terminal_directory(&root, &current_directory)?;
    let size = PtySize {
        rows: rows.clamp(8, 200),
        cols: cols.clamp(20, 400),
        pixel_width: 0,
        pixel_height: 0,
    };

    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(size)
        .map_err(|error| format!("Could not create terminal: {error}"))?;

    #[cfg(windows)]
    let shell = std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".to_string());
    #[cfg(not(windows))]
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());

    let mut command = CommandBuilder::new(shell);
    command.cwd(&cwd);
    command.env("TERM", "xterm-256color");
    command.env("COLORTERM", "truecolor");

    let mut child = pair
        .slave
        .spawn_command(command)
        .map_err(|error| format!("Could not start shell: {error}"))?;
    drop(pair.slave);

    let reader = pair
        .master
        .try_clone_reader()
        .map_err(|error| format!("Could not read terminal output: {error}"))?;
    let writer = pair
        .master
        .take_writer()
        .map_err(|error| format!("Could not open terminal input: {error}"))?;
    let killer = child.clone_killer();
    let session_id = make_session_id();

    {
        let mut sessions = state
            .sessions
            .lock()
            .map_err(|_| "Terminal session manager is unavailable.".to_string())?;
        sessions.insert(
            session_id.clone(),
            TerminalSession {
                writer,
                killer,
                master: pair.master,
            },
        );
    }

    let output_app = app.clone();
    let output_session = session_id.clone();
    thread::spawn(move || {
        let mut reader = reader;
        let mut buffer = [0_u8; 8192];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(count) => {
                    let data = String::from_utf8_lossy(&buffer[..count]).into_owned();
                    let _ = output_app.emit(
                        "terminal://output",
                        TerminalOutput {
                            session_id: output_session.clone(),
                            data,
                        },
                    );
                }
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(error) => {
                    let _ = output_app.emit(
                        "terminal://output",
                        TerminalOutput {
                            session_id: output_session.clone(),
                            data: format!("\r\n[Terminal read error: {error}]\r\n"),
                        },
                    );
                    break;
                }
            }
        }
    });

    let exit_app = app;
    let exit_session = session_id.clone();
    thread::spawn(move || {
        let code = child.wait().ok().map(|status| status.exit_code());
        let _ = exit_app.emit(
            "terminal://exit",
            TerminalExit {
                session_id: exit_session.clone(),
                code,
            },
        );
        if let Some(manager) = exit_app.try_state::<TerminalState>() {
            if let Ok(mut sessions) = manager.sessions.lock() {
                sessions.remove(&exit_session);
            }
        }
    });

    Ok(session_id)
}

#[tauri::command]
pub fn write_terminal_input(
    state: State<'_, TerminalState>,
    session_id: String,
    data: String,
) -> Result<(), String> {
    if data.len() > 65_536 {
        return Err("Terminal input is too large.".into());
    }
    let mut sessions = state
        .sessions
        .lock()
        .map_err(|_| "Terminal session manager is unavailable.".to_string())?;
    let session = sessions
        .get_mut(&session_id)
        .ok_or_else(|| "Terminal session is no longer running.".to_string())?;
    session
        .writer
        .write_all(data.as_bytes())
        .and_then(|_| session.writer.flush())
        .map_err(|error| format!("Could not write to terminal: {error}"))
}

#[tauri::command]
pub fn resize_terminal(
    state: State<'_, TerminalState>,
    session_id: String,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    let sessions = state
        .sessions
        .lock()
        .map_err(|_| "Terminal session manager is unavailable.".to_string())?;
    let session = sessions
        .get(&session_id)
        .ok_or_else(|| "Terminal session is no longer running.".to_string())?;
    session
        .master
        .resize(PtySize {
            rows: rows.clamp(8, 200),
            cols: cols.clamp(20, 400),
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|error| format!("Could not resize terminal: {error}"))
}

#[tauri::command]
pub fn close_terminal(
    state: State<'_, TerminalState>,
    session_id: String,
) -> Result<(), String> {
    let mut sessions = state
        .sessions
        .lock()
        .map_err(|_| "Terminal session manager is unavailable.".to_string())?;
    if let Some(mut session) = sessions.remove(&session_id) {
        session
            .killer
            .kill()
            .map_err(|error| format!("Could not stop terminal: {error}"))?;
    }
    Ok(())
}
