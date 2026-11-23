use serde::{Serialize,Deserialize};
use rusqlite::{Connection,params};
use tauri::{Emitter,AppHandle,Manager};
use crate::storage::{self,Db,Result};
#[derive(Serialize,Deserialize,Clone,Debug)]
#[serde(rename_all="camelCase")]
pub struct Message {pub id:String,pub role:String,pub body:String,pub action:Option<String>,pub created_at:String,pub follow_up:Option<serde_json::Value>}
#[derive(Serialize,Deserialize,Clone,Debug)]
#[serde(rename_all="camelCase")]
pub struct Proposal {pub id:String,pub kind:String,pub text:String,pub status:String,pub created_at:String,pub event_details:Option<EventDetails>}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EventDetails {
    pub title: String,
    pub start: String,
    pub end: String,
}

#[derive(Deserialize)]
#[serde(tag="type")]
pub enum Action {
 #[serde(rename="note.create")] Note {body:String},
 #[serde(rename="reminder.create",rename_all="camelCase")] Reminder {title:String,due_at:String,#[serde(default)] alarm:bool},
 #[serde(rename="calendar.event.propose")] Event {text:String},
 #[serde(rename="calendar.event.create")] CreateEvent {event:EventDetails},
 #[serde(rename="calendar.task.propose")] Task {text:String},
 #[serde(rename="agent.status.query")] Agents,
 #[serde(rename="report.query")] Report,
 #[serde(rename="undo.last")] Undo {target:Option<String>},
 #[serde(rename="summary.today")] Summary,
 #[serde(rename="clarify",rename_all="camelCase")] Clarify {message:String,follow_up:Option<serde_json::Value>},
}
pub fn migrate(c:&Connection)->Result<()> {c.execute_batch("CREATE TABLE IF NOT EXISTS messages(id TEXT PRIMARY KEY,role TEXT NOT NULL,body TEXT NOT NULL,action TEXT,created_at TEXT NOT NULL);
 CREATE TABLE IF NOT EXISTS proposals(id TEXT PRIMARY KEY,kind TEXT NOT NULL,text TEXT NOT NULL,status TEXT NOT NULL DEFAULT 'pending_connection',created_at TEXT NOT NULL);
 CREATE TABLE IF NOT EXISTS chat_context(id TEXT PRIMARY KEY, payload TEXT NOT NULL);
 PRAGMA user_version=3;").map_err(storage::err)}
fn read_context<T: serde::de::DeserializeOwned>(row: &rusqlite::Row<'_>, index: usize) -> rusqlite::Result<Option<T>> {
    row.get::<_, Option<String>>(index)?.map(|payload| serde_json::from_str(&payload).map_err(|error|
        rusqlite::Error::FromSqlConversionFailure(index, rusqlite::types::Type::Text, Box::new(error))
    )).transpose()
}

pub fn messages(c:&Connection)->Result<Vec<Message>>{let mut stmt=c.prepare("SELECT m.id,m.role,m.body,m.action,m.created_at,ctx.payload FROM (SELECT rowid,* FROM messages ORDER BY rowid DESC LIMIT 200) m LEFT JOIN chat_context ctx ON ctx.id=m.id AND m.action='clarify' ORDER BY m.rowid ASC").map_err(storage::err)?;let rows=stmt.query_map([],|r|Ok(Message{id:r.get(0)?,role:r.get(1)?,body:r.get(2)?,action:r.get(3)?,created_at:r.get(4)?,follow_up:read_context(r,5)?})).map_err(storage::err)?.collect::<std::result::Result<Vec<_>,_>>().map_err(storage::err)?;Ok(rows)}
pub fn proposals(c:&Connection)->Result<Vec<Proposal>>{let mut stmt=c.prepare("SELECT p.id,p.kind,p.text,p.status,p.created_at,ctx.payload FROM proposals p LEFT JOIN chat_context ctx ON ctx.id=p.id ORDER BY p.created_at DESC").map_err(storage::err)?;let rows=stmt.query_map([],|r|Ok(Proposal{id:r.get(0)?,kind:r.get(1)?,text:r.get(2)?,status:r.get(3)?,created_at:r.get(4)?,event_details:read_context(r,5)?})).map_err(storage::err)?.collect::<std::result::Result<Vec<_>,_>>().map_err(storage::err)?;Ok(rows)}
fn propose(c:&Connection,id:&str,kind:&str,text:&str)->Result<()>{if text.trim().is_empty()||text.chars().count()>20000{return Err("Describe el evento o la tarea".into())}c.execute("INSERT INTO proposals(id,kind,text,created_at) VALUES (?1,?2,?3,?4)",params![id,kind,text,storage::now()]).map_err(storage::err)?;Ok(())}
fn truncate_title(s:&str, max:usize)->String { if s.chars().count()<=max { s.to_string() } else { let t:String=s.chars().take(max).collect(); format!("{t}…") } }
fn undo_note(c:&Connection)->std::result::Result<(String,&'static str),String> {
 match c.query_row("SELECT id,body FROM notes ORDER BY created_at DESC LIMIT 1",[],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?))) {
  Ok((id,body))=>{c.execute("DELETE FROM notes WHERE id=?1",[&id]).map_err(storage::err)?;Ok((format!("Borré la nota «{}».",truncate_title(&body,40)),"undo"))},
  Err(_)=>Ok(("No hay notas que borrar.".into(),"undo")),
 }
}
fn undo_reminder(c:&Connection)->std::result::Result<(String,&'static str),String> {
 match c.query_row("SELECT id,title FROM reminders WHERE completed=0 ORDER BY rowid DESC LIMIT 1",[],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?))) {
  Ok((id,title))=>{c.execute("DELETE FROM reminders WHERE id=?1",[&id]).map_err(storage::err)?;Ok((format!("Borré el recordatorio «{}».",truncate_title(&title,40)),"undo"))},
  Err(_)=>Ok(("No hay recordatorios que borrar.".into(),"undo")),
 }
}
fn undo_event(c:&Connection)->std::result::Result<(String,&'static str),String> {
 match c.query_row("SELECT id,text FROM proposals WHERE status IN ('pending_connection','saving','saved') ORDER BY created_at DESC LIMIT 1",[],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?))) {
  Ok((id,text))=>{c.execute("DELETE FROM proposals WHERE id=?1",[&id]).map_err(storage::err)?;Ok((format!("Borré el evento «{}».",truncate_title(&text,40)),"undo"))},
  Err(_)=>Ok(("No hay eventos que borrar.".into(),"undo")),
 }
}
pub fn submit(c:&mut Connection,id:&str,text:&str,action:Action)->Result<Vec<Message>>{
 if uuid::Uuid::parse_str(id).is_err(){return Err("Identificador no válido".into())}
 if text.trim().is_empty()||text.chars().count()>20000{return Err("Escribe entre 1 y 20000 caracteres".into())}
 // A request id prevents a retry after a lost response from creating the action twice.
 if c.query_row("SELECT EXISTS(SELECT 1 FROM messages WHERE id=?1)",[id],|r|r.get::<_,bool>(0)).map_err(storage::err)? {return messages(c)}
 let tx=c.transaction().map_err(storage::err)?;
 let reply_id=format!("reply:{id}");
 let (reply,route)=match action {
 Action::Note{body}=>{storage::add_note(&tx,&body)?;("He guardado tu nota en Notas. Puedes seguir escribiéndome aquí o abrirla en la app.".into(),"notes")},
 Action::Reminder{title,due_at,alarm}=>{let r=storage::add_reminder_with(&tx,&title,&due_at,alarm)?;let local=chrono::DateTime::parse_from_rfc3339(&r.due_at).map_err(storage::err)?.with_timezone(&chrono::Local);
  let reply=if alarm{format!("Listo: alarma «{}» el {}. Sonará mientras Duck esté abierto y el Mac despierto; podrás pararla o posponerla.",r.title,local.format("%d/%m a las %H:%M"))}else{format!("Listo: te recordaré «{}» el {}. Duck debe seguir abierto para avisarte.",r.title,local.format("%d/%m a las %H:%M"))};
  (reply,"reminders")},
 Action::Event{text}=>{propose(&tx,id,"event",&text)?;("¿Quieres guardar este evento? Todavía no se ha enviado.".into(),"agenda")},
 Action::CreateEvent{event}=>{
     let (_, start, _) = crate::calendar::validate_event(&event.title,&event.start,&event.end)?;
     if start <= chrono::Utc::now() { return Err("Elige una fecha y hora futuras para el evento.".into()); }
     propose(&tx,id,"event",&event.title)?;
     tx.execute("INSERT INTO chat_context VALUES (?1,?2)",params![id,serde_json::to_string(&event).map_err(storage::err)?]).map_err(storage::err)?;
     ("Estoy enviando el evento a Calendario. Si se interrumpe el guardado, revísalo antes de repetirlo.".into(),"agenda")
 },
 Action::Task{text}=>{
  let normalized = text.replace(", y ", ", ").replace(", Y ", ", ").replace(" y ", ", ").replace(" Y ", ", ");
  let items:Vec<String> = normalized.split(',').map(|s|s.trim().to_string()).filter(|s|!s.is_empty()).collect();
  if items.len()>1 {
   for (i,item) in items.iter().enumerate() {
    let sub_id = format!("{id}:{i}");
    propose(&tx,&sub_id,"task",item)?;
   }
   (format!("He añadido {} tareas a Agenda.",items.len()),"agenda")
  } else {
   propose(&tx,id,"task",&text)?;
   ("La tarea está en Agenda.".into(),"agenda")
  }
 },
 Action::Agents=>("Abre Agentes para ver cómo van tus sesiones de Claude Code y Codex.".into(),"agents"),
 Action::Report=>{let s=storage::snapshot(&tx)?;let completed=s.reminders.iter().filter(|r|r.completed).count();let pending=proposals(&tx)?.iter().filter(|p|p.status=="pending_connection").count();(format!("En este Mac tienes {} notas, {} recordatorios pendientes y {} completados. Hay {} borradores para Google. Puedes ver el detalle en Reporting.",s.notes.len(),s.reminders.len()-completed,completed,pending),"reporting")},
 Action::Undo{target}=>{
  let reply = match target.as_deref() {
   Some("note") => undo_note(&tx),
   Some("reminder") => undo_reminder(&tx),
   Some("event") => undo_event(&tx),
   _ => {
    // Find the most recent action from chat to decide what to undo
    let last_action:Option<String> = tx.query_row(
     "SELECT action FROM messages WHERE role='assistant' AND action IN ('notes','reminders','agenda','event-saved') ORDER BY rowid DESC LIMIT 1",
     [],|r|r.get(0)).ok();
    match last_action.as_deref() {
     Some("notes") => undo_note(&tx),
     Some("reminders") => undo_reminder(&tx),
     Some("agenda"|"event-saved") => undo_event(&tx),
     _ => Ok(("No hay nada reciente que borrar.".into(),"undo")),
    }
   }
  }.map_err(storage::err)?;
  reply
 },
 Action::Summary=>{
  let today = chrono::Local::now().format("%Y-%m-%d").to_string();
  let s = storage::snapshot(&tx)?;
  let notes_today:Vec<_> = s.notes.iter().filter(|n|n.created_at.starts_with(&today)).collect();
  let reminders_today:Vec<_> = s.reminders.iter().filter(|r|!r.completed && r.due_at.starts_with(&today)).collect();
  let mut cal_events = crate::calendar::fetch_today_events().unwrap_or_default();
  cal_events.sort_by(|a,b|a.start.partial_cmp(&b.start).unwrap_or(std::cmp::Ordering::Equal));
  let has_content = !notes_today.is_empty() || !reminders_today.is_empty() || !cal_events.is_empty();
  if !has_content {
   ("Hoy no tienes nada apuntado.".into(),"home")
  } else {
   let mut parts = Vec::new();
   if !cal_events.is_empty() {
    let items:Vec<String> = cal_events.iter().map(|e|{
     if e.all_day {
      format!("· {} (todo el día, {})",e.title,e.calendar)
     } else {
      let start = chrono::DateTime::from_timestamp(e.start as i64,0).map(|t|t.with_timezone(&chrono::Local).format("%H:%M").to_string()).unwrap_or_default();
      let end = chrono::DateTime::from_timestamp(e.end as i64,0).map(|t|t.with_timezone(&chrono::Local).format("%H:%M").to_string()).unwrap_or_default();
      format!("· {} de {} a {} ({})",e.title,start,end,e.calendar)
     }
    }).collect();
    parts.push(format!("Calendario:\n{}",items.join("\n")));
   }
   if !reminders_today.is_empty() {
    let items:Vec<String> = reminders_today.iter().map(|r|{
     let time = chrono::DateTime::parse_from_rfc3339(&r.due_at).map(|t|t.with_timezone(&chrono::Local).format("%H:%M").to_string()).unwrap_or_default();
     format!("· {} a las {}",r.title,time)
    }).collect();
    parts.push(format!("Recordatorios:\n{}",items.join("\n")));
   }
   if !notes_today.is_empty() {
    let items:Vec<String> = notes_today.iter().map(|n|{
     let preview:String = n.body.chars().take(40).collect();
     format!("· {}{}",preview,if n.body.chars().count()>40{"…"}else{""})
    }).collect();
    parts.push(format!("Notas:\n{}",items.join("\n")));
   }
   (format!("Tu día:\n\n{}\n\nPara borrar algo, escríbeme «bórralo» o abre la app.",parts.join("\n\n")),"home")
  }
 },
 Action::Clarify{message,follow_up}=>{
     if let Some(follow_up) = follow_up {
         tx.execute("INSERT INTO chat_context VALUES (?1,?2)",params![reply_id,serde_json::to_string(&follow_up).map_err(storage::err)?]).map_err(storage::err)?;
     }
     (message.chars().take(2000).collect(),"clarify")
 },
 };
 let at=storage::now();
 tx.execute("INSERT INTO messages VALUES (?1,'user',?2,NULL,?3)",params![id,text,at]).map_err(storage::err)?;
 tx.execute("INSERT INTO messages VALUES (?1,'assistant',?2,?3,?4)",params![reply_id,reply,route,at]).map_err(storage::err)?;
 tx.commit().map_err(storage::err)?;messages(c)
}
#[tauri::command]
pub fn get_messages(db:tauri::State<Db>)->Result<Vec<Message>>{messages(&*db.lock().map_err(storage::err)?)}
#[tauri::command]
pub fn get_proposals(db:tauri::State<Db>)->Result<Vec<Proposal>>{proposals(&*db.lock().map_err(storage::err)?)}
#[tauri::command]
pub async fn submit_chat(app: AppHandle, id: String, text: String, action: Action) -> Result<Vec<Message>> {
    let auto_event = match &action {
        Action::CreateEvent { event } => Some(event.clone()),
        _ => None,
    };
    let (mut result, duplicate) = {
        let state = app.state::<Db>();
        let mut db = state.lock().map_err(storage::err)?;
        let duplicate = db.query_row(
            "SELECT EXISTS(SELECT 1 FROM messages WHERE id=?1)", [&id], |row| row.get::<_, bool>(0),
        ).map_err(storage::err)?;
        (submit(&mut db, &id, &text, action)?, duplicate)
    };
    if let Some(event) = auto_event.filter(|_| !duplicate) {
        let selected = crate::desktop::get_settings(app.state::<crate::desktop::DesktopState>()).calendar_id;
        let (body, route) = match selected {
            None => ("Ya tengo los datos. Elige tu calendario de Google en Agenda. Allí puedes guardar el evento con los datos que ya me diste.".into(), "event-retry"),
            Some(calendar_id) => match crate::calendar::save_calendar_event(
                app.clone(), id.clone(), calendar_id, event.title.clone(), event.start.clone(), event.end,
            ).await {
                Ok(calendar) => {
                    let local = chrono::DateTime::parse_from_rfc3339(&event.start).map_err(storage::err)?.with_timezone(&chrono::Local);
                    (format!("Guardé «{}» en {} el {}.", event.title, calendar.title, local.format("%d/%m a las %H:%M")), "event-saved")
                }
                Err(error) => (format!("No pude completar el guardado. {error} Puedes revisar el estado en Agenda."), "event-retry"),
            },
        };
        let state = app.state::<Db>();
        let db = state.lock().map_err(storage::err)?;
        db.execute("UPDATE messages SET body=?1,action=?2 WHERE id=?3",
            params![body, route, format!("reply:{id}")],
        ).map_err(storage::err)?;
        result = messages(&db)?;
    }
    let _ = app.emit("data-changed", ());
    let _ = app.emit("chat-changed", ());
    Ok(result)
}
#[tauri::command]
pub fn dismiss_proposal(app:AppHandle,id:String,db:tauri::State<Db>)->Result<()> {db.lock().map_err(storage::err)?.execute("UPDATE proposals SET status='dismissed' WHERE id=?1 AND status='pending_connection'",[id]).map_err(storage::err)?;let _=app.emit("data-changed",());Ok(())}
#[cfg(test)] mod tests {
 use super::*;
 fn db()->Connection{let c=storage::open(std::path::Path::new(":memory:")).unwrap();migrate(&c).unwrap();c}
 #[test] fn note_and_reply_are_saved_together(){let mut c=db();let id=uuid::Uuid::new_v4().to_string();let m=submit(&mut c,&id,"nota: Hola",Action::Note{body:"Hola".into()}).unwrap();assert_eq!(m.len(),2);assert_eq!(m[1].action.as_deref(),Some("notes"));assert_eq!(storage::snapshot(&c).unwrap().notes.len(),1);submit(&mut c,&id,"nota: Hola",Action::Note{body:"Hola".into()}).unwrap();assert_eq!(storage::snapshot(&c).unwrap().notes.len(),1);}
 #[test] fn alarm_from_chat_is_saved_as_alarm_and_the_reply_says_so(){
  let mut c=db();let due=(chrono::Utc::now()+chrono::Duration::hours(2)).to_rfc3339();
  let m=submit(&mut c,&uuid::Uuid::new_v4().to_string(),"alarma",Action::Reminder{title:"Despertar".into(),due_at:due.clone(),alarm:true}).unwrap();
  assert!(m[1].body.contains("alarma")&&m[1].body.contains("Despertar"));assert_eq!(m[1].action.as_deref(),Some("reminders"));
  submit(&mut c,&uuid::Uuid::new_v4().to_string(),"recordatorio",Action::Reminder{title:"Normal".into(),due_at:due,alarm:false}).unwrap();
  let s=storage::snapshot(&c).unwrap();
  assert_eq!(s.reminders.iter().filter(|r|r.alarm).map(|r|r.title.as_str()).collect::<Vec<_>>(),vec!["Despertar"]);
  assert_eq!(s.reminders.len(),2);
 }
 // The interpreter sends `alarm` only for alarms; an old payload without it must still work.
 #[test] fn reminder_payload_without_alarm_field_still_parses(){
  let a:Action=serde_json::from_str(r#"{"type":"reminder.create","title":"x","dueAt":"2999-01-01T00:00:00Z"}"#).unwrap();
  assert!(matches!(a,Action::Reminder{alarm:false,..}));
  let b:Action=serde_json::from_str(r#"{"type":"reminder.create","title":"x","dueAt":"2999-01-01T00:00:00Z","alarm":true}"#).unwrap();
  assert!(matches!(b,Action::Reminder{alarm:true,..}));
 }
 #[test] fn invalid_action_rolls_back_message(){let mut c=db();assert!(submit(&mut c,&uuid::Uuid::new_v4().to_string(),"nota",Action::Note{body:" ".into()}).is_err());assert!(messages(&c).unwrap().is_empty());}
 #[test] fn google_proposals_never_claim_to_be_sent(){let mut c=db();submit(&mut c,&uuid::Uuid::new_v4().to_string(),"evento",Action::Event{text:"Reunión".into()}).unwrap();submit(&mut c,&uuid::Uuid::new_v4().to_string(),"tarea",Action::Task{text:"Comprar leche".into()}).unwrap();let p=proposals(&c).unwrap();assert_eq!(p.len(),2);assert!(p.iter().all(|p|p.status=="pending_connection"));assert!(messages(&c).unwrap().iter().filter(|m|m.role=="assistant").all(|m|m.body.contains("no se ha enviado")));}
}
