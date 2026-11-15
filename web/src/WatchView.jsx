import { useEffect, useState } from 'react'
import { getJson } from './api.js'
import { compact } from './format.js'

const REFRESH_MS = 60_000 // the sampler interval
const DAY_S = 86400
const W = 200
const H = 40
const PAD = 4

/** One reading as text: whole numbers stay whole, else one decimal. `null` is "no data", never 0. */
const num = (v) => (v === null || v === undefined ? null : Number(v.toFixed(1)))
const pct = (v) => (num(v) === null ? 'no data' : `${num(v)} %`)
const slug = (name) => name.replace(/\W+/g, '-').toLowerCase()
const gb = (bytes) => (bytes / 2 ** 30).toFixed(1)
const ratio = (used, total) => (used === null || total === null || !total ? null : (used / total) * 100)

/**
 * The line of `points` ([{t, v}], oldest first) over the 24 h that end at `end` (the shared "now"), as SVG path text.
 * A `null` value or a pause longer than `gapS` ends the piece, so a gap is drawn as a gap, never as 0.
 * A piece of one reading is a zero-length line (a dot with a round cap). `max` is the top of the scale.
 */
function linePath(points, max, gapS, end) {
  if (points.length === 0) return ''
  const x = (t) => Math.max(0, ((t - (end - DAY_S)) / DAY_S) * W)
  const y = (v) => H - PAD - (Math.min(Math.max(v, 0), max) / max) * (H - 2 * PAD)
  const pieces = []
  let open = null
  let before = null
  for (const { t, v } of points) {
    if (v === null || v === undefined || (before !== null && t - before > gapS)) open = null
    before = t
    if (v === null || v === undefined) continue
    const p = `${Number(x(t).toFixed(2))} ${Number(y(v).toFixed(2))}`
    if (open) open.push(p)
    else pieces.push((open = [p]))
  }
  return pieces.map((piece) => `M${piece[0]} L${piece.at(-1) === piece[0] ? piece[0] : piece.slice(1).join(' L')}`).join(' ')
}

/** Frame of one 24 h graph: the lines are the children. */
function Chart({ label, tall, children }) {
  return (
    <>
      <svg className={tall ? 'watch-chart watch-chart-tall' : 'watch-chart'} viewBox={`0 0 ${W} ${H}`} preserveAspectRatio="none" role="img" aria-label={label}>
        {children}
      </svg>
      <p className="watch-axis" aria-hidden="true">
        <span>24 h ago</span>
        <span>now</span>
      </p>
    </>
  )
}

/**
 * Read `path` now and every 60 s while the view is open. A tick is skipped while the earlier read is in flight.
 * A failed read keeps the last good `data` next to the error: {status: 'loading'|'ready'|'error', data, message}.
 */
function useWatched(path) {
  const [state, setState] = useState({ status: 'loading' })
  useEffect(() => {
    let current = true
    let busy = false
    const read = () => {
      if (busy) return
      busy = true
      getJson(path)
        .then(
          (data) => current && setState({ status: 'ready', data }),
          (error) => current && setState((s) => ({ status: 'error', message: error.message, data: s.data })),
        )
        .finally(() => {
          busy = false
        })
    }
    read()
    const timer = setInterval(read, REFRESH_MS)
    return () => {
      current = false
      clearInterval(timer)
    }
  }, [path])
  return state
}

/** The newest sample time of all the graphs (`null` when there is no sample): every graph ends there. */
function newestSample(machine, limits) {
  const times = [machine?.samples.at(-1)?.t, ...(limits?.accounts ?? []).map((a) => a.samples.at(-1)?.t)].filter((t) => t !== undefined)
  return times.length ? Math.max(...times) : null
}

/** One part of the page: its heading, an error line (the last good data stays below it), a loading line or `children(data)`. */
function Part({ id, title, label, read, children }) {
  return (
    <section className="card" aria-labelledby={id}>
      <h2 id={id}>{title}</h2>
      {read.status === 'error' && (
        <p role="alert" className="error">
          Could not load {label}: {read.message}
        </p>
      )}
      {read.status === 'loading' ? (
        <p role="status" className="part-loading muted">
          <span className="spinner" aria-hidden="true" />
          Loading {label}
        </p>
      ) : (
        read.data && children(read.data)
      )}
    </section>
  )
}

