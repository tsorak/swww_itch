use tauri::State;
use tokio::sync::Mutex;

use swww_itch_shared::unix_socket::{self, Request, Response};

mod api;

struct AppState {
    pub itchd_socket: unix_socket::UnixSocket<Request, Response>,
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {name}! You've been greeted from Rust!")
}

#[tauri::command]
fn set_background(state: State<'_, Mutex<AppState>>, name: &str) -> anyhow::Result<bool, String> {
    let unchecked_path =
        api::quick_switch::background::canonicalize(name).map_err(|err| err.to_string())?;

    let path = match unchecked_path.try_exists() {
        Ok(true) => Ok(unchecked_path),
        _ => Err("Specified background does not exist"),
    }?;

    let path = path
        .to_str()
        .ok_or("Path contains invalid characters".to_string())?
        .to_string();

    tauri::async_runtime::block_on(async move {
        let mut lock = state.lock().await;

        let conn = lock
            .itchd_socket
            .connection
            .as_mut()
            .ok_or("Not connected")?;

        conn.send_request(Request::SwitchToBackground(path))
            .map_err(|err| err.to_string())?;

        let Response::SwitchToBackground(b) = conn
            .take_response(|r| matches!(r, Response::SwitchToBackground(_)))
            .await
        else {
            unreachable!()
        };

        Ok(b)
    })
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct RearrangeBackgroundResult {
    move_index: usize,
    to_index: usize,
}

#[tauri::command]
async fn rearrange_background(
    state: State<'_, Mutex<AppState>>,
    bg: String,
    before_or_after: String,
    target_bg: String,
) -> Result<RearrangeBackgroundResult, String> {
    let mut lock = state.lock().await;

    let conn = lock
        .itchd_socket
        .connection
        .as_mut()
        .ok_or("Not connected")?;

    conn.send_request(Request::RearrangeBackground((
        bg,
        before_or_after,
        target_bg,
    )))
    .map_err(|err| err.to_string())?;

    let Response::RearrangeBackground((ok, move_index, to_index)) = conn
        .take_response(|r| matches!(r, Response::RearrangeBackground((_, _, _))))
        .await
    else {
        unreachable!()
    };

    if !ok {
        Err("Failed to rearrange background".to_string())
    } else {
        Ok(RearrangeBackgroundResult {
            move_index,
            to_index,
        })
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app_state = AppState {
        // Wrapped in a block_on to create a runtime for tokio calls down the callstack
        itchd_socket: tauri::async_runtime::block_on(async { unix_socket::connect() }),
    };

    tauri::Builder::default()
        .manage(Mutex::new(app_state))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            set_background,
            rearrange_background
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
