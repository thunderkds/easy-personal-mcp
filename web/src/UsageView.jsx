import { useEffect, useMemo, useState } from 'react'
import { getJson } from './api.js'
import Breakdown from './Breakdown.jsx'
import Cost from './Cost.jsx'
import { NO_SCOPE } from './Scope.jsx'
import Timeline from './Timeline.jsx'
import { abstainedText, costFigure, fmt, percent, totalTokens } from './format.js'
import { Compact, Progress, Sparkline, Trend, useTopN } from './Widgets.jsx'
import Loading from './Loading.jsx'

export const PERIODS = [
  { id: '7', label: 'Last 7 days', days: 7 },
  { id: '30', label: 'Last 30 days', days: 30 },
  { id: 'all', label: 'All time', days: null },
]

const DAY_MS = 86400000
const MINUTE_MS = 60000

/**
 * `since` for a window of `days` ending now, rounded down to the whole minute (ISO, UTC), so every request of
 * one Usage load and the side rail's Cache reuse card ask the same question with the same URL (NFR-013).
 */
export function sinceOf(days, now = Date.now()) {
  return new Date(Math.floor((now - days * DAY_MS) / MINUTE_MS) * MINUTE_MS).toISOString()
}

/** The URL of every read the Usage page makes for one period + scope; `previous` is null for All time. */
function usageUrls(days, workspace, account) {
  const scope = [workspace && `workspace=${encodeURIComponent(workspace)}`, account && `account=${encodeURIComponent(account)}`]
  const url = (path, ...parts) => `${path}?${[...parts, ...scope].filter(Boolean).join('&')}`
  const since = days === null ? null : `since=${encodeURIComponent(sinceOf(days))}`
  const previous = days === null ? null : [`since=${encodeURIComponent(sinceOf(2 * days))}`, `until=${encodeURIComponent(sinceOf(days))}`]
  return {
    summary: url('/api/usage/summary', since),
    outliers: url('/api/usage/outliers', since),
    sessions: url('/api/usage/sessions', since),
    day: url('/api/usage/breakdown', 'by=day', since),
    model: url('/api/usage/breakdown', 'by=model', since),
    project: url('/api/usage/breakdown', 'by=project', since),
    tool: url('/api/usage/breakdown', 'by=tool', since),
    previous: previous && url('/api/usage/breakdown', 'by=day', ...previous),
  }
}

/**
 * One read of one URL: {status: 'loading' | 'ready' | 'error' | 'none', data, message}; `null` = no request ('none').
 * The answer is keyed by its URL, so a late answer from an older period or scope is dropped and never shown.
 */
function useGet(url) {
  const [state, setState] = useState({ url: null })
  useEffect(() => {
    if (url === null) return undefined
    let current = true
    getJson(url).then(
      (data) => current && setState({ url, status: 'ready', data }),
      (error) => current && setState({ url, status: 'error', message: error.message }),
    )
    return () => {
      current = false
    }
  }, [url])
  if (url === null) return { status: 'none' }
  return state.url === url ? state : { status: 'loading' }
}

/** One part of the page (no heading until it has data, so a heading on screen means the data is there): its own loading line or error line until every read it needs is there, then `children(...data)`. */
function Part({ label, reads, children }) {
  const failed = reads.find((r) => r.status === 'error')
  if (failed) {
    return (
      <section className="card">
        <p role="alert" className="error">
          Could not load {label}: {failed.message}
        </p>
      </section>
    )
  }
  if (reads.some((r) => r.status !== 'ready')) {
    return (
      <section className="card" aria-busy="true">
        <p role="status" className="part-loading muted">
          <span className="spinner" aria-hidden="true" />
          Loading {label}
        </p>
      </section>
    )
  }
  return children(...reads.map((r) => r.data))
}

const sumTokens = (row) => (row ? totalTokens(row) : null)
const usdOf = (cost) => (cost && cost.usd !== null && cost.abstained_models.length === 0 ? cost.usd : null)

