import { useState } from 'react';
import { Clock, Cpu, Table2 } from 'lucide-react';
import {
  dailySeries, formatMinutes, formatTokens, periodLabels, summarize, useUsage, usageProviders,
  type Period, type UsageProvider,
} from '../core/usage';

const names: Record<UsageProvider, string> = { 'claude-code': 'Claude Code', codex: 'Codex' };
const weekday = (date: string) => new Date(date + 'T12:00:00').toLocaleDateString('es-ES', { weekday: 'short' }).replace('.', '');
const longDate = (date: string) => new Date(date + 'T12:00:00').toLocaleDateString('es-ES', { weekday: 'long', day: 'numeric', month: 'long' });

export function UsageCard({ enabled, busy, onEnable }: { enabled: boolean; busy: boolean; onEnable: () => void }) {
  const usage = useUsage(enabled);
  const [period, setPeriod] = useState<Period>('week');
  const [table, setTable] = useState(false);
  const [hover, setHover] = useState<number | null>(null);

  if (!enabled || usage.status === 'off') {
    return <section className="usage-card" aria-label="Uso de IA">
      <header><h2><Cpu size={16}/> Uso de IA</h2></header>
      <p className="usage-note">Consulta los tokens y el tiempo activo de Claude Code y Codex. Duck usa las cifras y las marcas de actividad que guardan en este Mac, sin extraer el texto de las conversaciones.</p>
      <button className="usage-enable" disabled={busy} onClick={onEnable}>Activar uso de IA</button>
    </section>;
  }
  if (usage.status === 'loading') return <section className="usage-card" aria-label="Uso de IA"><header><h2><Cpu size={16}/> Uso de IA</h2></header><p className="usage-note">Leyendo los historiales… la primera vez puede tardar unos segundos.</p></section>;
  if (usage.status === 'error') return <section className="usage-card" aria-label="Uso de IA"><header><h2><Cpu size={16}/> Uso de IA</h2></header><p className="usage-note">No se pudieron leer los historiales.</p></section>;

  const rows = usage.report.rows;
  const { byProvider, topProjects, byModel } = summarize(rows, period);
  const count = period === 'month' ? 30 : 7;
  const series = dailySeries(rows, count);
  const max = Math.max(1, ...series.map(day => day['claude-code'].tokens + day.codex.tokens));
  const anyData = rows.length > 0;
  const hovered = hover === null ? null : series[hover];

  return <section className="usage-card" aria-label="Uso de IA">
    <header>
      <h2><Cpu size={16}/> Uso de IA</h2>
      <div className="usage-periods" role="group" aria-label="Periodo">
        {(Object.keys(periodLabels) as Period[]).map(key =>
          <button key={key} className={period === key ? 'active' : ''} aria-pressed={period === key} onClick={() => setPeriod(key)}>{periodLabels[key]}</button>)}
      </div>
    </header>

    <div className="usage-tiles">
      {usageProviders.map(provider => {
        const total = byProvider[provider.id];
        return <div className="usage-tile" key={provider.id}>
          <span className={`usage-key ${provider.id}`}>{names[provider.id]}</span>
          <strong>{formatTokens(total.tokens)}<small> tokens</small></strong>
          <span className="usage-time"><Clock size={12}/> {formatMinutes(total.minutes)}</span>
          <span className="usage-split">Entrada {formatTokens(total.input)} · Salida {formatTokens(total.output)}<br/>Caché {formatTokens(total.cache)}</span>
        </div>;
      })}
    </div>

    {!anyData ? <p className="usage-note">Todavía no hay actividad en los últimos {usage.report.windowDays} días.</p> : <>
      <div className="usage-chart-head">
        <span>{period === 'month' ? 'Últimos 30 días' : 'Últimos 7 días'} · tokens por día</span>
        <button className="usage-table-toggle" aria-pressed={table} onClick={() => setTable(!table)}><Table2 size={12}/> {table ? 'Ver gráfica' : 'Ver tabla'}</button>
      </div>
      {table ? <table className="usage-table">
        <thead><tr><th>Día</th><th>Claude Code</th><th>Codex</th></tr></thead>
        <tbody>{[...series].reverse().map(day => <tr key={day.date}>
          <th scope="row">{longDate(day.date)}</th>
          <td>{formatTokens(day['claude-code'].tokens)} · {formatMinutes(day['claude-code'].minutes)}</td>
          <td>{formatTokens(day.codex.tokens)} · {formatMinutes(day.codex.minutes)}</td>
        </tr>)}</tbody>
      </table> : <div className="usage-chart" role="img" aria-label="Tokens por día de Claude Code y Codex" onMouseLeave={() => setHover(null)}>
        {series.map((day, index) => {
          const claude = day['claude-code'].tokens, codex = day.codex.tokens;
          return <div className="usage-day" key={day.date} tabIndex={0} aria-label={`${longDate(day.date)}: Claude Code ${formatTokens(claude)} tokens, Codex ${formatTokens(codex)} tokens`}
            onMouseEnter={() => setHover(index)} onFocus={() => setHover(index)} onBlur={() => setHover(null)}>
            <div className="usage-stack">
              {codex > 0 && <span className="seg codex" style={{ height: `${Math.max(3, (100 * codex) / max)}%` }}/>}
              {claude > 0 && <span className="seg claude-code" style={{ height: `${Math.max(3, (100 * claude) / max)}%` }}/>}
            </div>
            <span className="usage-label">{count === 7 || index % 5 === 0 ? (count === 7 ? weekday(day.date) : day.date.slice(8)) : ''}</span>
          </div>;
        })}
        {hovered && <div className="usage-tooltip" role="status">
          <span>{longDate(hovered.date)}</span>
          {usageProviders.map(provider => <div key={provider.id}>
            <i className={`usage-swatch ${provider.id}`}/><strong>{formatTokens(hovered[provider.id].tokens)}</strong> {names[provider.id]} · {formatMinutes(hovered[provider.id].minutes)}
          </div>)}
        </div>}
      </div>}
      <div className="usage-legend">{usageProviders.map(provider => <span key={provider.id}><i className={`usage-swatch ${provider.id}`}/>{names[provider.id]}</span>)}</div>

      {byModel.length > 0 && <>
        <h3>Por modelo · {periodLabels[period].toLowerCase()}</h3>
        <ul className="usage-models">{byModel.map(item => {
          const total = byProvider[item.provider].tokens || 1;
          return <li key={item.provider + item.model}>
            <i className={`usage-swatch ${item.provider}`}/><span>{item.model}<small>{names[item.provider]}</small></span>
            <strong>{formatTokens(item.tokens)}</strong>
            <span className="usage-share" aria-hidden="true"><b className={item.provider} style={{ width: `${Math.max(2, (100 * item.tokens) / total)}%` }}/></span>
            <small>{Math.round((100 * item.tokens) / total)} %</small>
          </li>;
        })}</ul>
      </>}

      {topProjects.length > 0 && <>
        <h3>Proyectos con más uso · {periodLabels[period].toLowerCase()}</h3>
        <ul className="usage-projects">{topProjects.map(project => <li key={project.provider + project.project}>
          <i className={`usage-swatch ${project.provider}`}/><span>{project.project}</span>
          <strong>{formatTokens(project.tokens)}</strong><small>{formatMinutes(project.minutes)}</small>
        </li>)}</ul>
      </>}
    </>}
    <p className="usage-foot">Tokens = entrada + salida; la caché se muestra aparte. Las horas son tiempo con actividad (las pausas de más de 5 minutos no cuentan) y no se pueden repartir por modelo. El porcentaje es la parte de cada herramienta. Solo se ve lo que ocurre en este Mac.</p>
  </section>;
}
