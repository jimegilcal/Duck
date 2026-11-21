import { describe, expect, it } from 'vitest';
import { compactAgentSessions, effectiveAgentStatus } from './agentStatus';

const now = 1_700_000_000_000;

describe('agent status', () => {
  it('expires an approval that stopped receiving updates', () => {
    expect(effectiveAgentStatus({ status: 'approval', timestamp: now - 11 * 60_000 }, now)).toBe('idle');
  });

  it('lists every active thread with the newest first', () => {
    const sessions = [
      { id: 'older-approval', status: 'approval' as const, timestamp: now - 60_000 },
      { id: 'current-work', status: 'working' as const, timestamp: now - 5_000 },
    ];
    expect(compactAgentSessions(sessions, now).map(session => session.id)).toEqual([
      'current-work',
      'older-approval',
    ]);
  });

  it('falls back to only the newest thread when none are active', () => {
    const sessions = [
      { id: 'old', status: 'finished' as const, timestamp: now - 20_000 },
      { id: 'new', status: 'finished' as const, timestamp: now - 1_000 },
    ];
    expect(compactAgentSessions(sessions, now).map(session => session.id)).toEqual(['new']);
  });
});