function Kpi({ name, value, note, current, previous, waiting, spark, sparkLabel }) {
  const id = `kpi-${name.replace(/\W+/g, '-').toLowerCase()}`
  return (
    <section className="kpi" aria-labelledby={id}>
      <h2 id={id} className="kpi-name">{name}</h2>
      <div className="kpi-head">
        {typeof value === 'number' ? <Compact n={value} className="kpi-value" /> : <span className="kpi-value">{value}</span>}
        {waiting ? <span className="trend trend-none">loading previous period</span> : <Trend current={current} previous={previous} />}
      </div>
      {note && <span className="kpi-note">{note}</span>}
      <Sparkline values={spark} label={sparkLabel} />
    </section>
  )
}

/** `previousRead` is the previous-period read: its badges say "loading" until it ends, "no previous data" if it failed or there is none. */
function Widgets({ summary, day, previousRead, period }) {
  const previous = previousRead.status === 'ready' ? previousRead.data.total : null
  const waiting = previousRead.status === 'loading'
  const groups = day.groups
  const peakOf = (values, show) => show(Math.max(...values))
  const perDay = (name, values, show) =>
    `${name} per day, ${values.length} days, peak ${peakOf(values, show)}`
  const tokens = groups.map(totalTokens)
  const messages = groups.map((g) => g.messages)
  const usd = groups.map((g) => g.api_equivalent_cost.usd ?? 0)
  const abstained = [...new Set(groups.flatMap((g) => g.api_equivalent_cost.abstained_models))]
  const cost = day.total.api_equivalent_cost
  const ratio = summary.cache_hit_ratio
  return (
    <>
      <div className="kpi-strip" data-testid="kpi-strip">
        <Kpi
          name="Total tokens"
          value={totalTokens(summary)}
          current={totalTokens(summary)}
          previous={sumTokens(previous)}
          waiting={waiting}
          spark={tokens}
          sparkLabel={perDay('Total tokens', tokens, fmt)}
        />
        <Kpi
          name="Messages"
          value={summary.messages}
          current={summary.messages}
          previous={previous ? previous.messages : null}
          waiting={waiting}
          spark={messages}
          sparkLabel={perDay('Messages', messages, fmt)}
        />
        <Kpi
          name="API-equivalent cost"
          value={costFigure(cost)}
          note={abstainedText(cost)}
          current={usdOf(cost)}
          previous={usdOf(previous?.api_equivalent_cost)}
          waiting={waiting}
          spark={usd}
          sparkLabel={`${perDay('API-equivalent cost', usd, (n) => `$${n.toFixed(2)}`)}${abstained.length ? `, abstained: ${abstained.join(', ')}` : ''}`}
        />
        <section className="kpi kpi-cache" aria-labelledby="h-cache-ratio">
          <h2 id="h-cache-ratio" className="kpi-name">Cache hit ratio</h2>
          {ratio !== null && <span className="kpi-value">{percent(ratio)}</span>}
          <Progress ratio={ratio} label="Cache hit ratio" />
          <span className="kpi-note">cache read {fmt(summary.cache_read_input_tokens)}</span>
        </section>
      </div>
      <p className="footnote" data-testid="cost-footnote">
        API-equivalent cost is tokens times the API price table (price table {day.total.api_equivalent_cost.price_table_date}).
        It is not what a subscription pays.
        {period.days !== null && ` Change is against the previous ${period.days} days.`}
      </p>
    </>
  )
}

function Totals({ summary, total }) {
  const rows = [
    ['Total tokens', fmt(totalTokens(summary))],
    ['Input tokens', fmt(summary.input_tokens)],
    ['Output tokens', fmt(summary.output_tokens)],
    ['Cache write tokens', fmt(summary.cache_creation_input_tokens)],
    ['Cache read tokens', fmt(summary.cache_read_input_tokens)],
    ['Cache hit ratio', percent(summary.cache_hit_ratio)],
    ['Sessions', fmt(summary.sessions)],
    ['Messages', fmt(summary.messages)],
  ]
  return (
    <section className="card" aria-labelledby="h-totals">
      <h2 id="h-totals">Totals</h2>
      <dl className="totals">
        {rows.map(([name, value]) => (
          <div key={name}>
            <dt>{name}</dt>
            <dd>{value}</dd>
          </div>
        ))}
        <div>
          <dt>API-equivalent cost</dt>
          <dd>
            <Cost cost={total.api_equivalent_cost} />
          </dd>
        </div>
      </dl>
      {summary.abstained.total > 0 && (
        <p className="muted">
          {fmt(summary.abstained.total)} records could not be read (abstained) and are not in these numbers.
        </p>
      )}
      {summary.messages === 0 && <p className="muted">No usage recorded in this period.</p>}
    </section>
  )
}

