import { useEffect, useState } from 'react'
import { getJson } from './api.js'
import Breakdown from './Breakdown.jsx'
import Cost from './Cost.jsx'
import { NO_SCOPE, ScopeLine } from './Scope.jsx'
import Timeline from './Timeline.jsx'
import { fmt, percent, totalTokens } from './format.js'
import { Progress, Ring, Sparkline, Trend } from './Widgets.jsx'

const PERIODS = [
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

function Kpi({ name, tone, value, current, previous, spark, sparkLabel }) {
  const id = `kpi-${name.replace(/\W+/g, '-').toLowerCase()}`
  return (
    <section className="card kpi" aria-labelledby={id}>
      <div className="kpi-head">
        <span className="kpi-value">{value}</span>
        <Trend current={current} previous={previous} />
      </div>
      <h2 id={id} className="kpi-name">{name}</h2>
      <Sparkline values={spark} label={sparkLabel} tone={tone} />
    </section>
  )
}

function Widgets({ summary, day, previous }) {
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
  const ratioLabel = `Cache hit ratio ${percent(ratio)}`
  return (
    <div className="widgets">
      <Kpi
        name="Total tokens"
        tone="green"
        value={fmt(totalTokens(summary))}
        current={totalTokens(summary)}
        previous={sumTokens(previous)}
        spark={tokens}
        sparkLabel={perDay('Total tokens', tokens, fmt)}
      />
      <Kpi
        name="Messages"
        tone="amber"
        value={fmt(summary.messages)}
        current={summary.messages}
        previous={previous ? previous.messages : null}
        spark={messages}
        sparkLabel={perDay('Messages', messages, fmt)}
      />
      <Kpi
        name="API-equivalent cost"
        tone="cyan"
        value={<Cost cost={cost} />}
        current={usdOf(cost)}
        previous={usdOf(previous?.api_equivalent_cost)}
        spark={usd}
        sparkLabel={`${perDay('API-equivalent cost', usd, (n) => `$${n.toFixed(2)}`)}${abstained.length ? `, abstained: ${abstained.join(', ')}` : ''}`}
      />
      <section className="card cache" aria-labelledby="h-cache-ratio">
        <h2 id="h-cache-ratio" className="kpi-name">Cache hit ratio</h2>
        {ratio !== null && <p className="kpi-value">{percent(ratio)}</p>}
        <Progress ratio={ratio} label="Cache hit ratio" />
        <p className="muted">Cache read tokens: {fmt(summary.cache_read_input_tokens)}</p>
      </section>
      <section className="card gauge" aria-labelledby="h-cache-gauge">
        <h2 id="h-cache-gauge" className="kpi-name">Cache hit gauge</h2>
        <Ring ratio={ratio} label={ratio === null ? 'Cache hit ratio: no data' : ratioLabel} />
      </section>
    </div>
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
    <section aria-labelledby="h-totals">
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
      <p className="muted">
        API-equivalent cost is tokens times the API price table (price table {total.api_equivalent_cost.price_table_date}).
        It is not what a subscription pays.
      </p>
      {summary.abstained.total > 0 && (
        <p className="muted">
          {fmt(summary.abstained.total)} records could not be read (abstained) and are not in these numbers.
        </p>
      )}
      {summary.messages === 0 && <p className="muted">No usage recorded in this period.</p>}
    </section>
  )
}

function Outliers({ data }) {
  return (
    <section aria-labelledby="h-outliers">
      <h2 id="h-outliers">Outlier sessions</h2>
      {data.sessions.length === 0 ? (
        <p className="muted">No outlier sessions in this period.</p>
      ) : (
        <div className="table-wrap">
          <table>
            <thead>
              <tr>
                <th scope="col">Session</th>
                <th scope="col">Project</th>
                <th scope="col">Tool calls</th>
                <th scope="col">Max context tokens</th>
                <th scope="col">Reasons</th>
              </tr>
            </thead>
            <tbody>
              {data.sessions.map((s) => (
                <tr key={s.session_id}>
                  <th scope="row">{s.session_id}</th>
                  <td>{s.project}</td>
                  <td>{fmt(s.tool_calls)}</td>
                  <td>{fmt(s.max_context_tokens)}</td>
                  <td>{s.reasons.join(', ')}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
      <p className="muted">
        Outlier: {data.thresholds.tool_calls} or more tool calls, or a context of {fmt(data.thresholds.context_tokens)} tokens or more.
      </p>
    </section>
  )
}

export default function UsageView({ scope = NO_SCOPE, options = null, onOpenAgents = () => {} }) {
  const { workspace, account } = scope
  const [period, setPeriod] = useState('7')
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

  return (
    <main>
      <h1>Usage</h1>
      <p className="notice" role="note">
        Not included: cloud sessions and other machines. This view counts only Claude Code sessions stored on this machine.
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
      <ScopeLine scope={scope} options={options} />
      {selectedWorkspace && selectedWorkspace.repos.length === 0 && (
        <p className="muted">No repos in this workspace, so there is no usage to show. Add repos on the Workspaces page.</p>
      )}
      <div className="periods" role="group" aria-label="Period">
        {PERIODS.map((p) => (
          <button key={p.id} type="button" aria-pressed={p.id === period} onClick={() => setPeriod(p.id)}>
            {p.label}
          </button>
        ))}
      </div>
      {state.status === 'loading' && <p role="status">Loading usage</p>}
      {state.status === 'error' && (
        <p role="alert" className="error">
          Could not load usage: {state.message}
        </p>
      )}
      {state.status === 'ready' && (
        <>
          <Widgets summary={state.data.summary} day={state.data.breakdowns.day} previous={state.data.previous} />
          <Totals summary={state.data.summary} total={state.data.breakdowns.day.total} />
          <div className="panels">
            {DIMENSIONS.map((by) => (
              <Breakdown key={by} by={by} data={state.data.breakdowns[by]} />
            ))}
          </div>
          <Outliers data={state.data.outliers} />
          <Timeline data={state.data.sessions} />
        </>
      )}
    </main>
  )
}
