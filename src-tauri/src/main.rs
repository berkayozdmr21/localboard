#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use localboard_core::Board;
use serde_json::Value;
use std::sync::Mutex;
use tauri::Manager;
use tauri_plugin_dialog::DialogExt;

#[tauri::command]
async fn dispatch(
    board: tauri::State<'_, Mutex<Board>>,
    action: String,
    args: Value,
) -> Result<Value, String> {
    board
        .lock()
        .map_err(|_| "Kütüphane kilidi alınamadı.".to_string())?
        .dispatch(&action, args)
}
#[tauri::command]
async fn choose_root(app: tauri::AppHandle) -> Option<String> {
    app.dialog()
        .file()
        .blocking_pick_folder()
        .map(|p| p.to_string())
}
fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let path = app.path().app_data_dir()?;
            app.manage(Mutex::new(
                Board::open(path).map_err(std::io::Error::other)?,
            ));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![dispatch, choose_root])
        .run(tauri::generate_context!())
        .expect("LocalBoard failed to start");
}
