import { useEffect, useState } from 'react'
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
const DIMENSIONS = ['day', 'model', 'project', 'tool']

const DAY_MS = 86400000

/** Query string for the current period (`since`), or for the previous equal period (`since` and `until`). */
function query(days, workspace, account, previous = false) {
  const parts = []
  const now = Date.now()
  if (days !== null) {
    const back = previous ? 2 * days : days
    parts.push(`since=${encodeURIComponent(new Date(now - back * DAY_MS).toISOString())}`)
    if (previous) parts.push(`until=${encodeURIComponent(new Date(now - days * DAY_MS).toISOString())}`)
  }
  if (workspace) parts.push(`workspace=${encodeURIComponent(workspace)}`)
  if (account) parts.push(`account=${encodeURIComponent(account)}`)
  return parts.join('&')
}

async function load(days, workspace, account) {
  const q = query(days, workspace, account)
  const url = (path, extra = '') => `${path}?${[extra, q].filter(Boolean).join('&')}`
  const [previous, summary, outliers, sessions, ...breakdowns] = await Promise.all([
    loadPrevious(days, workspace, account),
    getJson(url('/api/usage/summary')),
    getJson(url('/api/usage/outliers')),
    getJson(url('/api/usage/sessions')),
    ...DIMENSIONS.map((by) => getJson(url('/api/usage/breakdown', `by=${by}`))),
  ])
  return { previous, summary, outliers, sessions, breakdowns: Object.fromEntries(DIMENSIONS.map((d, i) => [d, breakdowns[i]])) }
}

/** Totals of the previous equal period, or null (All time, or the request failed: badges say "no previous data"). */
async function loadPrevious(days, workspace, account) {
  if (days === null) return null
  try {
    return (await getJson(`/api/usage/breakdown?by=day&${query(days, workspace, account, true)}`)).total
  } catch {
    return null
  }
}

const sumTokens = (row) => (row ? totalTokens(row) : null)
const usdOf = (cost) => (cost && cost.usd !== null && cost.abstained_models.length === 0 ? cost.usd : null)

function Kpi({ name, value, note, current, previous, spark, sparkLabel }) {
  const id = `kpi-${name.replace(/\W+/g, '-').toLowerCase()}`
  return (
    <section className="kpi" aria-labelledby={id}>
      <h2 id={id} className="kpi-name">{name}</h2>
      <div className="kpi-head">
        {typeof value === 'number' ? <Compact n={value} className="kpi-value" /> : <span className="kpi-value">{value}</span>}
        <Trend current={current} previous={previous} />
      </div>
      {note && <span className="kpi-note">{note}</span>}
      <Sparkline values={spark} label={sparkLabel} />
    </section>
  )
}

function Widgets({ summary, day, previous, period }) {
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
          spark={tokens}
          sparkLabel={perDay('Total tokens', tokens, fmt)}
        />
        <Kpi
          name="Messages"
          value={summary.messages}
          current={summary.messages}
          previous={previous ? previous.messages : null}
          spark={messages}
          sparkLabel={perDay('Messages', messages, fmt)}
        />
        <Kpi
          name="API-equivalent cost"
          value={costFigure(cost)}
          note={abstainedText(cost)}
          current={usdOf(cost)}
          previous={usdOf(previous?.api_equivalent_cost)}
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
  const [state, setState] = useState({ status: 'loading' })

  useEffect(() => {
    let current = true
    setState({ status: 'loading' })
    load(PERIODS.find((p) => p.id === period).days, workspace, account).then(
      (data) => current && setState({ status: 'ready', data }),
      (error) => current && setState({ status: 'error', message: error.message }),
    )
    return () => {
      current = false
    }
  }, [period, workspace, account])

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
      {state.status === 'loading' && <Loading label="Loading usage" />}
      {state.status === 'error' && (
        <p role="alert" className="error">
          Could not load usage: {state.message}
        </p>
      )}
      {state.status === 'ready' && (
        <>
          <Widgets summary={state.data.summary} day={state.data.breakdowns.day} previous={state.data.previous} period={chosen} />
          <div className="grid-2">
            <Breakdown by="day" data={state.data.breakdowns.day} />
            <Breakdown by="model" data={state.data.breakdowns.model} />
          </div>
          <WorkSplit day={state.data.breakdowns.day} />
          <div className="grid-2">
            <Breakdown by="project" data={state.data.breakdowns.project} />
            <Outliers data={state.data.outliers} />
          </div>
          <Totals summary={state.data.summary} total={state.data.breakdowns.day.total} />
          <Breakdown by="tool" data={state.data.breakdowns.tool} />
          <Timeline data={state.data.sessions} />
        </>
      )}
    </main>
  )
}
