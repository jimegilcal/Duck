import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Bot, Check, CircleDashed, LoaderCircle, Users } from 'lucide-react';
import { agentProviders } from '../core/adapters';
import { compactAgentSessions, effectiveAgentStatus, type AgentStatus } from '../core/agentStatus';
import { formatMinutes, formatTokens, summarize, useUsage } from '../core/usage';

type AgentId = (typeof agentProviders)[number]['id'];
type Todo = { title: string; status: 'pending' | 'in_progress' | 'completed'; active: string };
type SubAgent = { id: string; title: string; kind: string; status: 'running' | 'done'; started: number; ended: number | null };
type Step = { t: number; text: string };
type Session = {
  provider: AgentId; status: AgentStatus; project: string; timestamp: number; session: string;
  title: string; goal: string; todos: Todo[]; agents: SubAgent[]; timeline: Step[];
};

const labels: Record<AgentStatus | 'idle', string> = {
  ready: 'Sesión abierta', working: 'Trabajando', approval: 'Esperando tu aprobación',
  finished: 'Terminó', interrupted: 'Interrumpido', closed: 'Sesión cerrada', idle: 'Sin actividad reciente',
};

function useAgentActivity(refreshMs: number) {
  const [activity, setActivity] = useState({ sessions: [] as Session[], now: Date.now(), unavailable: false });
  useEffect(() => {
    let active = true;
    let timer: number;
    async function update() {
      try {
        if (!document.hidden) {
          const sessions = await invoke<Session[]>('agent_updates');
          if (active) setActivity({ sessions, now: Date.now(), unavailable: false });
        }
      } catch {
        if (active) setActivity({ sessions: [], now: Date.now(), unavailable: true });
      } finally {
        if (active) timer = window.setTimeout(update, refreshMs);
      }
    }
    void update();
    return () => {
      active = false;
      window.clearTimeout(timer);
    };
  }, [refreshMs]);
  return activity;
}

function ago(timestamp: number, now: number) {
  const seconds = Math.max(0, Math.round((now - timestamp) / 1000));
  if (seconds < 10) return 'ahora';
  if (seconds < 60) return `hace ${seconds} s`;
  if (seconds < 3600) return `hace ${Math.round(seconds / 60)} min`;
  return `hace ${Math.round(seconds / 3600)} h`;
}
function progress(session: Session) {
  const done = session.todos.filter(todo => todo.status === 'completed').length;
  const running = session.agents.filter(agent => agent.status === 'running').length;
  return { done, total: session.todos.length, running, launched: session.agents.length };
}
function summary(session: Session) {
  const { done, total, running, launched } = progress(session);
  return [
    total ? `${done}/${total} pasos` : null,
    launched ? `${launched} ${launched === 1 ? 'agente' : 'agentes'}${running ? ` · ${running} activos` : ''}` : null,
  ].filter(Boolean).join(' · ');
}

function StepIcon({ status }: { status: Todo['status'] }) {
  if (status === 'completed') return <Check size={13} className="step-done" aria-label="Hecho"/>;
  if (status === 'in_progress') return <LoaderCircle size={13} className="step-active" aria-label="En curso"/>;
  return <CircleDashed size={13} className="step-pending" aria-label="Pendiente"/>;
}

function Detail({ session, now }: { session: Session; now: number }) {
  const { done, total } = progress(session);
  const state = effectiveAgentStatus(session, now);
  const current = session.todos.find(todo => todo.status === 'in_progress');
  return <article className={`agent-session state-${state}`}>
    <header>
      <div><strong>{session.title || session.project}</strong><span>{session.title && `${session.project} · `}{agentProviders.find(p => p.id === session.provider)?.name} · {ago(session.timestamp, now)}</span></div>
      <span className="agent-badge">{labels[state]}</span>
    </header>
    {session.goal && <p className="agent-goal">«{session.goal}»</p>}
    {total > 0 && <>
      <div className="agent-progress" role="progressbar" aria-valuemin={0} aria-valuemax={total} aria-valuenow={done} aria-label="Progreso del plan">
        <span style={{ width: `${(100 * done) / total}%` }}/>
      </div>
      <p className="agent-now">{current ? `Ahora: ${current.active || current.title}` : `${done} de ${total} pasos hechos`}</p>
      <ol className="agent-steps">{session.todos.map((todo, index) =>
        <li key={index} className={todo.status}><StepIcon status={todo.status}/><span>{todo.title}</span></li>)}
      </ol>
    </>}
    {session.agents.length > 0 && <>
      <h4><Users size={13}/> Agentes lanzados · {session.agents.length}</h4>
      <ul className="agent-subagents">{session.agents.map(agent =>
        <li key={agent.id} className={agent.status}>
          <StepIcon status={agent.status === 'done' ? 'completed' : 'in_progress'}/>
          <span>{agent.title}{agent.kind && <small> · {agent.kind}</small>}</span>
          <small>{agent.status === 'done' ? 'terminado' : 'en curso'}</small>
        </li>)}
      </ul>
    </>}
    {session.timeline.length > 1 && <>
      <h4>Cronología</h4>
      <ul className="agent-timeline">{[...session.timeline].reverse().map((step, index) =>
        <li key={index}><time>{new Date(step.t).toLocaleTimeString('es-ES', { hour: '2-digit', minute: '2-digit', second: '2-digit' })}</time><span>{step.text}</span></li>)}
      </ul>
    </>}
  </article>;
}

