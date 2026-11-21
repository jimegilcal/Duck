use tauri::{AppHandle, WebviewWindow};

#[cfg(target_os = "macos")]
use {
    serde::Serialize,
    std::ffi::{c_char, c_void, CStr, CString},
    tauri::Emitter,
};

#[cfg(target_os = "macos")]
type Callback = unsafe extern "C" fn(*mut c_void, i32, *const c_char);

#[cfg(target_os = "macos")]
extern "C" {
    fn duck_start_dictation(id: *const c_char, window: u8, context: *mut c_void, callback: Callback);
    fn duck_end_dictation(id: *const c_char, window: u8, cancel: u8);
    fn duck_cancel_window_dictation(window: u8);
}

#[cfg(target_os = "macos")]
struct Connection {
    window: WebviewWindow,
    id: String,
}

#[cfg(target_os = "macos")]
#[derive(Clone, Serialize)]
struct DictationEvent<'a> {
    id: &'a str,
    kind: &'static str,
    text: String,
}

#[cfg(target_os = "macos")]
unsafe extern "C" fn handle_event(context: *mut c_void, code: i32, text: *const c_char) {
    let kind = match code {
        0 => "listening",
        1 => "partial",
        2 => "finished",
        3 => "failed",
        4 => "cancelled",
        5 => "downloading",
        6 => "transcribing",
        _ => return,
    };
    let context = context.cast::<Connection>();
    let connection = unsafe { &*context };
    let text = if text.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(text) }.to_string_lossy().into_owned()
    };
    let _ = connection.window.emit_to(
        connection.window.label(),
        "dictation",
        DictationEvent { id: &connection.id, kind, text },
    );
    // The native bridge releases the connection exactly once, on a terminal event.
    if (2..=4).contains(&code) {
        drop(unsafe { Box::from_raw(context) });
    }
}

#[cfg(target_os = "macos")]
fn window_id(label: &str) -> Result<u8, String> {
    match label {
        "duck" => Ok(0),
        "panel" => Ok(1),
        _ => Err("El dictado solo está disponible en el chat de Duck.".into()),
    }
}

#[cfg(target_os = "macos")]
fn native_id(id: &str) -> Result<CString, String> {
    let id = uuid::Uuid::parse_str(id).map_err(|_| "La sesión de dictado no es válida.")?;
    CString::new(id.to_string()).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn start_dictation(app: AppHandle, window: WebviewWindow, id: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let owner = window_id(window.label())?;
        let native_id = native_id(&id)?;
        app.run_on_main_thread(move || {
            let context = Box::into_raw(Box::new(Connection { window, id })).cast();
            unsafe { duck_start_dictation(native_id.as_ptr(), owner, context, handle_event) };
        }).map_err(|error| error.to_string())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (app, window, id);
        Err("El dictado está disponible en macOS.".into())
    }
}

#[tauri::command]
pub fn end_dictation(app: AppHandle, window: WebviewWindow, id: String, cancel: bool) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let owner = window_id(window.label())?;
        let id = native_id(&id)?;
        app.run_on_main_thread(move || unsafe { duck_end_dictation(id.as_ptr(), owner, u8::from(cancel)) })
            .map_err(|error| error.to_string())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (app, window, id, cancel);
        Ok(())
    }
}

pub(crate) fn cancel_window(app: &AppHandle, label: &str) {
    #[cfg(target_os = "macos")]
    if let Ok(owner) = window_id(label) {
        let _ = app.run_on_main_thread(move || unsafe { duck_cancel_window_dictation(owner) });
    }
    #[cfg(not(target_os = "macos"))]
    let _ = (app, label);
}
