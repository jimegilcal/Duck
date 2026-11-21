import { useEffect, useState, type ReactNode } from 'react';
import { invoke } from '@tauri-apps/api/core';

export interface Calendar { id: string; title: string; account: string }

export function useCalendarDestination() {
  const [calendar, setCalendar] = useState<Calendar | null>(null);
  const [choices, setChoices] = useState<Calendar[] | null>(null);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');

  useEffect(() => {
    let active = true;
    void invoke<Calendar | null>('current_calendar').then(value => {
      if (active) setCalendar(value);
    }).catch(error => {
      if (active) setError(String(error));
    }).finally(() => {
      if (active) setLoading(false);
    });
    return () => { active = false; };
  }, []);

  async function chooseCalendar() {
    setBusy(true);
    setError('');
    try {
      const calendars = await invoke<Calendar[]>('list_calendars');
      setChoices(calendars);
      setCalendar(previous => calendars.find(item => item.id === previous?.id) ?? null);
    } catch (error) {
      setChoices(null);
      setError(String(error));
    } finally { setBusy(false); }
  }

  async function selectCalendar(id: string) {
    if (!id) return;
    setBusy(true);
    setError('');
    try {
      setCalendar(await invoke<Calendar>('select_calendar', { id }));
      setChoices(null);
    } catch (error) { setError(String(error)); }
    finally { setBusy(false); }
  }

  return { calendar, choices, loading, busy, error, chooseCalendar, selectCalendar };
}

export function CalendarDestination({ destination, disabled = false, children }: {
  destination: ReturnType<typeof useCalendarDestination>;
  disabled?: boolean;
  children?: ReactNode;
}) {
  const { calendar, choices, loading, busy, error, chooseCalendar, selectCalendar } = destination;
  const label = ({ title, account }: Calendar) => `${title} · ${account}`;
  if (loading) return <p role="status">Comprobando el calendario…</p>;

  return <>
    <p className="calendar-destination" role="status">
      {calendar ? <>Destino <strong>{label(calendar)}</strong></> : 'Sin calendario seleccionado.'}
    </p>
    {!calendar && <p>Para usar Google Calendar, añade tu cuenta en <strong>Ajustes del Sistema → Cuentas de internet</strong> y activa <strong>Calendarios</strong>. Después, elige aquí el calendario de Google.</p>}
    <div className="calendar-actions">
      <button className="secondary" type="button" disabled={disabled || busy} onClick={() => void chooseCalendar()}>
        {busy ? 'Consultando Calendario…' : calendar ? 'Cambiar calendario' : 'Elegir calendario'}
      </button>
      {children}
    </div>
    {choices && choices.length > 0 && <label>Calendarios disponibles
      <select value="" disabled={disabled || busy} onChange={event => void selectCalendar(event.target.value)}>
        <option value="">Selecciona un calendario</option>
        {choices.map(calendar => <option key={calendar.id} value={calendar.id}>{label(calendar)}</option>)}
      </select>
    </label>}
    {choices?.length === 0 && <p role="status">macOS no encontró calendarios de internet con permiso de escritura.</p>}
    {error && <p className="form-error" role="alert">{error}</p>}
  </>;
}

export function CalendarSetup() {
  const destination = useCalendarDestination();
  return <section className="settings-section">
    <h2>Calendario para tus eventos</h2>
    <div className="calendar-confirm"><CalendarDestination destination={destination}/></div>
  </section>;
}
