import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { Mic, Square } from 'lucide-react';

type Phase = 'idle' | 'starting' | 'downloading' | 'listening' | 'finishing';
type DictationEvent = {
  id: string;
  kind: 'downloading' | 'listening' | 'transcribing' | 'partial' | 'finished' | 'failed' | 'cancelled';
  text: string;
};
type Recording = {
  id: string;
  unlisten?: UnlistenFn;
  start?: Promise<void>;
  stopping?: boolean;
};

const statusLabels: Record<Phase, string> = {
  idle: '',
  starting: 'Preparando micrófono…',
  downloading: 'Descargando español…',
  listening: 'Grabando',
  finishing: 'Transcribiendo…',
};

function end(recording: Recording, cancel: boolean) {
  return recording.start?.then(() => invoke<void>('end_dictation', { id: recording.id, cancel }));
}

export function useDictation(
  value: string,
  onText: (value: string) => void,
  onError: (message: string) => void,
  enabled = true,
) {
  const session = useRef<Recording | null>(null);
  const [phase, setPhase] = useState<Phase>('idle');
  const [elapsed, setElapsed] = useState(0);

  useEffect(() => {
    if (phase !== 'listening') return;
    const start = performance.now();
    const timer = window.setInterval(() => {
      setElapsed(Math.floor((performance.now() - start) / 1000));
    }, 250);
    return () => window.clearInterval(timer);
  }, [phase]);

  useEffect(() => {
    if (!enabled) setPhase('idle');
    return () => {
      const recording = session.current;
      if (!recording) return;
      session.current = null;
      recording.unlisten?.();
      void end(recording, true)?.catch(() => {});
    };
  }, [enabled]);

  async function toggle() {
    if (!enabled) return;
    const current = session.current;
    if (current) {
      if (current.stopping) return;
      current.stopping = true;
      if (!current.start) {
        session.current = null;
        setPhase('idle');
        return;
      }
      if (phase !== 'listening') setPhase('finishing');
      try {
        await end(current, false);
      } catch (error) {
        if (session.current !== current) return;
        session.current = null;
        current.unlisten?.();
        void end(current, true)?.catch(() => {});
        setPhase('idle');
        onError(String(error));
      }
      return;
    }

    const recording: Recording = { id: crypto.randomUUID() };
    const prefix = value && !/\s$/.test(value) ? value + ' ' : value;
    session.current = recording;
    setPhase('starting');
    onError('');
    try {
      const unlisten = await listen<DictationEvent>('dictation', ({ payload }) => {
        if (session.current !== recording || payload.id !== recording.id) return;
        if (payload.kind === 'listening' || payload.kind === 'downloading') {
          if (!recording.stopping) {
            if (payload.kind === 'listening') setElapsed(0);
            setPhase(payload.kind);
          }
          return;
        }
        if (payload.kind === 'transcribing') {
          recording.stopping = true;
          setPhase('finishing');
          return;
        }
        if (payload.kind === 'partial' || payload.kind === 'finished') {
          const transcript = payload.text.trim();
          if (transcript) onText((prefix + transcript).slice(0, 20000));
        }
        if (payload.kind === 'partial') return;
        session.current = null;
        recording.unlisten?.();
        setPhase('idle');
        if (payload.kind === 'failed') onError(payload.text);
      });
      if (session.current !== recording) {
        unlisten();
        return;
      }
      recording.unlisten = unlisten;
      recording.start = invoke<void>('start_dictation', { id: recording.id });
      await recording.start;
    } catch (error) {
      if (session.current !== recording) return;
      session.current = null;
      recording.unlisten?.();
      setPhase('idle');
      onError(String(error));
    }
  }

  return { phase, active: phase !== 'idle', elapsed, status: statusLabels[phase], toggle };
}

type Dictation = ReturnType<typeof useDictation>;

export function DictationStatus({ dictation }: { dictation: Dictation }) {
  const recording = dictation.phase === 'listening';
  const duration = `${Math.floor(dictation.elapsed / 60)}:${String(dictation.elapsed % 60).padStart(2, '0')}`;
  return <span className={`dictation-status${recording ? ' is-recording' : ''}`}>
    {recording && <span className="dictation-dot" aria-hidden="true"/>}
    <span role="status">{dictation.status}</span>
    {recording && <span className="dictation-elapsed" aria-hidden="true">{duration}</span>}
  </span>;
}

export function DictationButton({ dictation, disabled = false, compact = false }: {
  dictation: Dictation;
  disabled?: boolean;
  compact?: boolean;
}) {
  const recording = dictation.phase === 'listening';
  const label = recording ? 'Parar grabación' : dictation.phase === 'finishing' ? 'Transcribiendo' : dictation.active ? 'Cancelar dictado' : 'Dictar con el micrófono';
  const Icon = dictation.active ? Square : Mic;
  return <button
    type="button"
    className={`icon-button dictation-button${compact ? ' quick-mic' : ''}${recording ? ' is-recording' : ''}`}
    aria-pressed={dictation.active}
    aria-label={label}
    title={label}
    disabled={disabled || dictation.phase === 'finishing'}
    onClick={() => void dictation.toggle()}
  >
    <Icon size={recording ? 12 : compact ? 16 : 18} aria-hidden="true"/>
    {recording && 'Parar'}
  </button>;
}
