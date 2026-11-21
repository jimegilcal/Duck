#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod desktop;
mod appearance;
mod overlay;
mod storage;
mod notifications;
mod alarm;
mod lifecycle;
mod chat;
mod google;
mod calendar;
mod agents;
mod dictation;
#[cfg(target_os = "macos")]
mod codex_activity;
mod usage;
use tauri::{Manager,menu::{Menu,MenuItem,Submenu},tray::TrayIconBuilder};
fn main() {
 let builder = tauri::Builder::default();
 #[cfg(target_os = "macos")]
 let builder = builder.plugin(tauri_nspanel::init());
 builder
 .plugin(tauri_plugin_single_instance::init(|app,_,_|{let _=desktop::open_panel(app.clone());}))
 .plugin(tauri_plugin_notification::init())
 .plugin(tauri_plugin_global_shortcut::Builder::new().build())
 .invoke_handler(tauri::generate_handler![desktop::get_settings,desktop::save_settings,desktop::adjust_duck_size,desktop::bring_duck,appearance::show_duck_sizes,desktop::toggle_duck,desktop::open_panel,desktop::set_hit_mask,desktop::prepare_drag,desktop::finish_drag,storage::get_snapshot,storage::create_note,storage::update_note,storage::delete_note,storage::create_reminder,storage::complete_reminder,storage::delete_reminder,notifications::test_notification,alarm::ringing_alarms,alarm::stop_alarm,alarm::snooze_alarm,desktop::open_chat,desktop::close_chat,desktop::toggle_chat,appearance::show_duck_menu,storage::open_data_folder,desktop::get_chat_layout,desktop::get_panel_page,desktop::set_chat_reply,desktop::navigate_panel,chat::get_messages,chat::get_proposals,chat::submit_chat,chat::dismiss_proposal,calendar::current_calendar,calendar::list_calendars,calendar::select_calendar,calendar::save_calendar_event,google::open_google_tasks,agents::running_agents,agents::agent_updates,usage::usage_report,dictation::start_dictation,dictation::end_dictation])
 .setup(|app| {
  storage::initialize(app.handle())?;
  desktop::initialize(app.handle())?;
  notifications::start(app.handle());
  let chat=MenuItem::with_id(app,"chat","Hablar con Duck",true,None::<&str>)?;
  let panel=MenuItem::with_id(app,"panel","Abrir panel",true,None::<&str>)?;
  let toggle=MenuItem::with_id(app,"toggle","Mostrar / ocultar Duck",true,None::<&str>)?;
  let quit=MenuItem::with_id(app,"quit","Salir de Duck",true,None::<&str>)?;
  let show_hide_chat=MenuItem::with_id(app,"toggle-chat","Mostrar u ocultar chat",true,None::<&str>)?;
  let appearance=MenuItem::with_id(app,"appearance-settings","Tamaño y accesorios…",true,None::<&str>)?;
  let bring=MenuItem::with_id(app,"bring-duck","Traer a Duck a esta pantalla",true,None::<&str>)?;
  let guide=MenuItem::with_id(app,"guide","Guía de Duck…",true,None::<&str>)?;
  let items: &[&dyn tauri::menu::IsMenuItem<_>] = &[&chat,&show_hide_chat,&panel,&bring,&appearance,&guide,&toggle,&quit];
  let menu=Menu::with_items(app,items)?;
  let app_menu=Menu::default(app.handle())?;
  let controls=Submenu::with_items(app,"Compañero",true,items)?;
  app_menu.append(&controls)?;app.set_menu(app_menu)?;
  app.on_menu_event(|app,event| {
   let result=match event.id.as_ref(){"chat"=>desktop::open_chat(app.clone()),"panel"=>desktop::open_panel(app.clone()),"toggle"=>desktop::toggle_duck(app.clone()),"quit"=>{app.exit(0);Ok(())},id=>appearance::handle_menu_event(app,id)};if let Err(e)=result{eprintln!("{e}");}
  });
  TrayIconBuilder::new().icon(app.default_window_icon().unwrap().clone()).tooltip("Duck · Tu compañero de escritorio").menu(&menu).build(app)?;
  let panel = app.get_webview_window("panel").unwrap();
  let p = panel.clone();
  panel.on_window_event(move |event| {
   if let tauri::WindowEvent::CloseRequested { api, .. } = event {
    api.prevent_close();
    dictation::cancel_window(p.app_handle(), "panel");
    let _ = p.hide();
   }
  });
  Ok(())
 }).build(tauri::generate_context!()).expect("No se pudo iniciar Duck")
 .run(|app,event| match event {
  tauri::RunEvent::Reopen{..} => { let _ = desktop::open_panel(app.clone()); }
  #[cfg(target_os = "macos")]
  tauri::RunEvent::Exit => overlay::detach(app),
  _ => {}
 });
}
