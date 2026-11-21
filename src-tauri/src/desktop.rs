use serde::{Deserialize, Serialize};
use std::{sync::Mutex, time::{Duration, Instant}};
use tauri::{AppHandle, Emitter, Manager, LogicalSize, PhysicalPosition};
#[cfg(not(target_os = "macos"))]
use tauri::PhysicalSize;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

#[derive(Clone, Serialize, Deserialize)]
#[serde(default, rename_all="camelCase")]
pub struct Settings {
    pub visible: bool, pub size: u32,
    #[serde(default)]
    pub size_schema: u8,
    pub animations: bool, pub notifications: bool,
    pub companion_alerts: bool, pub shortcut: String, pub skin: String, pub calendar_id: Option<String>, pub position: Option<(i32,i32)>,
    pub usage_tracking: bool,
}
impl Default for Settings {
    fn default()->Self { Self {visible:true,size:144,size_schema:2,animations:true,notifications:true,companion_alerts:true,shortcut:"CommandOrControl+Shift+D".into(),skin:"classic".into(),calendar_id:None,position:None,usage_tracking:false} }
}
pub struct Desktop { pub settings: Settings, pub chat_open: bool, chat_layout: Option<ChatLayout>, pub chat_reply: bool, pub calendar_form: bool, pub panel_page: Option<String>, pub alpha: Vec<u8>, pub temporary_until: Option<Instant>, pub leaving: Option<Instant>, pub position_dirty: Option<Instant>, pub dragging_until: Option<Instant> }
pub type DesktopState=Mutex<Desktop>;
fn err(e:impl std::fmt::Display)->String {e.to_string()}
pub fn persist(app:&AppHandle, settings:&Settings)->Result<(),String> {
    static WRITE_LOCK: Mutex<()> = Mutex::new(());
    let _write=WRITE_LOCK.lock().map_err(err)?;
    let dir=app.path().app_data_dir().map_err(err)?;
    std::fs::create_dir_all(&dir).map_err(err)?;
    std::fs::write(dir.join("settings.tmp"),serde_json::to_vec_pretty(settings).map_err(err)?).map_err(err)?;
    std::fs::rename(dir.join("settings.tmp"),dir.join("settings.json")).map_err(err)
}
pub const MIN_SIZE: u32 = 64;
pub const MAX_SIZE: u32 = 480;
const MIN_CHAT_WIDTH: u32 = 300;
const CHAT_CONTROLS: u32 = 86;
const CHAT_REPLY: u32 = 145;
const CALENDAR_FORM: u32 = 310;
const RESIZE_STEP: i32 = 12;

