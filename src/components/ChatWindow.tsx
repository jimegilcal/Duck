import { useEffect,useRef,useState,type FormEvent, type RefObject } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { ArrowUp, ArrowUpRight, Bot, X } from 'lucide-react';
import { api } from '../core/api';
import type { ChatMessage } from '../core/types';
import { AgentRoster } from './AgentRoster';
import { DictationButton, DictationStatus, useDictation } from './DictationButton';
const labels:Record<string,string>={notes:'Ver nota',reminders:'Ver recordatorio',agenda:'Ver Agenda','event-retry':'Abrir Agenda',agents:'Ver agentes',reporting:'Ver reporting',undo:'Abrir app',home:'Abrir app'};
export function ChatWindow({textDraft}:{textDraft:RefObject<string>}){
 const [reply,setReply]=useState<ChatMessage|null>(null),[text,setInputText]=useState(textDraft.current),[busy,setBusy]=useState(false),[error,setError]=useState('');
 const [showAgents,setShowAgents]=useState(false);
 useEffect(()=>{
  let active=true;
  void api.chat().then(messages=>{
   const last=messages.at(-1);
   if(active && last?.followUp && last.followUp.kind!=='unknown')setReply(current=>current??last);
  }).catch(console.error);
  return ()=>{active=false;};
 },[]);
 const draft=useRef<{id:string;text:string}|null>(null),sending=useRef(false),input=useRef<HTMLTextAreaElement>(null);
 function setText(value:string){textDraft.current=value;setInputText(value)}
 const dictation=useDictation(text,setText,message=>{
  setError(message);
  if(message)setShowAgents(false);
 });
 useEffect(()=>{
  void invoke('set_chat_reply',{expanded:showAgents||!!reply||!!error,calendarForm:false}).catch(console.error);
 },[showAgents,reply,error]);
 function dismissReply(){setReply(null);setError('');}
 async function submit(e?:FormEvent) {
  e?.preventDefault();
  const value=text.trim();
  if(!value||dictation.active||sending.current)return;
  sending.current=true;
  setBusy(true);
  setError('');
  setShowAgents(false);
  if(draft.current?.text!==value)draft.current={id:crypto.randomUUID(),text:value};
  const id=draft.current.id;
  try {
   const messages=await api.submit(id,value);
   const answer=messages.find(m=>m.id==='reply:'+id)??null;
   setReply(answer);
   if (answer?.action==='agents')setShowAgents(true);
   if(textDraft.current.trim()===value)setText('');
   draft.current=null;
   input.current?.focus();
  } catch(error){setError(String(error));}
  finally{sending.current=false;setBusy(false);}
 }

 return <div className={`quick-chat${showAgents?' agents-open':''}`}>
  {showAgents ? <section id="quick-agents" className="quick-reply quick-agents" aria-label="Agentes">
   <div className="quick-agents-header"><strong>Agentes</strong><span>Claude Code · Codex</span></div>
   <AgentRoster compact/>
  </section> : (reply||error)&&<div className={`quick-reply ${error?'form-error':''}`} role="status"><button className="dismiss-reply" aria-label="Ocultar respuesta" onClick={dismissReply}><X size={12}/></button><p>{error||reply?.body}</p>{reply?.action&&labels[reply.action]&&<button className="message-action" onClick={()=>void api.navigate(reply.action==='event-retry'?'agenda':reply.action!).catch(e=>setError(String(e)))}>{labels[reply.action]}<ArrowUpRight size={12}/></button>}</div>}
  <form className="quick-input" onSubmit={e=>void submit(e)}>
   <textarea ref={input} aria-label="Escribe a Duck" autoFocus value={text} maxLength={20000} readOnly={dictation.active}
    onChange={e=>setText(e.target.value)}
    onKeyDown={e=>{
     if(e.key==='Enter'&&!e.shiftKey&&!e.nativeEvent.isComposing){e.preventDefault();void submit()}
     if(e.key==='Escape'){if(showAgents)setShowAgents(false);else void api.closeChat();}
    }}
    placeholder={busy?'Guardando…':'Escribe a Duck'} rows={1}/>
   <button type="button" className="quick-agents-toggle" aria-pressed={showAgents} aria-controls={showAgents?'quick-agents':undefined} onClick={()=>setShowAgents(open=>!open)}><Bot size={15} aria-hidden="true"/>Agentes</button>
   <DictationButton dictation={dictation} disabled={busy} compact/>
   <button className="send-button" aria-label="Enviar mensaje" disabled={busy||dictation.active||!text.trim()}><ArrowUp size={17}/></button>
  </form>
  <div className="quick-footer">
   {dictation.active ? <DictationStatus dictation={dictation}/> : <span>{showAgents ? 'Resumen en directo · local' : 'Chat local'}</span>}
   <button onClick={()=>void api.navigate(showAgents?'agents':'home').catch(e=>setError(String(e)))}>Abrir app <ArrowUpRight size={11}/></button>
   <button className="quick-close" aria-label="Ocultar barra de chat" title="Ocultar chat (Esc)" onClick={()=>void api.closeChat().catch(e=>setError(String(e)))}><X size={12}/></button>
  </div>
 </div>
}