// The six machine tiles. `of` reads one number (or null) out of a sample, `max` is the top of the scale for the
// graph (null = the highest reading, at least 1), `show` is the latest value as text.
const TILES = [
  { name: 'CPU', of: (s) => s.cpu_pct, max: 100, show: (s) => pct(s.cpu_pct) },
  {
    name: 'RAM',
    of: (s) => ratio(s.ram_used, s.ram_total),
    max: 100,
    show: (s) => (ratio(s.ram_used, s.ram_total) === null ? 'no data' : `${gb(s.ram_used)} / ${gb(s.ram_total)} GB`),
    note: (s) => (ratio(s.ram_used, s.ram_total) === null ? null : `${pct(ratio(s.ram_used, s.ram_total))} used`),
  },
  {
    name: 'Disk',
    of: (s) => ratio(s.disk_used, s.disk_total),
    max: 100,
    show: (s) => pct(ratio(s.disk_used, s.disk_total)),
    note: (s) => (ratio(s.disk_used, s.disk_total) === null ? null : `${gb(s.disk_used)} / ${gb(s.disk_total)} GB`),
  },
  { name: 'Load (1 min)', of: (s) => s.load1, max: null, show: (s) => (s.load1 === null ? 'no data' : s.load1.toFixed(2)) },
  {
    name: 'Battery',
    of: (s) => s.battery_pct,
    max: 100,
    show: (s) => pct(s.battery_pct),
    note: (s) => (s.battery_pct === null || s.charging === null ? null : s.charging ? 'charging' : 'on battery'),
  },
  { name: 'Claude processes', of: (s) => s.claude_procs, max: null, show: (s) => (s.claude_procs === null ? 'no data' : String(s.claude_procs)) },
]

function Machine({ data, now }) {
  if (data.samples.length === 0) return <p className="muted">No samples yet. The first one comes within a minute.</p>
  const latest = data.latest ?? data.samples.at(-1)
  const gapS = 2 * data.interval_s
  return (
    <div className="watch-tiles">
      {TILES.map((tile) => {
        const points = data.samples.map((s) => ({ t: s.t, v: tile.of(s) }))
        const top = tile.max ?? Math.max(1, ...points.map((p) => p.v ?? 0))
        const d = linePath(points, top, gapS, now)
        const value = tile.show(latest)
        const note = tile.note?.(latest)
        return (
          <section key={tile.name} className="watch-tile" aria-labelledby={`watch-${slug(tile.name)}`}>
            <h3 id={`watch-${slug(tile.name)}`} className="kpi-name">{tile.name}</h3>
            <p className="kpi-value">{value}</p>
            {note && <p className="kpi-note">{note}</p>}
            <Chart label={`${tile.name}, last 24 hours, latest ${value}`}>{d && <path className="watch-line" d={d} fill="none" vectorEffect="non-scaling-stroke" />}</Chart>
          </section>
        )
      })}
    </div>
  )
}

const limitClass = (v) => (v !== null && v !== undefined && v >= 90 ? 'limit-red' : undefined)

function Limits({ data, now }) {
  if (data.accounts.length === 0) return <p className="muted">No Claude account yet.</p>
  const gapS = 2 * data.interval_s
  return (
    <div className="watch-accounts">
      {data.accounts.map((account) => {
        const last = account.samples.at(-1) ?? { five_h_pct: null, seven_d_pct: null }
        const five = linePath(account.samples.map((s) => ({ t: s.t, v: s.five_h_pct })), 100, gapS, now)
        const seven = linePath(account.samples.map((s) => ({ t: s.t, v: s.seven_d_pct })), 100, gapS, now)
        const ref = H - PAD - 0.9 * (H - 2 * PAD)
        return (
          <section key={account.id} className="watch-account" aria-labelledby={`watch-account-${account.id}`}>
            <div className="watch-account-head">
              <h3 id={`watch-account-${account.id}`}>{account.label}</h3>
              <p className="watch-values">
                <span className="watch-key watch-key-5h">5 h</span> <span className={limitClass(last.five_h_pct)}>{pct(last.five_h_pct)}</span>
                <span className="watch-key watch-key-7d">7 d</span> <span className={limitClass(last.seven_d_pct)}>{pct(last.seven_d_pct)}</span>
              </p>
            </div>
            <Chart tall label={`${account.label} plan limits, last 24 hours, 5 h ${pct(last.five_h_pct)}, 7 d ${pct(last.seven_d_pct)}`}>
              <line className="watch-ref" x1="0" x2={W} y1={ref} y2={ref} vectorEffect="non-scaling-stroke" />
              {five && <path className="watch-line" d={five} fill="none" vectorEffect="non-scaling-stroke" />}
              {seven && <path className="watch-line-7d" d={seven} fill="none" vectorEffect="non-scaling-stroke" />}
            </Chart>
          </section>
        )
      })}
    </div>
  )
}

