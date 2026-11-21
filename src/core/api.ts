import { invoke } from '@tauri-apps/api/core';
import { interpretWithContext } from './interpreter';
import type { Settings,Snapshot,ChatMessage,Proposal } from './types';
export const api={
 chat:()=>invoke<ChatMessage[]>('get_messages'),proposals:()=>invoke<Proposal[]>('get_proposals'),
 submit:async(id:string,text:string)=>{const context=await invoke<ChatMessage[]>('get_messages');return invoke<ChatMessage[]>('submit_chat',{id,text,action:interpretWithContext(text,context)});},
 dismissProposal:(id:string)=>invoke('dismiss_proposal',{id}),closeChat:()=>invoke('close_chat'),openChat:()=>invoke('open_chat'),navigate:(page:string)=>invoke('navigate_panel',{page}),
 snapshot:()=>invoke<Snapshot>('get_snapshot'),settings:()=>invoke<Settings>('get_settings'),
 note:(body:string)=>invoke('create_note',{body}),editNote:(id:string,body:string)=>invoke('update_note',{id,body}),deleteNote:(id:string)=>invoke('delete_note',{id}),
 reminder:(title:string,dueAt:string,alarm=false)=>invoke('create_reminder',{title,dueAt,alarm}),
 ringingAlarms:()=>invoke<{id:string;title:string}[]>('ringing_alarms'),stopAlarm:(id:string)=>invoke('stop_alarm',{id}),snoozeAlarm:(id:string,minutes=5)=>invoke('snooze_alarm',{id,minutes}),completeReminder:(id:string,completed:boolean)=>invoke('complete_reminder',{id,completed}),deleteReminder:(id:string)=>invoke('delete_reminder',{id}),
 saveSettings:(settings:Settings)=>invoke<Settings>('save_settings',{settings}),toggle:()=>invoke('toggle_duck'),test:()=>invoke('test_notification'),
};
