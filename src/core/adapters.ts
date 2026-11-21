import type { DuckEvent } from './types';
// Future integrations implement these boundaries. Claude Code and Codex report through local hooks (see scripts/duck_agent_hook.py).
export interface EventSource {provider:string;connect(onEvent:(event:DuckEvent)=>void):Promise<void>;disconnect():Promise<void>}
export interface CalendarAdapter {propose(input:{title:string;start:string;end:string}):Promise<{proposalId:string}>;confirm(proposalId:string):Promise<{eventId:string}>}
export interface VoiceAdapter {start():Promise<void>;stop():Promise<{transcript:string}>}
export const agentProviders = [
  {id:'codex',name:'Codex'},
  {id:'claude-code',name:'Claude Code'},
] as const;
export const agentStates = {working:'Trabajando',finished:'Terminado',approval:'Esperando aprobación',error:'Error',unknown:'Desconocido'} as const;
