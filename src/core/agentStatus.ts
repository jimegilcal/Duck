export type AgentStatus = 'ready' | 'working' | 'approval' | 'finished' | 'interrupted' | 'closed';

export const AGENT_STALE_MS = 10 * 60_000;

export function effectiveAgentStatus(
  session: { status: AgentStatus; timestamp: number },
  now: number,
): AgentStatus | 'idle' {
  const canBecomeStale = session.status === 'working' || session.status === 'approval';
  return canBecomeStale && now - session.timestamp > AGENT_STALE_MS ? 'idle' : session.status;
}

export function compactAgentSessions<T extends { status: AgentStatus; timestamp: number }>(
  sessions: T[],
  now: number,
): T[] {
  const newestFirst = [...sessions].sort((left, right) => right.timestamp - left.timestamp);
  const active = newestFirst.filter(session => ['working', 'approval'].includes(effectiveAgentStatus(session, now)));
  return active.length ? active : newestFirst.slice(0, 1);
}