const SPLIT_KINDS = [
  { id: 'typed', label: 'You' },
  { id: 'agent', label: 'Agents' },
  { id: 'unknown', label: 'Unknown' },
]
const DENSE_DAYS = 14

/** You / Agents tokens per day (FR-039): a stacked column per day. Unknown only shows while the period has some. */
function WorkSplit({ day }) {
  const { groups, total } = day
  if (groups.length === 0 || !total.split || groups.some((g) => !g.split)) return null
  const kinds = SPLIT_KINDS.filter((k) => k.id !== 'unknown' || total.split.unknown > 0)
  const max = Math.max(...groups.map(totalTokens), 1)
  const sum = kinds.reduce((n, k) => n + total.split[k.id], 0)
  const dense = groups.length > DENSE_DAYS
  const label = `You and agents tokens per day, ${groups.length} days: ${kinds.map((k) => `${k.label} ${fmt(total.split[k.id])}`).join(', ')}`
  return (
    <section className="card" aria-labelledby="h-split">
      <h2 id="h-split">You and agents</h2>
      <div className={`day-chart${dense ? ' dense' : ''}`} role="img" aria-label={label}>
        <div className="day-bars">
          {groups.map((g) => (
            <div key={g.key} className="split-col" title={`${g.key}: ${kinds.map((k) => `${k.label} ${fmt(g.split[k.id])}`).join(', ')}`}>
              {kinds.map((k) => (
                <div key={k.id} className={`split-seg split-${k.id}`} style={{ height: `${(g.split[k.id] / max) * 100}%` }} />
              ))}
            </div>
          ))}
        </div>
        {!dense && (
          <div className="day-labels">
            {groups.map((g) => (
              <span key={g.key}>{g.key.slice(5)}</span>
            ))}
          </div>
        )}
      </div>
      <ul className="split-legend" aria-label="Legend">
        {kinds.map((k) => (
          <li key={k.id}>
            <span className={`split-swatch split-${k.id}`} aria-hidden="true" />
            {`${k.label} ${fmt(total.split[k.id])} (${percent(sum > 0 ? total.split[k.id] / sum : null)})`}
          </li>
        ))}
      </ul>
      <p className="muted">
        You: sessions you typed in. Agents: sub-agents and sessions started by a program (SDK).
        {total.split.unknown > 0 && ' Unknown: stored before the entrypoint was recorded; the next ingest fills it in. These rows are not yet backfilled.'}
      </p>
      <details className="full-table">
        <summary>Show table</summary>
        <div className="table-wrap">
          <table>
            <thead>
              <tr>
                <th scope="col">Day</th>
                {kinds.map((k) => (
                  <th key={k.id} scope="col">{k.label}</th>
                ))}
              </tr>
            </thead>
            <tbody>
              {groups.map((g) => (
                <tr key={g.key}>
                  <th scope="row">{g.key}</th>
                  {kinds.map((k) => (
                    <td key={k.id}>{fmt(g.split[k.id])}</td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </details>
    </section>
  )
}

function Outliers({ data }) {
  const { shown, toggle } = useTopN(data.sessions)
  return (
    <section className="card" aria-labelledby="h-outliers">
      <div className="card-head">
        <h2 id="h-outliers">Outlier sessions</h2>
        <span className="count" data-testid="outlier-count">{data.sessions.length}</span>
      </div>
      {data.sessions.length === 0 ? (
        <p className="muted">No outlier sessions in this period.</p>
      ) : (
        <ul className="outliers">
          {shown.map((s) => (
            <li key={s.session_id} className="outlier">
              <div className="outlier-top">
                <span className="mono-ellipsis" title={s.project}>{s.project}</span>
                <span className="outlier-side">
                  <span className="outlier-context">{fmt(s.max_context_tokens)}</span> context tokens
                </span>
              </div>
              <span className="outlier-sub">
                <span>{s.session_id}</span> · <span>{fmt(s.tool_calls)}</span> tool calls · {s.reasons.join(', ').replaceAll('_', ' ')}
              </span>
            </li>
          ))}
        </ul>
      )}
      {toggle}
      <p className="footnote">
        Outlier: {data.thresholds.tool_calls} or more tool calls, or a context of {fmt(data.thresholds.context_tokens)} tokens or more.
      </p>
    </section>
  )
}

/** The period is app-wide (HUN-196): App owns it and the right rail reads it too. */
export default function UsageView({ scope = NO_SCOPE, options = null, onOpenAgents = () => {}, period = '7', onPeriodChange = () => {} }) {
  const { workspace, account } = scope
  const days = PERIODS.find((p) => p.id === period).days
  // The URLs are worked out once per period + scope, so `since` stays the same while the answers come in.
  const urls = useMemo(() => usageUrls(days, workspace, account), [days, workspace, account])
  const summary = useGet(urls.summary)
  const day = useGet(urls.day)
  const model = useGet(urls.model)
  const project = useGet(urls.project)
  const tool = useGet(urls.tool)
  const outliers = useGet(urls.outliers)
  const sessions = useGet(urls.sessions)
  const previous = useGet(urls.previous)
  const nothingYet = [summary, day, model, project, tool, outliers, sessions].every((r) => r.status === 'loading')

  const selectedWorkspace = options?.workspaces.find((w) => w.name === workspace)
  const chosen = PERIODS.find((p) => p.id === period)

  return (
    <main className="usage">
      <h1>Usage</h1>
      <p className="notice" role="note">
        <svg width="16" height="16" viewBox="0 0 16 16" aria-hidden="true" focusable="false">
          <circle cx="8" cy="8" r="6.5" fill="none" stroke="currentColor" strokeWidth="1.3" />
          <path d="M8 7v4M8 4.6v.1" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" />
        </svg>
        <span>Not included: cloud sessions and other machines. This view counts only Claude Code sessions stored on this machine.</span>
      </p>
      {options?.ignored > 0 && (
        <div className="notice">
          <p>
            {options.ignored === 1
              ? '1 agent account is ignored, so its usage is not counted.'
              : `${options.ignored} agent accounts are ignored, so their usage is not counted.`}
          </p>
          <button type="button" onClick={onOpenAgents}>
            Open Agents
          </button>
        </div>
      )}
      {selectedWorkspace && selectedWorkspace.repos.length === 0 && (
        <p className="muted">No repos in this workspace, so there is no usage to show. Add repos on the Workspaces page.</p>
      )}
      <div className="periods" role="group" aria-label="Period">
        {PERIODS.map((p) => (
          <button key={p.id} type="button" aria-pressed={p.id === period} onClick={() => onPeriodChange(p.id)}>
            {p.label}
          </button>
        ))}
      </div>
      {nothingYet ? (
        <Loading label="Loading usage" />
      ) : (
        <>
          <Part label="key figures" reads={[summary, day]}>
            {(s, d) => <Widgets summary={s} day={d} previousRead={previous} period={chosen} />}
          </Part>
          <div className="grid-2">
            <Part label="usage by day" reads={[day]}>{(d) => <Breakdown by="day" data={d} />}</Part>
            <Part label="usage by model" reads={[model]}>{(d) => <Breakdown by="model" data={d} />}</Part>
          </div>
          <Part label="you and agents" reads={[day]}>{(d) => <WorkSplit day={d} />}</Part>
          <div className="grid-2">
            <Part label="usage by project" reads={[project]}>{(d) => <Breakdown by="project" data={d} />}</Part>
            <Part label="outlier sessions" reads={[outliers]}>{(d) => <Outliers data={d} />}</Part>
          </div>
          <Part label="totals" reads={[summary, day]}>{(s, d) => <Totals summary={s} total={d.total} />}</Part>
          <Part label="usage by tool" reads={[tool]}>{(d) => <Breakdown by="tool" data={d} />}</Part>
          <Part label="sessions" reads={[sessions]}>{(d) => <Timeline data={d} />}</Part>
        </>
      )}
    </main>
  )
}
