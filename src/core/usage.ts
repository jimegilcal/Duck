import { invoke } from '@tauri-apps/api/core';
import { useEffect, useState } from 'react';

export type UsageProvider = 'claude-code' | 'codex';
export type UsageRow = {
  date: string; provider: UsageProvider; project: string; model: string;
  input: number; output: number; cache: number; activeMinutes: number;
};
export type UsageReport = { rows: UsageRow[]; generatedAt: number; files: number; windowDays: number };
export type Period = 'today' | 'week' | 'month';

export const usageProviders: { id: UsageProvider; name: string }[] = [
  { id: 'claude-code', name: 'Claude Code' },
  { id: 'codex', name: 'Codex' },
];
export const periodLabels: Record<Period, string> = { today: 'Hoy', week: '7 días', month: '30 días' };

const pad = (n: number) => String(n).padStart(2, '0');
export const localDate = (date: Date) => `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`;
function daysBack(count: number, now: Date) {
  return Array.from({ length: count }, (_, i) => {
    const day = new Date(now.getFullYear(), now.getMonth(), now.getDate() - (count - 1 - i));
    return localDate(day);
  });
}
const lengths: Record<Period, number> = { today: 1, week: 7, month: 30 };

export type Totals = { input: number; output: number; cache: number; tokens: number; minutes: number };
const empty = (): Totals => ({ input: 0, output: 0, cache: 0, tokens: 0, minutes: 0 });
function add(total: Totals, row: UsageRow) {
  total.input += row.input; total.output += row.output; total.cache += row.cache;
  total.tokens += row.input + row.output; // the headline figure leaves cache reads out
  total.minutes += row.activeMinutes;
}

export function summarize(rows: UsageRow[], period: Period, now = new Date()) {
  const days = new Set(daysBack(lengths[period], now));
  const byProvider: Record<UsageProvider, Totals> = { 'claude-code': empty(), codex: empty() };
  const projects = new Map<string, { project: string; provider: UsageProvider; tokens: number; minutes: number }>();
  for (const row of rows) {
    if (!days.has(row.date) || !(row.provider in byProvider)) continue;
    add(byProvider[row.provider], row);
    const key = `${row.provider}:${row.project}`;
    const entry = projects.get(key) ?? { project: row.project, provider: row.provider, tokens: 0, minutes: 0 };
    entry.tokens += row.input + row.output; entry.minutes += row.activeMinutes;
    projects.set(key, entry);
  }
  const models = new Map<string, { model: string; provider: UsageProvider; tokens: number; cache: number }>();
  for (const row of rows) {
    if (!days.has(row.date) || !row.model || !(row.provider in byProvider)) continue;
    const key = `${row.provider}:${row.model}`;
    const entry = models.get(key) ?? { model: row.model, provider: row.provider, tokens: 0, cache: 0 };
    entry.tokens += row.input + row.output; entry.cache += row.cache;
    models.set(key, entry);
  }
  const byModel = [...models.values()].filter(m => m.tokens > 0).sort((a, b) => b.tokens - a.tokens);
  const topProjects = [...projects.values()].filter(p => p.tokens + p.minutes > 0)
    .sort((a, b) => b.tokens - a.tokens || b.minutes - a.minutes).slice(0, 5);
  return { byProvider, topProjects, byModel };
}

/** One entry per day for the last `count` days, split by tool. */
export function dailySeries(rows: UsageRow[], count: number, now = new Date()) {
  const series = daysBack(count, now).map(date => ({
    date, 'claude-code': empty(), codex: empty(),
  }));
  const index = new Map(series.map(day => [day.date, day]));
  for (const row of rows) {
    const day = index.get(row.date);
    if (day && row.provider in day) add(day[row.provider], row);
  }
  return series;
}

export function formatTokens(value: number) {
  if (value >= 1_000_000) return `${(value / 1_000_000).toLocaleString('es-ES', { maximumFractionDigits: 1 })} M`;
  if (value >= 10_000) return `${Math.round(value / 1000).toLocaleString('es-ES')} k`;
  if (value >= 1000) return `${(value / 1000).toLocaleString('es-ES', { maximumFractionDigits: 1 })} k`;
  return String(value);
}
export function formatMinutes(minutes: number) {
  if (minutes < 60) return `${minutes} min`;
  const hours = Math.floor(minutes / 60);
  const rest = minutes % 60;
  return rest ? `${hours} h ${rest} min` : `${hours} h`;
}

export type UsageState =
  | { status: 'off'; report: null }
  | { status: 'loading'; report: null }
  | { status: 'error'; report: null }
  | { status: 'ready'; report: UsageReport };

/** Loads the report. Reads nothing unless the user switched usage tracking on in Settings. */
export function useUsage(enabled: boolean, refreshMs = 60_000): UsageState {
  const [state, setState] = useState<UsageState>({ status: enabled ? 'loading' : 'off', report: null });
  useEffect(() => {
    if (!enabled) { setState({ status: 'off', report: null }); return; }
    let active = true;
    async function load() {
      try {
        const report = await invoke<UsageReport>('usage_report');
        if (active) setState({ status: 'ready', report });
      } catch (error) {
        if (active) setState(String(error).includes('usage_disabled') ? { status: 'off', report: null } : { status: 'error', report: null });
      }
    }
    void load();
    const timer = window.setInterval(() => { if (!document.hidden) void load(); }, refreshMs);
    return () => { active = false; window.clearInterval(timer); };
  }, [enabled, refreshMs]);
  return state;
}
