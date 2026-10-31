import { useEffect, useReducer, useState } from 'react'
import { readToken } from './api.js'
import Cost from './Cost.jsx'
import { ago, fmt } from './format.js'

export const FEED_MAX = 50 // oldest feed items beyond this are dropped
const DEFAULT_WINDOW_S = 300
const CLOSED = 2 // EventSource.readyState: the browser gave up and will not retry
const TICK_MS = 15_000 // how often "active" and "idle" are worked out again
const ID_CHARS = 8

const EMPTY = { sessions: {}, lastId: 0, windowS: DEFAULT_WINDOW_S }

const priced = (cost) => cost !== null && cost !== undefined
const hasTokens = (tokensIn, tokensOut) => (tokensIn ?? 0) + (tokensOut ?? 0) > 0

function fromSnapshot(s, old) {
  return {
    session_id: s.session_id,
    account: s.account,
    project: s.project,
    last_ts: s.last_ts,
    tool_calls: s.tool_calls,
    tokens_in: s.tokens_in,
    tokens_out: s.tokens_out,
    cost: s.cost_usd ?? 0,
    unpriced: !priced(s.cost_usd) && hasTokens(s.tokens_in, s.tokens_out),
    feed: old?.feed ?? [],
  }
}

/** A feed row is a tool call, or a tool result that failed. Everything else only moves the totals. */
const feedItem = (e) =>
  e.kind === 'tool_use' || (e.kind === 'tool_result' && e.is_error)
    ? [{ id: e.id, ts: e.ts, kind: e.kind, tool: e.tool ?? e.kind, is_error: Boolean(e.is_error), is_sidechain: Boolean(e.is_sidechain) }]
    : []

function reduce(state, action) {
  if (action.type === 'snapshot') {
    const sessions = Object.fromEntries(action.data.sessions.map((s) => [s.session_id, fromSnapshot(s, state.sessions[s.session_id])]))
    // A snapshot is the whole truth: the server numbers its events from here.
    return { sessions, lastId: 0, windowS: action.data.active_window_s ?? DEFAULT_WINDOW_S }
  }
  const e = action.data
  if (e.id <= state.lastId) return state // already seen
  const s = state.sessions[e.session_id] ?? fromSnapshot({ ...e, last_ts: e.ts, tool_calls: 0, tokens_in: 0, tokens_out: 0, cost_usd: 0 })
  const next = {
    ...s,
    last_ts: e.ts,
    tool_calls: s.tool_calls + (e.kind === 'tool_use' ? 1 : 0),
    tokens_in: s.tokens_in + (e.tokens_in ?? 0),
    tokens_out: s.tokens_out + (e.tokens_out ?? 0),
    cost: s.cost + (e.cost_usd ?? 0),
    unpriced: s.unpriced || (!priced(e.cost_usd) && hasTokens(e.tokens_in, e.tokens_out)),
    feed: [...feedItem(e), ...s.feed].slice(0, FEED_MAX),
  }
  return { ...state, lastId: e.id, sessions: { ...state.sessions, [e.session_id]: next } }
}

const sum = (list) =>
  list.reduce(
    (t, s) => ({
      tool_calls: t.tool_calls + s.tool_calls,
      tokens_in: t.tokens_in + s.tokens_in,
      tokens_out: t.tokens_out + s.tokens_out,
      cost: t.cost + s.cost,
      unpriced: t.unpriced || s.unpriced,
    }),
    { tool_calls: 0, tokens_in: 0, tokens_out: 0, cost: 0, unpriced: false },
  )

const before = (a, b) => (a < b ? -1 : a > b ? 1 : 0)

/** account -> project -> sessions, names in order, the newest session of a project first. */
function group(sessions) {
  const accounts = {}
  for (const s of sessions) ((accounts[s.account] ??= {})[s.project] ??= []).push(s)
  return Object.keys(accounts)
    .sort(before)
    .map((account) => ({
      account,
      projects: Object.keys(accounts[account])
        .sort(before)
        .map((project) => ({ project, sessions: accounts[account][project].sort((a, b) => before(b.last_ts, a.last_ts)) })),
    }))
}

function Totals({ total }) {
  // Cost is shown like the Usage view: an unknown price is named, never added in as $0.
  const cost = { usd: total.unpriced && total.cost === 0 ? null : total.cost, abstained_models: total.unpriced ? ['unknown price'] : [] }
  return (
    <dl className="live-totals">
      <div>
        <dt>Tool calls</dt>
        <dd>{fmt(total.tool_calls)}</dd>
      </div>
      <div>
        <dt>Tokens in</dt>
        <dd>{fmt(total.tokens_in)}</dd>
      </div>
      <div>
        <dt>Tokens out</dt>
        <dd>{fmt(total.tokens_out)}</dd>
      </div>
      <div>
        <dt>API-equivalent cost</dt>
        <dd>
          <Cost cost={cost} />
        </dd>
      </div>
    </dl>
  )
}

