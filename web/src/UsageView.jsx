import { useEffect, useState } from 'react'
import { getJson } from './api.js'
import Breakdown from './Breakdown.jsx'
import Cost from './Cost.jsx'
import Timeline from './Timeline.jsx'
import { fmt, percent, totalTokens } from './format.js'

const PERIODS = [
  { id: '7', label: 'Last 7 days', days: 7 },
  { id: '30', label: 'Last 30 days', days: 30 },
  { id: 'all', label: 'All time', days: null },
]
const DIMENSIONS = ['day', 'model', 'project', 'tool']

function query(days) {
  if (days === null) return ''
  return `since=${encodeURIComponent(new Date(Date.now() - days * 86400000).toISOString())}`
}

async function load(days) {
  const q = query(days)
  const url = (path, extra = '') => `${path}?${[extra, q].filter(Boolean).join('&')}`
  const [summary, outliers, sessions, ...breakdowns] = await Promise.all([
    getJson(url('/api/usage/summary')),
    getJson(url('/api/usage/outliers')),
    getJson(url('/api/usage/sessions')),
    ...DIMENSIONS.map((by) => getJson(url('/api/usage/breakdown', `by=${by}`))),
  ])
  return { summary, outliers, sessions, breakdowns: Object.fromEntries(DIMENSIONS.map((d, i) => [d, breakdowns[i]])) }
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

export default function UsageView() {
  const [period, setPeriod] = useState('7')
  const [state, setState] = useState({ status: 'loading' })

  useEffect(() => {
    let current = true
    setState({ status: 'loading' })
    load(PERIODS.find((p) => p.id === period).days).then(
      (data) => current && setState({ status: 'ready', data }),
      (error) => current && setState({ status: 'error', message: error.message }),
    )
    return () => {
      current = false
    }
  }, [period])

  return (
    <main>
      <h1>Usage</h1>
      <p className="notice" role="note">
        Not included: cloud sessions and other machines. This view counts only Claude Code sessions stored on this machine.
      </p>
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
