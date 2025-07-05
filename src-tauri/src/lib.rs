use std::sync::Mutex;
use tauri::State;

mod api;

#[derive(Debug, Default)]
struct AppState {
    pub count: u32,
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn set_background(state: State<'_, Mutex<AppState>>, name: &str) -> Result<bool, bool> {
    let mut state = state.lock().unwrap();
    state.count += 1;
    dbg!(&state);

    let unchecked_path = api::quick_switch::background::canonicalize(name).map_err(|_| false)?;

    let path = match unchecked_path.try_exists() {
        Ok(true) => Ok(unchecked_path),
        _ => Err(false),
    }?;

    let path = path.to_str().ok_or(false)?;

    Ok(api::swww_ffi::set_background(path))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(Mutex::new(AppState::default()))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![greet, set_background])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