const clock = (ts) => new Date(ts).toLocaleTimeString()

function Feed({ items }) {
  return (
    <>
      {items.length === 0 && <p className="muted">No tool calls yet.</p>}
      <ol className="live-feed" aria-live="polite" aria-relevant="additions" aria-label="Tool calls, newest first">
        {items.map((i) => (
          <li key={i.id}>
            <span className="live-time">{clock(i.ts)}</span>
            <span className="live-tool">{i.tool}</span>
            {i.is_sidechain && <span className="live-tag"> agent</span>}
            {i.is_error && <span className="live-tag error"> error</span>}
          </li>
        ))}
      </ol>
    </>
  )
}

function Session({ s, now, windowS }) {
  const idle = now - Date.parse(s.last_ts) > windowS * 1000
  return (
    <li className="live-session" data-idle={idle || undefined}>
      <div className="card-head">
        <h4 className="live-id" title={s.session_id}>
          {s.session_id.slice(0, ID_CHARS)}
        </h4>
        <span className="count">{`${idle ? 'idle' : 'active'} · ${ago(s.last_ts, now)}`}</span>
      </div>
      <Totals total={s} />
      <Feed items={s.feed} />
    </li>
  )
}

const STATUS = {
  connecting: 'Connecting to the live stream',
  live: 'Live stream connected',
  reconnecting: 'Reconnecting: the stream dropped, showing what we had',
  closed: 'The live stream closed. Check that the dashboard is still running.',
}

/** The stream: one EventSource on /api/live. The browser reconnects on its own and resumes with Last-Event-ID. */
function useLive(attempt) {
  const [state, dispatch] = useReducer(reduce, EMPTY)
  const [link, setLink] = useState('connecting')
  useEffect(() => {
    setLink('connecting')
    const source = new EventSource(`/api/live?token=${encodeURIComponent(readToken())}`)
    const onData = (type) => (event) => {
      try {
        dispatch({ type, data: JSON.parse(event.data) })
      } catch {
        // a message that is not JSON is skipped
      }
    }
    source.addEventListener('open', () => setLink('live'))
    source.addEventListener('error', () => setLink(source.readyState === CLOSED ? 'closed' : 'reconnecting'))
    source.addEventListener('snapshot', onData('snapshot'))
    source.addEventListener('live', onData('live'))
    return () => source.close()
  }, [attempt])
  return [state, link]
}

/** `// work` › Live: active Claude Code sessions, a feed of their tool calls and running totals. Metadata only. */
export default function LiveView() {
  const [attempt, setAttempt] = useState(0)
  const [state, link] = useLive(attempt)
  const [now, setNow] = useState(Date.now())
  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), TICK_MS)
    return () => clearInterval(timer)
  }, [])
  const sessions = Object.values(state.sessions)
  return (
    <main className="live">
      <h1>Live</h1>
      <div className="live-status">
        <p role="status">{STATUS[link]}</p>
        {link === 'closed' && (
          <button type="button" onClick={() => setAttempt((n) => n + 1)}>
            Reconnect
          </button>
        )}
      </div>
      <section className="card" aria-labelledby="h-live-all">
        <h2 id="h-live-all">All sessions</h2>
        <Totals total={sum(sessions)} />
        <p className="footnote">
          API-equivalent cost is tokens times the API price table, not what a subscription pays. Only tool names, counts and times are shown, never message or tool content.
        </p>
      </section>
      {sessions.length === 0 ? (
        <p className="muted">{`No active sessions. A session appears here when it wrote a record in the last ${Math.round(state.windowS / 60)} minutes.`}</p>
      ) : (
        <div className="live-sessions">
          {group(sessions).map((a, i) => (
            <section key={a.account} aria-labelledby={`live-acc-${i}`}>
              <h2 id={`live-acc-${i}`}>{a.account}</h2>
              {a.projects.map((p) => (
                <div key={p.project} className="live-project">
                  <h3 className="live-name" title={p.project}>
                    {p.project}
                  </h3>
                  <ul className="live-grid">
                    {p.sessions.map((s) => (
                      <Session key={s.session_id} s={s} now={now} windowS={state.windowS} />
                    ))}
                  </ul>
                </div>
              ))}
            </section>
          ))}
        </div>
      )}
    </main>
  )
}
