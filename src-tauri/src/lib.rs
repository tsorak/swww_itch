mod helper;
mod swww_ffi;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn set_background(name: &str) -> Result<bool, bool> {
    let unchecked_path = helper::background::canonicalize(name).map_err(|_| false)?;

    let path = match unchecked_path.try_exists() {
        Ok(true) => Ok(unchecked_path),
        _ => Err(false),
    }?;

    let path = path.to_str().ok_or(false)?;

    Ok(swww_ffi::set_background(path))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![greet, set_background])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
