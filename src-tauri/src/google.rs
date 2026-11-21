#[tauri::command]
pub async fn open_google_tasks() -> Result<(), String> {
    let status = std::process::Command::new("/usr/bin/open")
        .arg("https://calendar.google.com/calendar/u/0/r/tasks")
        .status()
        .map_err(|error| error.to_string())?;
    if status.success() { Ok(()) } else { Err("No se pudo abrir Google Tasks.".into()) }
}
