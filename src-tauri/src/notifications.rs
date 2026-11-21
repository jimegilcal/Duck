use std::time::{Duration,Instant};
use tauri::{AppHandle,Emitter,Manager};
use tauri_plugin_notification::NotificationExt;
use crate::{desktop::DesktopState,storage::{self,Db,DuckEvent,Result}};

// Single entry point for real local reminders and explicitly identified test events.
pub fn deliver(app:&AppHandle,event:&DuckEvent)->Result<()> {
 let settings=app.state::<DesktopState>().lock().map_err(storage::err)?.settings.clone();
 if settings.notifications {
   if let Err(e)=app.notification().builder().title(&event.title).body(&event.description).show(){
    let _=app.emit("notification-error",format!("macOS no ha podido enviar el aviso: {e}"));
   }
 }
 if settings.companion_alerts {
  {let state=app.state::<DesktopState>();let mut s=state.lock().map_err(storage::err)?;s.temporary_until=Some(Instant::now()+Duration::from_secs(8));s.leaving=None;}
  crate::desktop::refresh_mascot(app, false)?;
  app.emit_to("duck","duck-notification",event).map_err(storage::err)?;
 }
 // An alarm rings regardless of the two switches above: asking for an alarm is an explicit request.
 // It goes last so its longer on-screen time replaces the 8 seconds of a normal notice.
 let meta=event.metadata.as_ref();
 if meta.and_then(|m|m.get("alarm")).and_then(|v|v.as_bool())==Some(true) {
  if let Some(id)=meta.and_then(|m|m.get("reminderId")).and_then(|v|v.as_str()) {crate::alarm::start_ringing(app,id,&event.title);}
 }
 app.emit("data-changed",()).map_err(storage::err)?;Ok(())
}
#[tauri::command]
pub fn test_notification(app:AppHandle)->Result<()> {
 let event=DuckEvent{id:uuid::Uuid::new_v4().to_string(),provider:"test".into(),session:None,project:None,status:"unknown".into(),event_type:"notification.test".into(),title:"Prueba · ¡Duck está aquí!".into(),description:"Este es un aviso de prueba. No procede de ningún agente.".into(),timestamp:storage::now(),metadata:None};
 storage::store_event(&*app.state::<Db>().lock().map_err(storage::err)?,&event)?;deliver(&app,&event)
}
pub fn start(app:&AppHandle){
 let app=app.clone();
 std::thread::spawn(move||loop{
  std::thread::sleep(Duration::from_secs(2));
  let due={let db=app.state::<Db>();let mut c=match db.lock(){Ok(c)=>c,Err(e)=>{eprintln!("{e}");continue}};storage::claim_due(&mut c,&storage::now())};
  match due {Ok(events)=>for event in events{if let Err(e)=deliver(&app,&event){eprintln!("{e}");let _=app.emit("notification-error",e);}},Err(e)=>{eprintln!("{e}");let _=app.emit("notification-error",e);}}
 });
}
