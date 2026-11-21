use chrono::{DateTime, Utc};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::ffi::{c_char, CStr, CString};
use tauri::{AppHandle, Emitter, Manager};

use crate::{
    desktop::{self, DesktopState},
    storage::Db,
};

#[derive(Deserialize, Serialize)]
pub struct Calendar {
    id: String,
    pub(crate) title: String,
    account: String,
}

#[cfg(target_os = "macos")]
extern "C" {
    fn duck_list_calendars() -> *mut c_char;
    fn duck_calendar_detail(identifier: *const c_char) -> *mut c_char;
    fn duck_commit_event(identifier: *const c_char, title: *const c_char, start: f64, end: f64) -> *mut c_char;
    fn duck_free_calendar_reply(result: *mut c_char);
}

#[derive(Deserialize)]
#[serde(untagged)]
enum NativeReply<T> {
    Failure { error: String },
    Success { value: T },
}

#[cfg(target_os = "macos")]
fn read_native<T: serde::de::DeserializeOwned>(result: *mut c_char) -> Result<T, String> {
    if result.is_null() { return Err("No se pudo contactar con Calendario.".into()); }
    // The bridge returns an owned, NUL-terminated UTF-8 JSON buffer.
    let decoded = unsafe {
        let decoded = serde_json::from_slice(CStr::from_ptr(result).to_bytes());
        duck_free_calendar_reply(result);
        decoded
    }.map_err(|_| "Calendario devolvió una respuesta inválida.".to_string())?;
    match decoded {
        NativeReply::Success { value } => Ok(value),
        NativeReply::Failure { error } => Err(error),
    }
}

fn c_string(value: &str) -> Result<CString, String> {
    CString::new(value).map_err(|_| "El texto contiene caracteres inválidos.".into())
}

fn detail(id: &str) -> Result<Option<Calendar>, String> {
    let id = c_string(id)?;
    read_native(unsafe { duck_calendar_detail(id.as_ptr()) })
}

#[tauri::command]
pub async fn current_calendar(app: AppHandle) -> Result<Option<Calendar>, String> {
    let id = desktop::get_settings(app.state::<DesktopState>()).calendar_id;
    match id {
        Some(id) => tauri::async_runtime::spawn_blocking(move || detail(&id)).await.map_err(crate::storage::err)?,
        None => Ok(None),
    }
}

#[tauri::command]
pub async fn list_calendars() -> Result<Vec<Calendar>, String> {
    tauri::async_runtime::spawn_blocking(|| read_native(unsafe { duck_list_calendars() }))
        .await.map_err(crate::storage::err)?
}

#[tauri::command]
pub async fn select_calendar(app: AppHandle, id: String) -> Result<Calendar, String> {
    let choice = tauri::async_runtime::spawn_blocking(move || detail(&id)).await.map_err(crate::storage::err)??
        .ok_or("El calendario elegido ya no está disponible.")?;
    let mut settings = desktop::get_settings(app.state::<DesktopState>());
    settings.calendar_id = Some(choice.id.clone());
    desktop::save_settings(app, settings)?;
    Ok(choice)
}

pub(crate) fn validate_event(title: &str, start: &str, end: &str) -> Result<(CString, DateTime<Utc>, DateTime<Utc>), String> {
    let title = title.trim();
    if title.is_empty() || title.chars().count() > 240 {
        return Err("Escribe un título de hasta 240 caracteres.".into());
    }
    let parse = |value: &str| {
        DateTime::parse_from_rfc3339(value)
            .map(|date| date.with_timezone(&Utc))
            .map_err(|_| "Revisa la fecha y hora del evento.".to_string())
    };
    let start = parse(start)?;
    let end = parse(end)?;
    if end <= start { return Err("La hora de fin tiene que ser posterior al inicio.".into()); }
    Ok((c_string(title)?, start, end))
}

fn claim(connection: &Connection, id: &str) -> Result<(), String> {
    let changed = connection.execute(
        "UPDATE proposals SET status='saving' WHERE id=?1 AND kind='event' AND status='pending_connection'",
        [id],
    ).map_err(crate::storage::err)?;
    if changed == 1 { Ok(()) } else {
        Err("Este borrador ya se está guardando, se guardó o se descartó. Si hay un guardado interrumpido, comprueba primero tu calendario.".into())
    }
}

#[tauri::command]
pub async fn save_calendar_event(
    app: AppHandle,
    proposal_id: String,
    expected_calendar_id: String,
    title: String,
    start: String,
    end: String,
) -> Result<Calendar, String> {
    let (title, start, end) = validate_event(&title, &start, &end)?;
    let selected = desktop::get_settings(app.state::<DesktopState>()).calendar_id
        .ok_or("Elige dónde quieres guardar el evento.")?;
    if selected != expected_calendar_id {
        return Err("El calendario cambió. Vuelve a elegirlo antes de guardar.".into());
    }
    let selected = c_string(&selected)?;

    tauri::async_runtime::spawn_blocking(move || {
        let db = app.state::<Db>();
        {
            let connection = db.lock().map_err(crate::storage::err)?;
            claim(&connection, &proposal_id)?;
        }
        let saved = read_native(unsafe {
            duck_commit_event(selected.as_ptr(), title.as_ptr(), start.timestamp() as f64, end.timestamp() as f64)
        });
        let status = if saved.is_ok() { "saved" } else { "pending_connection" };
        let connection = db.lock().map_err(crate::storage::err)?;
        connection.execute(
            "UPDATE proposals SET status=?1 WHERE id=?2 AND status='saving'",
            rusqlite::params![status, proposal_id],
        ).map_err(|error| {
            if saved.is_ok() {
                "Calendario guardó el evento. Duck no pudo actualizar el borrador. Comprueba tu calendario antes de repetirlo.".into()
            } else { crate::storage::err(error) }
        })?;
        drop(connection);
        let _ = app.emit("data-changed", ());
        saved
    }).await.map_err(crate::storage::err)?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_dates_in_the_wrong_order() {
        assert!(validate_event("Reunión", "2026-11-19T12:00:00Z", "2026-11-19T12:00:00Z").is_err());
        let (_, start, end) = validate_event("Café & revisión", "2026-11-19T12:00:00+01:00", "2026-11-19T12:30:00+01:00").unwrap();
        assert_eq!((end - start).num_minutes(), 30);
        assert_eq!(start.to_rfc3339(), "2026-11-19T11:00:00+00:00");
    }

    #[test]
    fn a_single_pending_event_can_be_claimed_once() {
        let c = crate::storage::open(std::path::Path::new(":memory:")).unwrap();
        c.execute("INSERT INTO proposals(id,kind,text,created_at) VALUES ('one','event','Café','now'),('two','task','Tarea','now')", []).unwrap();
        assert!(claim(&c, "one").is_ok());
        assert!(claim(&c, "one").is_err());
        assert!(claim(&c, "two").is_err());
    }
}