/** Cost cell: a sum with an unpriced model says so, and no sum at all is "unknown price", never $0. */
function cost(c) {
  if (c.usd === null) return 'unknown price'
  return c.unknown_price ? `$${c.usd.toFixed(2)} + unknown price` : `$${c.usd.toFixed(2)}`
}

const orNoData = (n) => (n === null || n === undefined ? 'no data' : compact(n))
const DONE_TEXT = { 'not done': 'not done', unknown: 'unknown' }

function Row({ task, line }) {
  const unattributed = !task
  return (
    <tr className={unattributed ? 'watch-unattributed' : undefined}>
      <th scope="row" title={task?.project}>
        {task ? task.task_id : 'Unattributed'}
      </th>
      <td>{compact(line.tokens)}</td>
      <td>{cost(line.api_equivalent_cost)}</td>
      <td>{line.sessions}</td>
      <td>{line.good_results}</td>
      <td>{orNoData(line.tokens_per_good_result)}</td>
      <td>{unattributed ? '—' : task.done_state === 'done' ? task.done_utc.slice(0, 10) : (DONE_TEXT[task.done_state] ?? task.done_state)}</td>
      <td>{unattributed ? '—' : orNoData(task.tokens_per_task_done)}</td>
    </tr>
  )
}

function Effectiveness({ data }) {
  const { tasks, unattributed } = data
  const empty = tasks.length === 0 && unattributed.tokens === 0
  return (
    <>
      {empty ? (
        <p className="muted">No tasks yet.</p>
      ) : (
        <div className="table-wrap table-frame">
          <table>
            <thead>
              <tr>
                <th>Task</th>
                <th>Tokens</th>
                <th>Cost</th>
                <th>Sessions</th>
                <th>Good results</th>
                <th>Tokens per good result</th>
                <th>Done</th>
                <th>Tokens per task done</th>
              </tr>
            </thead>
            <tbody>
              {tasks.map((task) => (
                <Row key={`${task.project}/${task.task_id}`} task={task} line={task} />
              ))}
              <Row task={null} line={unattributed} />
            </tbody>
          </table>
        </div>
      )}
      {data.retention_note && <p className="footnote">{data.retention_note}</p>}
      {(data.notes ?? []).map((note) => (
        <p key={note} className="footnote">
          {note}
        </p>
      ))}
    </>
  )
}

/** The Watch view (FR-100): 24 h machine status, 24 h plan limits per account and effectiveness per task. Read-only. */
export default function WatchView() {
  const machine = useWatched('/api/machine')
  const limits = useWatched('/api/limits/history')
  const effectiveness = useWatched('/api/effectiveness')
  const now = newestSample(machine.data, limits.data)
  return (
    <main className="watch">
      <h1>Watch</h1>
      <p className="muted">Last 24 h, one sample a minute. Kept in memory: the graphs start empty after a restart.</p>
      <Part id="watch-machine" title="Machine" label="machine status" read={machine}>
        {(data) => <Machine data={data} now={now} />}
      </Part>
      <Part id="watch-limits" title="Plan limits" label="plan limits" read={limits}>
        {(data) => <Limits data={data} now={now} />}
      </Part>
      <Part id="watch-effectiveness" title="Effectiveness" label="effectiveness" read={effectiveness}>
        {(data) => <Effectiveness data={data} />}
      </Part>
    </main>
  )
}
