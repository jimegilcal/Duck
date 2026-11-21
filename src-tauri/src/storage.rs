use chrono::{Utc, SecondsFormat};
use rusqlite::{Connection, params};
use serde::{Serialize,Deserialize};
use std::{path::Path,sync::Mutex};
use tauri::{Manager,Emitter};
use uuid::Uuid;
pub type Db=Mutex<Connection>;
pub type Result<T>=std::result::Result<T,String>;
pub fn err(e:impl std::fmt::Display)->String{e.to_string()}
pub fn now()->String{Utc::now().to_rfc3339_opts(SecondsFormat::Millis,true)}
#[derive(Clone,Serialize,Deserialize,Debug)]
#[serde(rename_all="camelCase")]
pub struct Note {pub id:String,pub body:String,pub created_at:String,pub updated_at:String}
#[derive(Clone,Serialize,Deserialize,Debug)]
#[serde(rename_all="camelCase")]
pub struct Reminder {pub id:String,pub title:String,pub due_at:String,pub completed:bool,pub notified_at:Option<String>,#[serde(default)] pub alarm:bool}
#[derive(Clone,Serialize,Deserialize,Debug)]
#[serde(rename_all="camelCase")]
pub struct DuckEvent {pub id:String,pub provider:String,pub session:Option<String>,pub project:Option<String>,pub status:String,pub event_type:String,pub title:String,pub description:String,pub timestamp:String,pub metadata:Option<serde_json::Value>}
#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct Snapshot {pub notes:Vec<Note>,pub reminders:Vec<Reminder>,pub events:Vec<DuckEvent>}
pub fn open(path:&Path)->Result<Connection>{
 let c=Connection::open(path).map_err(err)?;
 c.busy_timeout(std::time::Duration::from_secs(5)).map_err(err)?;
 c.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;
 CREATE TABLE IF NOT EXISTS notes(id TEXT PRIMARY KEY,body TEXT NOT NULL,created_at TEXT NOT NULL,updated_at TEXT NOT NULL);
 CREATE TABLE IF NOT EXISTS reminders(id TEXT PRIMARY KEY,title TEXT NOT NULL,due_at TEXT NOT NULL,completed INTEGER NOT NULL DEFAULT 0,notified_at TEXT);
 CREATE INDEX IF NOT EXISTS reminders_due ON reminders(completed,notified_at,due_at);
 CREATE TABLE IF NOT EXISTS events(id TEXT PRIMARY KEY,payload TEXT NOT NULL,created_at TEXT NOT NULL);
 ").map_err(err)?;add_alarm_column(&c)?;crate::chat::migrate(&c)?;Ok(c)
}
// Older databases have no alarm flag; add it once and keep every existing reminder as a normal one.
fn add_alarm_column(c:&Connection)->Result<()> {
 let has=c.prepare("PRAGMA table_info(reminders)").map_err(err)?.query_map([],|r|r.get::<_,String>(1)).map_err(err)?.filter_map(|n|n.ok()).any(|n|n=="alarm");
 if !has {c.execute("ALTER TABLE reminders ADD COLUMN alarm INTEGER NOT NULL DEFAULT 0",[]).map_err(err)?;}
 Ok(())
}
fn text(value:&str,max:usize)->Result<String>{let s=value.trim();if s.is_empty(){return Err("El texto no puede estar vacío".into())}if s.chars().count()>max{return Err(format!("Máximo {max} caracteres"))}Ok(s.into())}
pub fn add_note(c:&Connection,body:&str)->Result<Note>{let body=text(body,20000)?;let n=Note{id:Uuid::new_v4().to_string(),body,created_at:now(),updated_at:now()};c.execute("INSERT INTO notes VALUES (?1,?2,?3,?4)",params![n.id,n.body,n.created_at,n.updated_at]).map_err(err)?;Ok(n)}
pub fn edit_note(c:&Connection,id:&str,body:&str)->Result<()> {let body=text(body,20000)?;if c.execute("UPDATE notes SET body=?1,updated_at=?2 WHERE id=?3",params![body,now(),id]).map_err(err)?==0{return Err("La nota ya no existe".into())}Ok(())}
pub fn add_reminder(c:&Connection,title:&str,due_at:&str)->Result<Reminder>{add_reminder_with(c,title,due_at,false)}
pub fn add_reminder_with(c:&Connection,title:&str,due_at:&str,alarm:bool)->Result<Reminder>{
 let title=text(title,240)?;let due=chrono::DateTime::parse_from_rfc3339(due_at).map_err(|_|"Fecha no válida")?.with_timezone(&Utc);
 if due<=Utc::now(){return Err("Elige una fecha y hora futuras".into())}
 let r=Reminder{id:Uuid::new_v4().to_string(),title,due_at:due.to_rfc3339_opts(SecondsFormat::Millis,true),completed:false,notified_at:None,alarm};
 c.execute("INSERT INTO reminders(id,title,due_at,alarm) VALUES (?1,?2,?3,?4)",params![r.id,r.title,r.due_at,r.alarm]).map_err(err)?;Ok(r)
}
// Snooze: the same reminder fires again after `minutes` (1 to 120). Only a pending reminder can be snoozed.
pub fn snooze_reminder(c:&Connection,id:&str,minutes:i64)->Result<String> {
 if !(1..=120).contains(&minutes){return Err("Elige entre 1 y 120 minutos".into())}
 let due=(Utc::now()+chrono::Duration::minutes(minutes)).to_rfc3339_opts(SecondsFormat::Millis,true);
 if c.execute("UPDATE reminders SET due_at=?1,notified_at=NULL WHERE id=?2 AND completed=0",params![due,id]).map_err(err)?==0{return Err("El recordatorio ya no está pendiente".into())}
 Ok(due)
}
pub fn snapshot(c:&Connection)->Result<Snapshot>{
 let notes=c.prepare("SELECT id,body,created_at,updated_at FROM notes ORDER BY created_at DESC").map_err(err)?.query_map([],|r|Ok(Note{id:r.get(0)?,body:r.get(1)?,created_at:r.get(2)?,updated_at:r.get(3)?})).map_err(err)?.collect::<std::result::Result<Vec<_>,_>>().map_err(err)?;
 let reminders=c.prepare("SELECT id,title,due_at,completed,notified_at,alarm FROM reminders ORDER BY completed,due_at").map_err(err)?.query_map([],|r|Ok(Reminder{id:r.get(0)?,title:r.get(1)?,due_at:r.get(2)?,completed:r.get(3)?,notified_at:r.get(4)?,alarm:r.get(5)?})).map_err(err)?.collect::<std::result::Result<Vec<_>,_>>().map_err(err)?;
 let payloads=c.prepare("SELECT payload FROM events ORDER BY created_at DESC LIMIT 100").map_err(err)?.query_map([],|r|r.get::<_,String>(0)).map_err(err)?.collect::<std::result::Result<Vec<_>,_>>().map_err(err)?;
 let events=payloads.into_iter().map(|p|serde_json::from_str(&p).map_err(err)).collect::<Result<Vec<_>>>()?;Ok(Snapshot{notes,reminders,events})
}
pub fn store_event(c:&Connection,event:&DuckEvent)->Result<()> {c.execute("INSERT OR IGNORE INTO events VALUES (?1,?2,?3)",params![event.id,serde_json::to_string(event).map_err(err)?,event.timestamp]).map_err(err)?;Ok(())}
// Claim and persist each due event in one transaction: repeated scheduler ticks cannot duplicate it.
pub fn claim_due(c:&mut Connection,at:&str)->Result<Vec<DuckEvent>> {
 let tx=c.transaction().map_err(err)?;
 let due={let mut stmt=tx.prepare("SELECT id,title,due_at,alarm FROM reminders WHERE completed=0 AND notified_at IS NULL AND due_at<=?1 ORDER BY due_at").map_err(err)?;
 let rows=stmt.query_map([at],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,bool>(3)?))).map_err(err)?.collect::<std::result::Result<Vec<_>,_>>().map_err(err)?;rows};
 let mut events=vec![];
 for (id,title,due,alarm) in due {let e=DuckEvent{id:format!("reminder:{id}"),provider:"local".into(),session:None,project:None,status:"finished".into(),event_type:"reminder.due".into(),title,description:if alarm{"¡Alarma! Ha llegado la hora."}else{"Ha llegado la hora de tu recordatorio."}.into(),timestamp:at.into(),metadata:Some(serde_json::json!({"reminderId":id,"dueAt":due,"alarm":alarm}))};
 store_event(&tx,&e)?;tx.execute("UPDATE reminders SET notified_at=?1 WHERE id=?2",params![at,id]).map_err(err)?;events.push(e);}
 tx.commit().map_err(err)?;Ok(events)
}
fn changed(app:&tauri::AppHandle){let _=app.emit("data-changed",());}
#[tauri::command]
pub fn get_snapshot(db:tauri::State<Db>)->Result<Snapshot>{snapshot(&*db.lock().map_err(err)?)}
#[tauri::command]
pub fn create_note(app:tauri::AppHandle,body:String,db:tauri::State<Db>)->Result<Note>{let n=add_note(&*db.lock().map_err(err)?,&body)?;changed(&app);Ok(n)}
#[tauri::command]
pub fn update_note(app:tauri::AppHandle,id:String,body:String,db:tauri::State<Db>)->Result<()>{edit_note(&*db.lock().map_err(err)?,&id,&body)?;changed(&app);Ok(())}
#[tauri::command]
pub fn delete_note(app:tauri::AppHandle,id:String,db:tauri::State<Db>)->Result<()>{db.lock().map_err(err)?.execute("DELETE FROM notes WHERE id=?1",[id]).map_err(err)?;changed(&app);Ok(())}
#[tauri::command]
pub fn create_reminder(app:tauri::AppHandle,title:String,due_at:String,alarm:Option<bool>,db:tauri::State<Db>)->Result<Reminder>{let r=add_reminder_with(&*db.lock().map_err(err)?,&title,&due_at,alarm.unwrap_or(false))?;changed(&app);Ok(r)}
#[tauri::command]
pub fn complete_reminder(app:tauri::AppHandle,id:String,completed:bool,db:tauri::State<Db>)->Result<()>{db.lock().map_err(err)?.execute("UPDATE reminders SET completed=?1 WHERE id=?2",params![completed,id]).map_err(err)?;if completed{crate::alarm::stop(&app,&id);}changed(&app);Ok(())}
#[tauri::command]
pub fn delete_reminder(app:tauri::AppHandle,id:String,db:tauri::State<Db>)->Result<()>{db.lock().map_err(err)?.execute("DELETE FROM reminders WHERE id=?1",[&id]).map_err(err)?;crate::alarm::stop(&app,&id);changed(&app);Ok(())}
pub fn initialize(app:&tauri::AppHandle)->Result<()> {let dir=app.path().app_data_dir().map_err(err)?;std::fs::create_dir_all(&dir).map_err(err)?;app.manage(Mutex::new(open(&dir.join("duck.sqlite3"))?));Ok(())}

