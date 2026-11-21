import { describe, expect, it } from 'vitest';
import { dailySeries, formatMinutes, formatTokens, localDate, summarize, type UsageRow } from './usage';

const now = new Date(2026, 10, 20, 12, 0, 0);
const day = (back: number) => localDate(new Date(2026, 10, 20 - back));
const row = (back: number, provider: UsageRow['provider'], extra: Partial<UsageRow> = {}): UsageRow =>
  ({ date: day(back), provider, project: 'Duck', model: 'm1', input: 100, output: 20, cache: 5000, activeMinutes: 30, ...extra });

describe('usage summary', () => {
  it('counts input + output as the headline and keeps cache apart', () => {
    const { byProvider } = summarize([row(0, 'claude-code')], 'today', now);
    expect(byProvider['claude-code']).toMatchObject({ tokens: 120, cache: 5000, minutes: 30 });
    expect(byProvider.codex.tokens).toBe(0);
  });
  it('limits each period to its own days', () => {
    const rows = [row(0, 'codex'), row(3, 'codex'), row(20, 'codex')];
    expect(summarize(rows, 'today', now).byProvider.codex.tokens).toBe(120);
    expect(summarize(rows, 'week', now).byProvider.codex.tokens).toBe(240);
    expect(summarize(rows, 'month', now).byProvider.codex.tokens).toBe(360);
  });
  it('ranks projects by tokens', () => {
    const rows = [row(0, 'codex', { project: 'A', input: 10 }), row(0, 'claude-code', { project: 'B', input: 900 })];
    expect(summarize(rows, 'today', now).topProjects.map(p => p.project)).toEqual(['B', 'A']);
  });
  it('splits tokens by model and ignores the rows that only carry time', () => {
    const rows = [row(0, 'claude-code', { model: 'opus' }), row(0, 'claude-code', { model: 'sonnet', input: 10, output: 5 }), row(0, 'claude-code', { model: '', input: 0, output: 0, cache: 0 })];
    const { byModel } = summarize(rows, 'today', now);
    expect(byModel.map(m => [m.model, m.tokens])).toEqual([['opus', 120], ['sonnet', 15]]);
  });
  it('builds one entry per day, oldest first, split by tool', () => {
    const series = dailySeries([row(0, 'codex'), row(1, 'claude-code')], 7, now);
    expect(series).toHaveLength(7);
    expect(series[6].date).toBe(day(0));
    expect(series[6].codex.tokens).toBe(120);
    expect(series[5]['claude-code'].tokens).toBe(120);
  });
});

describe('formatting', () => {
  it('shortens tokens and durations', () => {
    expect(formatTokens(950)).toBe('950');
    expect(formatTokens(12_400)).toMatch(/^12\s?k$/);
    expect(formatTokens(2_300_000)).toMatch(/^2,3 M$/);
    expect(formatMinutes(45)).toBe('45 min');
    expect(formatMinutes(125)).toBe('2 h 5 min');
    expect(formatMinutes(120)).toBe('2 h');
  });
});