export function AgentRoster({ compact = false }: { compact?: boolean }) {
  const { sessions, now, unavailable } = useAgentActivity(compact ? 1000 : 3000);
  const [running, setRunning] = useState<AgentId[] | null>(null);
  const [tracking, setTracking] = useState(false);
  const usage = useUsage(tracking);

  useEffect(() => {
    let active = true;
    async function update() {
      try {
        const [agents, settings] = await Promise.all([
          invoke<AgentId[]>('running_agents'),
          invoke<{ usageTracking?: boolean }>('get_settings'),
        ]);
        if (active) {
          setRunning(agents);
          setTracking(!!settings.usageTracking);
        }
      } catch {
        if (active) setRunning(null);
      }
    }
    void update();
    const interval = window.setInterval(() => {
      if (!document.hidden) void update();
    }, 3000);
    return () => {
      active = false;
      window.clearInterval(interval);
    };
  }, []);

  if (unavailable) return <p className="agent-empty">Estado no disponible.</p>;

  return <div className={`agent-roster${compact ? ' compact' : ' detailed'}`} aria-label="Claude Code y Codex">
    {agentProviders.map(provider => {
      const own = sessions.filter(session => session.provider === provider.id);
      const open = !!running?.includes(provider.id);
      const active = own.filter(session => ['working', 'approval'].includes(effectiveAgentStatus(session, now)));
      const compactSessions = compact ? compactAgentSessions(own, now) : [];
      const presence = open ? 'Abierto' : running ? 'Sin sesiones' : 'Comprobando…';
      const activeLabel = `${active.length} ${active.length === 1 ? 'hilo activo' : 'hilos activos'}`;
      return <section key={provider.id} className="agent-provider">
        <h3><Bot size={14} aria-hidden="true"/>{provider.name}
          <span className={active.length || open ? 'agent-detected' : undefined}>{active.length ? activeLabel : presence}</span>
        </h3>
        {usage.status === 'ready' && (() => {
          const today = summarize(usage.report.rows, 'today').byProvider[provider.id];
          const week = summarize(usage.report.rows, 'week').byProvider[provider.id];
          return <p className="agent-usage" title="Tokens = entrada + salida"><strong>Hoy</strong> {formatTokens(today.tokens)} tokens · {formatMinutes(today.minutes)}
            {!compact && <> <span>· 7 días {formatTokens(week.tokens)} tokens · {formatMinutes(week.minutes)}</span></>}</p>;
        })()}
        {own.length === 0 && <p className="agent-empty">
          {provider.id === 'codex'
            ? 'El estado aparece aquí cuando Duck detecta un turno de Codex.'
            : 'Aún no hay avisos. Inicia una sesión nueva de Claude Code con los hooks de Duck.'}
        </p>}
        {compact
          ? compactSessions.map(session => {
              const state = effectiveAgentStatus(session, now);
              const details = [labels[state], session.title ? session.project : null, summary(session), ago(session.timestamp, now)]
                .filter(Boolean).join(' · ');
              return <div key={session.session || session.timestamp} className={`agent-line state-${state}`}>
                <strong>{session.title || session.project}</strong>
                <span>{state === 'working' && <LoaderCircle size={10} className="agent-status-spinner" aria-hidden="true"/>}{details}</span>
              </div>;
            })
          : own.map(session => <Detail key={session.session || session.timestamp} session={session} now={now}/>)}
      </section>;
    })}
  </div>;
}