#[cfg(test)] mod tests {
 use super::*;
 #[test] fn persists_reopens_edits_and_deletes(){let path=std::env::temp_dir().join(format!("duck-test-{}.sqlite3",Uuid::new_v4()));
 {let c=open(&path).unwrap();let n=add_note(&c,"Prueba persistente ' ; DROP TABLE notes;").unwrap();edit_note(&c,&n.id,"Nota editada").unwrap();}
 {let c=open(&path).unwrap();let s=snapshot(&c).unwrap();assert_eq!(s.notes[0].body,"Nota editada");c.execute("DELETE FROM notes WHERE id=?1",[&s.notes[0].id]).unwrap();assert!(snapshot(&c).unwrap().notes.is_empty());}
 std::fs::remove_file(path).unwrap();}
 #[test] fn validates_input(){let c=open(Path::new(":memory:")).unwrap();assert!(add_note(&c,"  ").is_err());assert!(add_note(&c,&"x".repeat(20001)).is_err());assert!(add_reminder(&c,"x","not a date").is_err());assert!(add_reminder(&c,"x","2000-01-01T00:00:00Z").is_err());}
 #[test] fn due_is_atomic_and_not_repeated(){let mut c=open(Path::new(":memory:")).unwrap();let due=(Utc::now()+chrono::Duration::minutes(1)).to_rfc3339();let r=add_reminder(&c,"Aviso",&due).unwrap();assert!(claim_due(&mut c,&now()).unwrap().is_empty());let future=(Utc::now()+chrono::Duration::minutes(2)).to_rfc3339();assert_eq!(claim_due(&mut c,&future).unwrap().len(),1);assert!(claim_due(&mut c,&future).unwrap().is_empty());let s=snapshot(&c).unwrap();assert_eq!(s.events.len(),1);assert!(s.reminders.iter().find(|x|x.id==r.id).unwrap().notified_at.is_some());}
 #[test] fn alarm_flag_is_saved_and_marks_the_due_event(){
  let mut c=open(Path::new(":memory:")).unwrap();let due=(Utc::now()+chrono::Duration::minutes(1)).to_rfc3339();
  let normal=add_reminder(&c,"Normal",&due).unwrap();let alarm=add_reminder_with(&c,"Despertar",&due,true).unwrap();
  assert!(!normal.alarm&&alarm.alarm);
  let s=snapshot(&c).unwrap();assert_eq!(s.reminders.iter().filter(|r|r.alarm).count(),1);
  let events=claim_due(&mut c,&(Utc::now()+chrono::Duration::minutes(2)).to_rfc3339_opts(SecondsFormat::Millis,true)).unwrap();
  assert_eq!(events.len(),2);
  let flags:Vec<(String,bool)>=events.iter().map(|e|(e.title.clone(),e.metadata.as_ref().unwrap()["alarm"].as_bool().unwrap())).collect();
  assert!(flags.contains(&("Despertar".to_string(),true))&&flags.contains(&("Normal".to_string(),false)));
 }
 #[test] fn old_database_without_alarm_column_is_upgraded_and_keeps_reminders(){
  let path=std::env::temp_dir().join(format!("duck-test-{}.sqlite3",Uuid::new_v4()));
  {let c=Connection::open(&path).unwrap();c.execute_batch("CREATE TABLE reminders(id TEXT PRIMARY KEY,title TEXT NOT NULL,due_at TEXT NOT NULL,completed INTEGER NOT NULL DEFAULT 0,notified_at TEXT);INSERT INTO reminders(id,title,due_at) VALUES ('a','Antiguo','2999-01-01T00:00:00.000Z');").unwrap();}
  {let c=open(&path).unwrap();let s=snapshot(&c).unwrap();assert_eq!(s.reminders.len(),1);assert_eq!(s.reminders[0].title,"Antiguo");assert!(!s.reminders[0].alarm);}
  {let c=open(&path).unwrap();assert_eq!(snapshot(&c).unwrap().reminders.len(),1);} // reopening twice must not fail
  let _=std::fs::remove_file(&path);
 }
 #[test] fn snooze_fires_again_and_rejects_bad_requests(){
  let mut c=open(Path::new(":memory:")).unwrap();let due=(Utc::now()+chrono::Duration::minutes(1)).to_rfc3339();
  let r=add_reminder_with(&c,"Alarma",&due,true).unwrap();
  let later=(Utc::now()+chrono::Duration::minutes(2)).to_rfc3339_opts(SecondsFormat::Millis,true);
  assert_eq!(claim_due(&mut c,&later).unwrap().len(),1);
  assert!(claim_due(&mut c,&later).unwrap().is_empty()); // already fired
  let new_due=snooze_reminder(&c,&r.id,5).unwrap();
  assert!(new_due>later); // it moved to the future
  assert!(claim_due(&mut c,&later).unwrap().is_empty()); // not yet
  let after=(Utc::now()+chrono::Duration::minutes(6)).to_rfc3339_opts(SecondsFormat::Millis,true);
  assert_eq!(claim_due(&mut c,&after).unwrap().len(),1); // fires again after the snooze
  assert!(snooze_reminder(&c,&r.id,0).is_err()&&snooze_reminder(&c,&r.id,121).is_err());
  assert!(snooze_reminder(&c,"no-existe",5).is_err());
  c.execute("UPDATE reminders SET completed=1 WHERE id=?1",[&r.id]).unwrap();
  assert!(snooze_reminder(&c,&r.id,5).is_err()); // a finished reminder is not revived
 }
 #[test] fn completed_reminders_are_not_notified(){let mut c=open(Path::new(":memory:")).unwrap();let due=(Utc::now()+chrono::Duration::minutes(1)).to_rfc3339();let r=add_reminder(&c,"Hecho",&due).unwrap();c.execute("UPDATE reminders SET completed=1 WHERE id=?1",[r.id]).unwrap();let future=(Utc::now()+chrono::Duration::minutes(2)).to_rfc3339();assert!(claim_due(&mut c,&future).unwrap().is_empty());}
}


#[tauri::command]
pub async fn open_data_folder(app: tauri::AppHandle) -> Result<()> {
    let directory = app.path().app_data_dir().map_err(err)?;
    let status = std::process::Command::new("/usr/bin/open")
        .arg(directory)
        .status()
        .map_err(err)?;
    if status.success() { Ok(()) } else { Err("No se pudo abrir la carpeta de Duck en Finder.".into()) }
}
