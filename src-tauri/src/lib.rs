use std::sync::Mutex;
use tauri::State;

use swww_itch_shared::unix_socket::{self, Message};

mod api;

#[derive(Default)]
struct AppState {
    pub itchd_socket: unix_socket::UnixSocket,
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {name}! You've been greeted from Rust!")
}

#[tauri::command]
fn set_background(state: State<'_, Mutex<AppState>>, name: &str) -> Result<bool, bool> {
    let unchecked_path = api::quick_switch::background::canonicalize(name).map_err(|_| false)?;

    let path = match unchecked_path.try_exists() {
        Ok(true) => Ok(unchecked_path),
        _ => Err(false),
    }?;

    let path = path.to_str().ok_or(false)?.to_string();

    let mut lock = state.lock().unwrap();
    tauri::async_runtime::block_on(async move {
        let _ = lock
            .itchd_socket
            .send(Message::SwitchToBackground(path))
            .await;
    });

    Ok(true)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut app_state = AppState::default();
    let _ = app_state.itchd_socket.connect();

    tauri::Builder::default()
        .manage(Mutex::new(app_state))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![greet, set_background])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
