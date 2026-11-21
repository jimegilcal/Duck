use crate::desktop::{self, DesktopState};
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, Submenu},
    AppHandle, Manager, WebviewWindow,
};

pub const ACCESSORIES: &[(&str, &str)] = &[
    ("classic", "Sin accesorio"),
    ("glasses", "Gafas redondas"),
    ("party", "Gorro de fiesta"),
    ("bow", "Pajarita rosa"),
    ("frog", "Gorro de ranita"),
    ("headphones", "Auriculares morados"),
];

const SIZE_PRESETS: &[(u32, &str)] = &[
    (64, "Mini"),
    (96, "Muy pequeño"),
    (144, "Pequeño"),
    (210, "Mediano"),
    (280, "Grande"),
    (360, "Muy grande"),
    (480, "Gigante"),
];

fn fill_size_presets(window: &WebviewWindow, add: impl Fn(&CheckMenuItem<tauri::Wry>) -> tauri::Result<()>) -> tauri::Result<()> {
    let app = window.app_handle();
    let size = desktop::get_settings(app.state::<DesktopState>()).size;
    for &(points, label) in SIZE_PRESETS {
        let item = CheckMenuItem::with_id(
            app, format!("duck-size-{points}"), format!("{label} · {points}"),
            true, points == size, None::<&str>,
        )?;
        add(&item)?;
    }
    Ok(())
}

#[tauri::command]
pub fn show_duck_sizes(window: WebviewWindow) -> Result<(), String> {
    if window.label() != "duck" {
        return Err("El menú se abre desde la mascota.".into());
    }
    let menu = Menu::new(window.app_handle()).map_err(|error| error.to_string())?;
    fill_size_presets(&window, |item| menu.append(item)).map_err(|error| error.to_string())?;
    window.popup_menu(&menu).map_err(|error| error.to_string())
}


#[tauri::command]
pub fn show_duck_menu(window: WebviewWindow) -> Result<(), String> {
    if window.label() != "duck" {
        return Err("El menú se abre desde la mascota.".into());
    }
    show_menu(&window).map_err(|error| error.to_string())
}

fn show_menu(window: &WebviewWindow) -> tauri::Result<()> {
    let app = window.app_handle();
    let (settings, chat_open) = {
        let state = app.state::<DesktopState>();
        let desktop = state.lock().unwrap();
        (desktop.settings.clone(), desktop.chat_open)
    };
    let chat_label = if chat_open { "Ocultar chat" } else { "Mostrar chat" };
    let chat = MenuItem::with_id(app, "toggle-chat", chat_label, true, None::<&str>)?;
    let smaller = MenuItem::with_id(
        app, "size-down", "Hacer más pequeño", settings.size > desktop::MIN_SIZE, None::<&str>,
    )?;
    let larger = MenuItem::with_id(
        app, "size-up", "Hacer más grande", settings.size < desktop::MAX_SIZE, None::<&str>,
    )?;
    let sizes = Submenu::new(app, "Tamaños", true)?;
    fill_size_presets(window, |item| sizes.append(item))?;
    let exact = MenuItem::with_id(app, "appearance-settings", "Elegir tamaño exacto…", true, None::<&str>)?;
    let accessories = Submenu::new(app, "Accesorios", true)?;
    for &(id, label) in ACCESSORIES {
        let item = CheckMenuItem::with_id(
            app, format!("accessory-{id}"), label, true, id == settings.skin, None::<&str>,
        )?;
        accessories.append(&item)?;
    }
    let guide = MenuItem::with_id(app, "guide", "Guía de Duck…", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&chat, &smaller, &larger, &sizes, &accessories, &exact, &guide])?;
    window.popup_menu(&menu)
}

pub fn handle_menu_event(app: &AppHandle, id: &str) -> Result<(), String> {
    match id {
        "toggle-chat" => desktop::toggle_chat(app.clone()),
        "appearance-settings" => desktop::navigate_panel(app.clone(), "settings".into()),
        "guide" => desktop::navigate_panel(app.clone(), "guide".into()),
        "bring-duck" => desktop::bring_duck(app.clone()),
        "size-up" => desktop::adjust_duck_size(app.clone(), true).map(|_| ()),
        "size-down" => desktop::adjust_duck_size(app.clone(), false).map(|_| ()) ,
        _ => {
            let mut settings = desktop::get_settings(app.state::<DesktopState>());
            if let Some(points) = id.strip_prefix("duck-size-").and_then(|value| value.parse::<u32>().ok()) {
                if !SIZE_PRESETS.iter().any(|(size, _)| *size == points) { return Ok(()); }
                settings.size = points;
            } else {
                match id.strip_prefix("accessory-") {
                    Some(skin) if ACCESSORIES.iter().any(|(key, _)| *key == skin) => {
                        settings.skin = skin.into();
                    }
                    _ => return Ok(()),
                }
            }
            desktop::save_settings(app.clone(), settings)?;
            Ok(())
        }
    }
}
