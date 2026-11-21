import { useState, type FormEvent } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Check, X } from 'lucide-react';
import { parseSchedule } from '../core/interpreter';
import type { Proposal } from '../core/types';

import { CalendarDestination, useCalendarDestination, type Calendar } from './CalendarDestination';

function localValue(date: Date) {
  const pad = (value: number) => String(value).padStart(2, '0');
  return [date.getFullYear(), pad(date.getMonth()+1), pad(date.getDate())].join('-') +
    'T'+pad(date.getHours())+':'+pad(date.getMinutes());
}

function readLocal(value: string) {
  const date = new Date(value);
  if (!Number.isFinite(date.getTime()) || localValue(date)!==value) {
    throw new Error('Elige una fecha y hora válidas en tu zona horaria.');
  }
  return date.toISOString();
}

export function CalendarConfirm({proposal, cancel}: {proposal:Proposal; cancel:()=>void}) {
  const schedule = parseSchedule(proposal.text, new Date(proposal.createdAt));
  const [title,setTitle] = useState(() => (proposal.eventDetails?.title ?? (schedule.date ? schedule.rest.trim() || proposal.text : proposal.text)).slice(0,240));
  const [start,setStart] = useState(() => proposal.eventDetails ? localValue(new Date(proposal.eventDetails.start)) : schedule.date ? localValue(schedule.date) : '');
  const [duration,setDuration] = useState(() => proposal.eventDetails
    ? Math.round((Date.parse(proposal.eventDetails.end)-Date.parse(proposal.eventDetails.start))/60_000)
    : 30);
  const destination = useCalendarDestination();
  const { calendar } = destination;
  const [saving,setSaving] = useState(false);
  const busy = saving || destination.busy;
  const [error,setError] = useState('');
  const [saved,setSaved] = useState<Calendar|null>(null);

  async function save(event: FormEvent) {
    event.preventDefault();
    if(busy||!calendar||saved)return;
    setSaving(true);
    setError('');
    try {
      const from=readLocal(start);
      const to=new Date(new Date(from).getTime()+duration*60_000).toISOString();
      setSaved(await invoke<Calendar>('save_calendar_event',{proposalId:proposal.id,expectedCalendarId:calendar.id,title,start:from,end:to}));
    } catch(error) { setError(String(error instanceof Error ? error.message : error)); }
    finally { setSaving(false); }
  }
  if(saved) return <div className="calendar-saved" role="status"><Check size={18}/><p>Guardado en <strong>{saved.title}</strong>. Si es un calendario de Google, macOS sincronizará el evento.</p><button className="text-button" onClick={cancel}>Cerrar</button></div>;

  return <form className="calendar-confirm" onSubmit={event=>void save(event)}>
    <label>Título<input value={title} maxLength={240} required onChange={e=>setTitle(e.target.value)}/></label>
    <div className="calendar-time">
      <label>Inicio<input type="datetime-local" required value={start} onChange={e=>setStart(e.target.value)}/></label>
      <label>Minutos<input type="number" min={1} max={1440} required value={duration} onChange={e=>setDuration(Number(e.target.value))}/></label>
    </div>
    <CalendarDestination destination={destination} disabled={saving}>
      <button className="primary" disabled={busy||!calendar||!title.trim()||!start}>{saving?'Guardando…':'Guardar evento'}</button>
    </CalendarDestination>
    {error&&<p className="form-error" role="alert">{error}</p>}
  </form>;
}

export function GoogleDraft({proposal,close}: {proposal:Proposal;close:()=>void}) {
  return <div className="modal-backdrop"><div className="modal" role="dialog" aria-modal="true" aria-label="Revisar evento">
    <div className="modal-heading"><h2>¿Quieres guardar este evento?</h2><button type="button" className="icon-button" aria-label="Cerrar revisión" onClick={close}><X size={18}/></button></div>
    <CalendarConfirm proposal={proposal} cancel={close}/>
  </div></div>;
}
