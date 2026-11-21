#[cfg(target_os = "macos")]
use tauri_nspanel::{ManagerExt, WebviewWindowExt};

#[cfg(target_os = "macos")]
tauri_nspanel::tauri_panel! {
    panel!(DuckFloatingPanel {
        config: {
            can_become_key_window: true,
            can_become_main_window: false,
            is_floating_panel: true
        }
    })
}

#[cfg(target_os = "macos")]
pub fn attach(window: &tauri::WebviewWindow) -> Result<(), String> {
    use tauri_nspanel::objc2_app_kit::NSWindowStyleMask;

    let panel = window.to_panel::<DuckFloatingPanel>().map_err(|error| error.to_string())?;
    panel.set_released_when_closed(false);
    panel.set_floating_panel(true);
    panel.set_becomes_key_only_if_needed(true);
    panel.set_hides_on_deactivate(false);
    panel.add_style_mask(NSWindowStyleMask::NonactivatingPanel).map_err(|error| error.to_string())
}

#[cfg(target_os = "macos")]
pub fn detach(app: &tauri::AppHandle) {
    if let Ok(panel) = app.get_webview_panel("duck") {
        let _ = panel.to_window();
    }
}
