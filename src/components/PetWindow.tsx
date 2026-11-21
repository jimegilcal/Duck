import { useEffect, useRef, useState, type CSSProperties } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { DuckScene } from './DuckScene';
import { ChatWindow } from './ChatWindow';
import { duckSizeLimits, defaultSettings, type Settings, type DuckEvent, type RingingAlarm } from '../core/types';

type ChatLayout = {
 modelX:number; modelY:number;
 chatX:number; chatY:number; chatWidth:number; chatHeight:number;
};

export function PetWindow(){
 const [settings,setSettings]=useState(defaultSettings),[pulse,setPulse]=useState(0),[active,setActive]=useState(true),[leaving,setLeaving]=useState(false),[event,setEvent]=useState<DuckEvent|null>(null);
 const [chatLayout,setChatLayout]=useState<ChatLayout|null>(null);
 const [alarms,setAlarms]=useState<RingingAlarm[]>([]);
 const chatOpen=chatLayout!==null;
 const pendingClick=useRef<ReturnType<typeof setTimeout>|null>(null);
 const draft=useRef('');
 const [resizing,setResizing]=useState(false);
 const [draftSize,setDraftSize]=useState(settings.size);
 const dragged=useRef(false);
 const pointer=useRef<{x:number;y:number;dragging:boolean}|null>(null);
 useEffect(()=>{
  const stops:Promise<()=>void>[]=[];
  void invoke<Settings>('get_settings').then(s=>{setSettings(s);setActive(s.visible)}).catch(console.error);
  stops.push(listen<ChatLayout|null>('chat-layout',e=>setChatLayout(e.payload)));
  void invoke<ChatLayout|null>('get_chat_layout').then(setChatLayout).catch(console.error);
  stops.push(listen<Settings>('settings-changed',e=>setSettings(e.payload)));
  stops.push(listen('duck-enter',()=>{setActive(true);setLeaving(false);setPulse(p=>p+1)}));
  stops.push(listen('duck-leave',()=>setLeaving(true)));
  stops.push(listen('duck-clear-notification',()=>setEvent(null)));
  stops.push(listen<boolean>('duck-active',e=>setActive(e.payload)));
  stops.push(listen<DuckEvent>('duck-notification',e=>{setActive(true);setLeaving(false);setEvent(e.payload.metadata?.alarm===true?null:e.payload);setPulse(p=>p+1)}));
  stops.push(listen<RingingAlarm[]>('alarm-changed',e=>{setAlarms(e.payload);if(e.payload.length>0){setActive(true);setLeaving(false);}}));
  void invoke<RingingAlarm[]>('ringing_alarms').then(setAlarms).catch(console.error);
  return()=>{stops.forEach(s=>void s.then(f=>f()));if(pendingClick.current)clearTimeout(pendingClick.current);};
 },[]);
 useEffect(()=>{if(!event)return;const timer=setTimeout(()=>setEvent(null),7600);return()=>clearTimeout(timer)},[event]);
 function activate(clickCount:number) {
  if (dragged.current) return;
  if (pendingClick.current || clickCount>1) {
   if (pendingClick.current) clearTimeout(pendingClick.current);
   pendingClick.current=null;
   void (async () => {
    if (chatOpen) await invoke('close_chat');
    await invoke('open_panel');
   })().catch(console.error);
   return;
  }
  pendingClick.current=setTimeout(()=>{
   pendingClick.current=null;
   setPulse(p=>p+1);
   void invoke('toggle_chat').catch(console.error);
  },450);
 }
 const commitTimer=useRef<ReturnType<typeof setTimeout>|null>(null);
 useEffect(()=>setDraftSize(settings.size),[settings.size]);
 useEffect(()=>()=>{if(commitTimer.current)clearTimeout(commitTimer.current)},[]);
 async function commitSize(size:number) {
  if(commitTimer.current){clearTimeout(commitTimer.current);commitTimer.current=null;}
  if(resizing||size===settings.size)return;
  setResizing(true);
  try {setSettings(await invoke<Settings>('save_settings',{settings:{...settings,size}}));}
  catch(error){console.error(error);setDraftSize(settings.size);}
  finally {setResizing(false);}
 }
 function changeSize(size:number) {
  setDraftSize(size);
  // The window only resizes on release (or after a short pause) so the bar does not move under the cursor.
  if(commitTimer.current)clearTimeout(commitTimer.current);
  commitTimer.current=setTimeout(()=>void commitSize(size),900);
 }

 return <div style={{'--duck-size':`${settings.size}px`} as CSSProperties} className={`pet-window ${leaving?'leaving':''} ${!settings.animations?'no-motion':''}`}>
  {chatLayout&&<div className="duck-conversation" style={{left:chatLayout.chatX+8,top:chatLayout.chatY,width:chatLayout.chatWidth-16,height:chatLayout.chatHeight-6}}><ChatWindow textDraft={draft}/></div>}
  <div className="pet-model" style={{left:chatLayout?.modelX??0,top:chatLayout?.modelY??0}}>
  <div role="button" aria-label="Duck. Un clic alterna el chat, doble clic abre la app y clic secundario abre los ajustes." tabIndex={chatOpen?0:-1} className="pet-stage" onPointerDown={e=>{if(e.button!==0)return;dragged.current=false;pointer.current={x:e.screenX,y:e.screenY,dragging:false};}}
   onPointerMove={async e=>{const p=pointer.current;if(!(e.buttons&1)){pointer.current=null;return;}if(!p||p.dragging||Math.hypot(e.screenX-p.x,e.screenY-p.y)<5)return;p.dragging=true;dragged.current=true;if(pendingClick.current){clearTimeout(pendingClick.current);pendingClick.current=null;}try{await invoke('prepare_drag');await getCurrentWindow().startDragging();}catch(e){console.error(e)}finally{await invoke('finish_drag').catch(console.error);pointer.current=null;}}}
   onPointerUp={()=>{pointer.current=null;}}
   onPointerCancel={()=>{pointer.current=null;dragged.current=true;}}
   onClick={e=>{if(e.button===0&&!e.ctrlKey)activate(e.detail);}}
   onContextMenu={e=>{
    e.preventDefault();
    if(pendingClick.current)clearTimeout(pendingClick.current);
    pendingClick.current=null;
    pointer.current=null;
    void invoke('show_duck_menu').catch(console.error);
   }}
   onKeyDown={e=>{if(e.key==='Enter'||e.key===' '){e.preventDefault();void invoke('toggle_chat').catch(console.error);}}}>

   <DuckScene skin={settings.skin} animated={settings.animations} active={active} pulse={pulse} mask/>
  </div>
  <div className="pet-size-controls" role="group" aria-label="Tamaño de Duck">
    <input type="range" className="pet-size-slider" aria-label="Tamaño de Duck" title="Tamaño de Duck"
      min={duckSizeLimits.min} max={duckSizeLimits.max} step={1} value={draftSize} disabled={resizing}
      onChange={e=>changeSize(Number(e.currentTarget.value))}
      onPointerUp={e=>void commitSize(Number(e.currentTarget.value))}
      onKeyUp={e=>void commitSize(Number(e.currentTarget.value))}/>
  </div>
  {alarms.length>0&&<div className="bubble alarm" role="alert" aria-live="assertive"><small>⏰ ALARMA{alarms.length>1?` · ${alarms.length} sonando`:''}</small><strong title={alarms[0].title}>{alarms[0].title}</strong>
   <div className="alarm-actions"><button className="alarm-stop" onClick={()=>void invoke('stop_alarm',{id:alarms[0].id}).catch(console.error)}>Parar</button><button className="alarm-snooze" onClick={()=>void invoke('snooze_alarm',{id:alarms[0].id,minutes:5}).catch(console.error)}>5 min más</button></div></div>}
  {alarms.length===0&&event&&!chatOpen&&<button className="bubble"onClick={()=>void invoke('navigate_panel',{page:'settings'}).catch(console.error)}><small>{event.provider==='test'?'PRUEBA DE NOTIFICACIÓN':'RECORDATORIO'}</small><strong>{event.title}</strong><span>{event.description}</span></button>}
  {event&&chatOpen&&<button className="chat-alert" onClick={()=>void invoke('navigate_panel',{page:'settings'}).catch(console.error)}>{event.title} ↗</button>}
  </div>
 </div>
}
