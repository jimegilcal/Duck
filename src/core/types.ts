import type { EventDetails, FollowUp } from './interpreter';

export const accessories = [
  { id: 'classic', label: 'Sin accesorio' },
  { id: 'glasses', label: 'Gafas redondas' },
  { id: 'party', label: 'Gorro de fiesta' },
  { id: 'bow', label: 'Pajarita rosa' },
  { id: 'frog', label: 'Gorro de ranita' },
  { id: 'headphones', label: 'Auriculares morados' },
] as const;

export type Skin = typeof accessories[number]['id'];

export const duckSizeLimits = {min:64,max:480} as const;

export interface Settings {visible:boolean;size:number;animations:boolean;notifications:boolean;companionAlerts:boolean;shortcut:string;skin:Skin;calendarId:string|null;position:[number,number]|null;usageTracking:boolean}
export interface DuckEvent {id:string;provider:string;session:string|null;project:string|null;status:'working'|'finished'|'approval'|'error'|'unknown';eventType:string;title:string;description:string;timestamp:string;metadata:Record<string,unknown>|null}
export interface Note {id:string;body:string;createdAt:string;updatedAt:string}
export interface Reminder {id:string;title:string;dueAt:string;completed:boolean;notifiedAt:string|null;alarm?:boolean}
export interface RingingAlarm {id:string;title:string}
export interface Snapshot {notes:Note[];reminders:Reminder[];events:DuckEvent[]}
export const defaultSettings:Settings={visible:true,size:144,animations:true,notifications:true,companionAlerts:true,shortcut:'CommandOrControl+Shift+D',skin:'classic',calendarId:null,position:null,usageTracking:false};

export interface ChatMessage {id:string;role:'user'|'assistant';body:string;action:string|null;createdAt:string;followUp?:FollowUp|null}
export interface Proposal {id:string;kind:'event'|'task';text:string;status:'pending_connection'|'dismissed'|'saving'|'saved';createdAt:string;eventDetails?:EventDetails|null}