pub fn load(app: &AppHandle) -> Settings {
    let mut settings: Settings = app.path().app_data_dir().ok()
        .and_then(|dir| std::fs::read(dir.join("settings.json")).ok())
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default();
    // The first build used a larger 220–400 pt scale. Keep the same relative preference.
    if settings.size_schema < 2 {
        settings.size = match settings.size {
            220 => 112,
            300 => 144,
            400 => 184,
            size => size,
        };
    }
    settings.size = settings.size.clamp(MIN_SIZE, MAX_SIZE);
    settings.size_schema = 2;
    settings
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ChatLayout {
    model_x: f64,
    model_y: f64,
    chat_x: f64,
    chat_y: f64,
    chat_width: f64,
    chat_height: f64,
}

fn model_origin(position: PhysicalPosition<i32>, scale: f64, desktop: &Desktop) -> (i32, i32) {
    match desktop.chat_layout {
        Some(layout) => (position.x + (layout.model_x * scale).round() as i32,
                         position.y + (layout.model_y * scale).round() as i32),
        None => (position.x, position.y),
    }
}

fn position_chat((x, y): (i32, i32), model: i32, width: i32, height: i32,
                 minimum_height: i32, bounds: Option<(i32, i32, i32, i32)>) -> (i32, i32, i32, i32) {
    let model_bottom = y + model;
    let preferred_x = x + (model - width) / 2;
    let Some((left, top, right, bottom)) = bounds else {
        return (preferred_x, model_bottom, width, height);
    };
    let width = width.min((right - left).max(1));
    let chat_x = (x + (model - width) / 2).clamp(left, right.max(left + width) - width);
    let above = (y - top).max(0);
    let below = (bottom - model_bottom).max(0);
    let use_below = below >= height || below >= above;
    let room = if use_below { below } else { above };
    let height = height.min(room.max(minimum_height));
    let chat_y = if use_below { model_bottom } else { y - height };
    (chat_x, chat_y, width, height)
}
#[tauri::command]
pub fn get_settings(state:tauri::State<DesktopState>)->Settings {state.lock().unwrap().settings.clone()}
#[tauri::command]
pub fn set_hit_mask(alpha:Vec<u8>,state:tauri::State<DesktopState>)->Result<(),String> {if alpha.len()!=4096{return Err("Máscara no válida".into())} state.lock().unwrap().alpha=alpha;Ok(())}
#[tauri::command]
pub fn open_panel(app:AppHandle)->Result<(),String> {let w=app.get_webview_window("panel").ok_or("Panel no disponible")?;w.show().map_err(err)?;w.unminimize().map_err(err)?;w.set_focus().map_err(err)}
#[tauri::command]
pub fn prepare_drag(state:tauri::State<DesktopState>) {state.lock().unwrap().dragging_until=Some(Instant::now()+Duration::from_secs(8));}
#[tauri::command]
pub fn finish_drag(app:AppHandle)->Result<(),String> {
    let w=app.get_webview_window("duck").ok_or("Duck no disponible")?;let p=w.outer_position().map_err(err)?;
    let settings={let state=app.state::<DesktopState>();let mut s=state.lock().unwrap();s.dragging_until=None;s.settings.position=Some(model_origin(p,w.scale_factor().map_err(err)?,&s));s.settings.clone()};persist(&app,&settings)
}
pub fn set_visible(app:&AppHandle,visible:bool)->Result<(),String> {
    if !visible {set_chat_open(app,false,false)?;}
    let settings={let state=app.state::<DesktopState>();let mut s=state.lock().unwrap();s.settings.visible=visible;s.temporary_until=None;s.leaving=if visible{None}else{Some(Instant::now()+Duration::from_millis(220))};s.settings.clone()};
    persist(app,&settings)?;
    refresh_mascot(app, false)?;
    if visible {app.emit_to("duck","duck-enter",()).map_err(err)?;}
    else {app.emit_to("duck","duck-leave",()).map_err(err)?;}
    app.emit("settings-changed",&settings).map_err(err)
}
#[tauri::command]
pub fn toggle_duck(app:AppHandle)->Result<(),String> {let visible=app.state::<DesktopState>().lock().unwrap().settings.visible;set_visible(&app,!visible)}
pub fn register_shortcut(app:&AppHandle,shortcut:&str)->Result<(),String> {
    app.global_shortcut().on_shortcut(shortcut,|app,_,event|{if event.state==ShortcutState::Pressed {if let Err(e)=toggle_duck(app.clone()){eprintln!("{e}");}}}).map_err(err)
}
#[tauri::command]
pub fn save_settings(app:AppHandle,mut settings:Settings)->Result<Settings,String> {
    if !(MIN_SIZE..=MAX_SIZE).contains(&settings.size) || !crate::appearance::ACCESSORIES.iter().any(|(id, _)| *id == settings.skin) {return Err("Configuración no válida".into())}
    let old=app.state::<DesktopState>().lock().unwrap().settings.clone();
    settings.size_schema = 2;
    settings.position = old.position;
    settings.visible = old.visible;
    if old.size != settings.size {
        // Grow or shrink around the mascot's centre, starting from where it really is now,
        // and keep it inside the current monitor so a big jump never throws it into a corner.
        let window = app.get_webview_window("duck");
        let scale = window.as_ref().and_then(|w| w.scale_factor().ok()).unwrap_or(1.0);
        let delta = ((old.size as f64 - settings.size as f64) * scale).round() as i32;
        let current = window.as_ref().and_then(|w| w.outer_position().ok()).map(|position| {
            let state = app.state::<DesktopState>();
            let desktop = state.lock().unwrap();
            model_origin(position, scale, &desktop)
        }).or(old.position);
        let area = window.as_ref().and_then(|w| w.current_monitor().ok().flatten()).map(|monitor| {
            let area = monitor.work_area();
            (area.position.x, area.position.y,
             area.position.x + area.size.width as i32, area.position.y + area.size.height as i32)
        });
        let model = (f64::from(settings.size) * scale).round() as i32;
        settings.position = current.map(|(x, y)| {
            let (mut new_x, mut new_y) = (x + delta / 2, y + delta / 2);
            if let Some((left, top, right, bottom)) = area {
                new_x = new_x.min(right - model).max(left);
                new_y = new_y.min(bottom - model).max(top);
            }
            (new_x, new_y)
        });
    }
    if old.shortcut!=settings.shortcut {register_shortcut(&app,&settings.shortcut)?;if let Err(e)=app.global_shortcut().unregister(old.shortcut.as_str()){let _=app.global_shortcut().unregister(settings.shortcut.as_str());return Err(err(e))}}
    if let Err(e)=persist(&app,&settings) {if old.shortcut!=settings.shortcut {let _=app.global_shortcut().unregister(settings.shortcut.as_str());let _=register_shortcut(&app,&old.shortcut);}return Err(e)}
    let open = {
        let state = app.state::<DesktopState>();
        let mut desktop = state.lock().unwrap();
        desktop.settings = settings.clone();
        desktop.chat_open
    };
    if old.size != settings.size {
        resize_duck(&app, open, settings.position)?;
    }
    app.emit("settings-changed", &settings).map_err(err)?;
    Ok(settings)
}
#[tauri::command]
pub fn adjust_duck_size(app: AppHandle, increase: bool) -> Result<Settings, String> {
    let mut settings = get_settings(app.state::<DesktopState>());
    let step = if increase { RESIZE_STEP } else { -RESIZE_STEP };
    settings.size = (settings.size as i32 + step).clamp(MIN_SIZE as i32, MAX_SIZE as i32) as u32;
    save_settings(app, settings)
}

#[cfg(target_os = "macos")]
extern "C" {
    fn duck_join_workspaces(native_window: *mut std::ffi::c_void);
    fn duck_set_front(visible: i32, recenter: i32);
    fn duck_set_chat_focus(open: i32, focus: i32);
    fn duck_resize_frame(dx: f64, dy: f64, width: f64, height: f64);
}

pub fn refresh_mascot(app: &AppHandle, recenter: bool) -> Result<(), String> {
    let app_handle = app.clone();
    app.run_on_main_thread(move || {
        let Some(window) = app_handle.get_webview_window("duck") else { return; };
        let state = app_handle.state::<DesktopState>();
        let (enabled, chat_open) = {
            let desktop = state.lock().unwrap();
            (desktop.settings.visible || desktop.temporary_until.is_some(), desktop.chat_open)
        };
        #[cfg(target_os = "macos")]
        unsafe { duck_set_front(i32::from(enabled), i32::from(recenter)); }
        #[cfg(not(target_os = "macos"))]
        if enabled { let _ = window.show(); }
        if enabled && recenter {
            if let (Ok(position), Ok(scale)) = (window.outer_position(), window.scale_factor()) {
                let settings = {
                    let mut desktop = state.lock().unwrap();
                    desktop.settings.position = Some(model_origin(position, scale, &desktop));
                    desktop.settings.clone()
                };
                if let Err(error) = persist(&app_handle, &settings) { eprintln!("{error}"); }
            }
        }
        if enabled {
            let _ = app_handle.emit_to("duck", "duck-active", true);
            if !chat_open && recenter { let _ = app_handle.emit_to("duck", "duck-enter", ()); }
        }
    }).map_err(err)
}

#[tauri::command]
pub fn bring_duck(app: AppHandle) -> Result<(), String> {
    set_visible(&app, true)?;
    refresh_mascot(&app, true)
}

pub fn initialize(app:&AppHandle)->Result<(),String> {
    let settings=load(app);
    app.manage(Mutex::new(Desktop {settings:settings.clone(),chat_open:false,chat_layout:None,chat_reply:false,calendar_form:false,panel_page:None,alpha:vec![0;4096],temporary_until:None,leaving:None,position_dirty:None,dragging_until:None}));
    let w=app.get_webview_window("duck").ok_or("Duck no disponible")?;
    #[cfg(target_os = "macos")]
    {
        // Tauri calls setup on the AppKit main thread, before the overlay is shown.
        crate::overlay::attach(&w)?;
        let native_window = w.ns_window().map_err(err)?;
        unsafe { duck_join_workspaces(native_window) };
    }
    w.set_size(LogicalSize::new(settings.size,settings.size)).map_err(err)?;
    let monitors=w.available_monitors().map_err(err)?;
    let position=settings.position.filter(|(x,y)|monitors.iter().any(|m|{let p=m.position();let z=m.size();*x>=p.x && *y>=p.y && *x+100<p.x+z.width as i32 && *y+100<p.y+z.height as i32}));
    let position = match position {
        Some(position) => Some(position),
        None => w.primary_monitor().map_err(err)?.map(|monitor| {
            let origin = monitor.position();
            let bounds = monitor.size();
            let size = (f64::from(settings.size) * monitor.scale_factor()) as i32;
            (origin.x + bounds.width as i32 - size - 40, origin.y + bounds.height as i32 - size - 80)
        }),
    };
    if let Some((x, y)) = position {
        w.set_position(PhysicalPosition::new(x, y)).map_err(err)?;
    }
    app.state::<DesktopState>().lock().unwrap().settings.position = position;
    let moved_app=app.clone();
    w.on_window_event(move |event| {
        if let tauri::WindowEvent::Moved(pos) = event {
            let scale = moved_app.get_webview_window("duck").and_then(|w| w.scale_factor().ok()).unwrap_or(1.0);
            let state = moved_app.state::<DesktopState>();
            let mut desktop = state.lock().unwrap();
            if desktop.dragging_until.is_some_and(|until| Instant::now() < until) {
                desktop.settings.position = Some(model_origin(*pos, scale, &desktop));
                desktop.position_dirty = Some(Instant::now());
            }
        }
    });
    w.set_ignore_cursor_events(true).map_err(err)?;
    if settings.visible {refresh_mascot(app, false)?;}
    if let Err(e)=register_shortcut(app,&settings.shortcut) {eprintln!("Atajo no disponible: {e}");let _=app.emit("desktop-error",format!("No se pudo registrar el atajo: {e}"));}
    let app=app.clone();
    std::thread::spawn(move|| {
        let mut last_ignore=true;
        loop {
            std::thread::sleep(Duration::from_millis(32));
            let Some(w)=app.get_webview_window("duck") else {break};
            let (visible,alpha,bubble,drag,hide,leave,expired,save,layout,model_size)={let state=app.state::<DesktopState>();let mut s=state.lock().unwrap();let now=Instant::now();
                let desired=s.settings.visible;
                let mut temporary=s.temporary_until;let mut leaving=s.leaving;
                let (expired,leave,hide)=crate::lifecycle::advance(desired,&mut temporary,&mut leaving,now);
                s.temporary_until=temporary;s.leaving=leaving;
                let save=if s.position_dirty.is_some_and(|t|now.duration_since(t)>Duration::from_millis(600)){s.position_dirty=None;Some(s.settings.clone())}else{None};
                (s.settings.visible||s.temporary_until.is_some()||s.leaving.is_some(),s.alpha.clone(),s.temporary_until.is_some(),s.dragging_until.is_some_and(|t|now<t),hide,leave,expired,save,s.chat_layout,s.settings.size)
            };
            if let Some(settings)=save {if let Err(e)=persist(&app,&settings){eprintln!("{e}");}}
            if expired {let _=refresh_mascot(&app, false);let _=app.emit_to("duck","duck-clear-notification",());}
            if leave {let _=app.emit_to("duck","duck-leave",());}
            if hide {let state=app.state::<DesktopState>();let s=state.lock().unwrap();if !s.settings.visible && s.temporary_until.is_none(){let _=w.hide();let _=app.emit_to("duck","duck-active",false);}}
            if !visible || hide {continue}
            if let (Ok(cursor),Ok(pos),Ok(scale))=(w.cursor_position(),w.inner_position(),w.scale_factor()) {
                let x=(cursor.x-f64::from(pos.x))/scale;
                let y=(cursor.y-f64::from(pos.y))/scale;
                let model = f64::from(model_size);
                let (model_x, model_y) = layout.map_or((0.0, 0.0), |layout| (layout.model_x, layout.model_y));
                let mx = (x - model_x) / model;
                let my = (y - model_y) / model;
                let in_model = (0.0..1.0).contains(&mx) && (0.0..1.0).contains(&my);
                let on_model = in_model && alpha[(my*64.0) as usize*64+(mx*64.0) as usize]>0;
                let on_size_controls = in_model && (mx - 0.5).abs() * model <= 42.0 && my * model >= model - 25.0;
                let on_chat = layout.is_some_and(|chat| {
                    (chat.chat_x + 5.0..chat.chat_x + chat.chat_width - 5.0).contains(&x) &&
                    (chat.chat_y..chat.chat_y + chat.chat_height).contains(&y)
                });
                // While an alarm rings its buttons sit lower in the bubble, so the clickable area is taller.
                let alarm = crate::alarm::is_ringing();
                let on_bubble = (layout.is_none() || alarm) && bubble && (0.0..if alarm { 0.82 } else { 0.34 }).contains(&my) && (0.03..0.97).contains(&mx);
                let hit = on_model || on_chat || on_size_controls || on_bubble;
                let ignore=!hit&&!drag;
                if ignore!=last_ignore {let _=w.set_ignore_cursor_events(ignore);last_ignore=ignore;}
            }
        }
    });
    Ok(())
}

#[tauri::command]
pub(crate) fn get_chat_layout(state: tauri::State<DesktopState>) -> Option<ChatLayout> {
    state.lock().unwrap().chat_layout
}

fn resize_duck(app: &AppHandle, open: bool, anchor_override: Option<(i32, i32)>) -> Result<(), String> {
    let window = app.get_webview_window("duck").ok_or("Duck no disponible")?;
    let position = window.outer_position().map_err(err)?;
    let scale = window.scale_factor().map_err(err)?;
    let (size, extra, anchor) = {
        let state = app.state::<DesktopState>();
        let desktop = state.lock().unwrap();
        let extra = if desktop.calendar_form { CALENDAR_FORM } else if desktop.chat_reply { CHAT_REPLY } else { 0 };
        (desktop.settings.size, extra, anchor_override.unwrap_or_else(|| model_origin(position, scale, &desktop)))
    };
    let pixels = |points| (f64::from(points) * scale).round() as i32;
    let points = |pixels| f64::from(pixels) / scale;
    let model = pixels(size);
    let (model_x, model_y) = anchor;
    let (x, y, width, height, layout) = if open {
        let area = window.current_monitor().ok().flatten().map(|monitor| {
            let area = monitor.work_area();
            (area.position.x, area.position.y,
             area.position.x + area.size.width as i32, area.position.y + area.size.height as i32)
        });
        let (chat_x, chat_y, chat_width, chat_height) = position_chat(
            anchor, model, pixels(size.max(MIN_CHAT_WIDTH)), pixels(CHAT_CONTROLS + extra), pixels(CHAT_CONTROLS), area,
        );
        let x = model_x.min(chat_x);
        let y = model_y.min(chat_y);
        let width = (model_x + model).max(chat_x + chat_width) - x;
        let height = (model_y + model).max(chat_y + chat_height) - y;
        let layout = ChatLayout {
            model_x: points(model_x - x), model_y: points(model_y - y),
            chat_x: points(chat_x - x), chat_y: points(chat_y - y),
            chat_width: points(chat_width), chat_height: points(chat_height),
        };
        (x, y, width, height, Some(layout))
    } else {
        (model_x, model_y, model, model, None)
    };
    #[cfg(target_os = "macos")]
    {
        let dx = points(x - position.x);
        let dy = points(y - position.y);
        let width = points(width);
        let height = points(height);
        app.run_on_main_thread(move || unsafe { duck_resize_frame(dx, dy, width, height) }).map_err(err)?;
    }
    #[cfg(not(target_os = "macos"))]
    {
        window.set_size(PhysicalSize::new(width as u32, height as u32)).map_err(err)?;
        window.set_position(PhysicalPosition::new(x, y)).map_err(err)?;
    }
    {
        let state = app.state::<DesktopState>();
        let mut desktop = state.lock().unwrap();
        desktop.chat_open = open;
        desktop.chat_layout = layout;
        if !open {
            desktop.chat_reply = false;
            desktop.calendar_form = false;
        }
    }
    app.emit_to("duck", "chat-layout", layout).map_err(err)
}

pub fn set_chat_open(app: &AppHandle, open: bool, focus: bool) -> Result<(), String> {
    if !open {
        crate::dictation::cancel_window(app, "duck");
    }
    resize_duck(app, open, None)?;
    #[cfg(target_os = "macos")]
    app.run_on_main_thread(move || unsafe { duck_set_chat_focus(i32::from(open), i32::from(focus)) }).map_err(err)?;
    #[cfg(not(target_os = "macos"))]
    {
        let window = app.get_webview_window("duck").ok_or("Duck no disponible")?;
        window.set_focusable(open).map_err(err)?;
        if focus {
            window.show().map_err(err)?;
            window.set_focus().map_err(err)?;
        }
    }
    Ok(())
}
#[tauri::command]
pub fn open_chat(app: AppHandle) -> Result<(), String> {
    let visible = app.state::<DesktopState>().lock().unwrap().settings.visible;
    if !visible { set_visible(&app, true)?; }
    set_chat_open(&app, true, true)
}
#[tauri::command]
pub fn close_chat(app:AppHandle)->Result<(),String>{set_chat_open(&app,false,false)}
#[tauri::command]
pub fn toggle_chat(app: AppHandle) -> Result<(), String> {
    let is_open = app.state::<DesktopState>().lock().unwrap().chat_open;
    if is_open { close_chat(app) } else { open_chat(app) }
}

#[tauri::command]
pub fn navigate_panel(app:AppHandle,page:String)->Result<(),String>{
 if !["home","notes","reminders","agenda","reporting","agents","settings","guide"].contains(&page.as_str()){return Err("Sección no válida".into())}
 app.state::<DesktopState>().lock().unwrap().panel_page = Some(page.clone());
 open_panel(app.clone())?;
 app.emit_to("panel","navigate",page).map_err(err)
}

#[tauri::command]
pub fn set_chat_reply(app:AppHandle,expanded:bool,calendar_form:Option<bool>)->Result<(),String>{
 let open={let state=app.state::<DesktopState>();let mut s=state.lock().unwrap();s.chat_reply=expanded;s.calendar_form=calendar_form.unwrap_or(false);s.chat_open};
 if open {resize_duck(&app, true, None)?;} Ok(())
}


#[tauri::command]
pub fn get_panel_page(state: tauri::State<DesktopState>) -> Option<String> {
    state.lock().unwrap().panel_page.take()
}
