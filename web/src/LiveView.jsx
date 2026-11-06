import { memo, useCallback, useEffect, useMemo, useReducer, useState } from 'react'
import { readToken } from './api.js'
import Cost from './Cost.jsx'
import { ago, compact, fmt } from './format.js'

export const FEED_MAX = 50 // oldest feed items beyond this are dropped
const DEFAULT_WINDOW_S = 300
const CLOSED = 2 // EventSource.readyState: the browser gave up and will not retry
const TICK_MS = 15_000 // how often "active" and "idle" are worked out again
const BATCH_MS = 250 // live events wait at most this long, then reach the page as one update (NFR-015)
const ID_CHARS = 8
const PANEL_ID = 'live-panel'

const EMPTY = { sessions: {}, lastId: 0, windowS: DEFAULT_WINDOW_S }

const priced = (cost) => cost !== null && cost !== undefined
const hasTokens = (tokensIn, tokensOut) => (tokensIn ?? 0) + (tokensOut ?? 0) > 0

const SAME_KEYS = ['account', 'project', 'last_ts', 'tool_calls', 'tokens_in', 'tokens_out', 'cost', 'unpriced', 'stage', 'stage_skill']

function fromSnapshot(s, old) {
  const next = {
    session_id: s.session_id,
    account: s.account,
    project: s.project,
    last_ts: s.last_ts,
    tool_calls: s.tool_calls,
    tokens_in: s.tokens_in,
    tokens_out: s.tokens_out,
    cost: s.cost_usd ?? 0,
    unpriced: !priced(s.cost_usd) && hasTokens(s.tokens_in, s.tokens_out),
    stage: s.stage ?? null, // the kit stage and its skill come from the server's snapshot (FR-083); the page never works one out
    stage_skill: s.stage_skill ?? null,
    attention: s.attention ?? [], // the server's states; the page never works one out itself
    feed: old?.feed ?? [],
  }
  // A session the snapshot did not change keeps its object, so its memoized row does not draw again.
  const same = old && SAME_KEYS.every((k) => old[k] === next[k]) && JSON.stringify(old.attention) === JSON.stringify(next.attention)
  return same ? old : next
}

const STATE_ORDER = ['failing', 'waiting', 'needs_attention'] // the first one of a session names its border

/** A session's states, failing first. Unknown state names are left out. */
const statesOf = (s) => STATE_ORDER.flatMap((state) => s.attention.filter((a) => a.state === state))

/** The words of one state. `tool: null` means the server could not name one (mixed or unknown tools). */
function pillText(a) {
  const agent = a.is_sidechain ? ' · agent' : ''
  if (a.state === 'failing') return a.tool ? `failing: ${a.tool} ×${a.count}${agent}` : `failing: ${a.count} in a row${agent}`
  if (a.state === 'waiting') return `waiting for you: question${agent}`
  return `needs attention: ${a.tool ?? 'unknown tool'}${agent} · hung or waiting for permission`
}

const isName = (v) => typeof v === 'string' && v !== ''

/** The call label as one line of text: a name, or "server · tool" for MCP. Any other shape gives no label. */
function labelText(label) {
  if (isName(label)) return label
  if (label && typeof label === 'object' && isName(label.server) && isName(label.tool)) return `${label.server} · ${label.tool}`
  return null
}

/** Who made the call: the sub-agent type, "sub-agent" for a sidechain call whose type is unknown, else "main". */
const whoMade = (e) => (isName(e.agent) ? e.agent : e.is_sidechain ? 'sub-agent' : 'main')

/** A feed row is a tool call, or a tool result that failed. Everything else only moves the totals. */
const feedItem = (e) =>
  e.kind === 'tool_use' || (e.kind === 'tool_result' && e.is_error)
    ? [{ id: e.id, ts: e.ts, kind: e.kind, tool: e.tool ?? e.kind, is_error: Boolean(e.is_error), who: whoMade(e), label: labelText(e.label) }]
    : []

function applyLive(state, e) {
  if (e.id <= state.lastId) return state // already seen
  const s = state.sessions[e.session_id] ?? fromSnapshot({ ...e, last_ts: e.ts, tool_calls: 0, tokens_in: 0, tokens_out: 0, cost_usd: 0, stage: null, stage_skill: null }) // stage only comes in a snapshot
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

function reduce(state, action) {
  if (action.type === 'snapshot') {
    // A last_id below the one we reached means the server restarted and its ids began again: old feed rows would repeat ids.
    const restarted = (action.data.last_id ?? 0) < state.lastId
    const sessions = Object.fromEntries(
      action.data.sessions.map((s) => [s.session_id, fromSnapshot(s, restarted ? undefined : state.sessions[s.session_id])]),
    )
    // The snapshot's totals already count every event up to last_id; ids never restart, so older ones are dropped.
    return { sessions, lastId: action.data.last_id ?? 0, windowS: action.data.active_window_s ?? DEFAULT_WINDOW_S }
  }
  return action.events.reduce(applyLive, state) // `batch`: the events of one window, in order
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
const needsYou = (s) => statesOf(s).length > 0

/** account -> sessions: the ones needing you first, then the newest. Accounts in name order. */
function group(sessions) {
  const accounts = {}
  for (const s of sessions) (accounts[s.account] ??= []).push(s)
  return Object.keys(accounts)
    .sort(before)
    .map((account) => ({
      account,
      sessions: accounts[account].sort((a, b) => needsYou(b) - needsYou(a) || before(b.last_ts, a.last_ts) || before(a.session_id, b.session_id)),
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
      {items.length === 0 && <p className="muted">No tool calls seen since this page opened.</p>}
      <ol className="live-feed" aria-live="polite" aria-relevant="additions" aria-label="Tool calls, newest first">
        {items.map((i) => (
          <li key={i.id}>
            <span className="live-time">{clock(i.ts)}</span>
            <span className="live-who">{i.who}</span>
            <span className="live-tool">{i.tool}</span>
            {i.label && <span className="live-label">{i.label}</span>}
            {i.is_error && <span className="live-tag error"> error</span>}
          </li>
        ))}
      </ol>
    </>
  )
}

const noSpace = (n) => compact(n).replace(' ', '') // "1.2M": the number beside a bar is short

/** A full bar is the largest value among the listed sessions (owner choice, 2026-11-04). */
const share = (value, max) => (max > 0 ? Math.round((value / max) * 1000) / 10 : 0)

/** One measure. `ratio` is 0..100; null draws no bar at all (the price is unknown, which is not $0). */
function Bar({ label, text, ratio }) {
  return (
    <span className="live-bar" role="img" aria-label={`${label} ${text}`}>
      <span className="live-bar-label" aria-hidden="true">
        {label}
      </span>
      <span className="live-bar-num" aria-hidden="true">
        {text}
      </span>
      <span className="live-bar-track" aria-hidden="true">
        {ratio !== null && <span className="live-bar-fill" style={{ width: `${ratio}%` }} />}
      </span>
    </span>
  )
}

/** One session: a button (id, project, states, time) with its three bars. Memoized: it draws again only when its own figures change. */
const SessionRow = memo(function SessionRow({ s, now, windowS, open, onToggle, tokensPct, costPct, callsPct }) {
  const states = statesOf(s)
  const idle = states.length === 0 && now - Date.parse(s.last_ts) > windowS * 1000 // a session with a state is never idle
  const tokens = s.tokens_in + s.tokens_out
  const unknown = s.unpriced && s.cost === 0
  return (
    <li className="live-session" data-idle={idle || undefined} data-state={states[0]?.state} data-open={open || undefined}>
      <button type="button" id={`live-row-${s.session_id}`} className="live-row" aria-expanded={open} aria-controls={PANEL_ID} onClick={() => onToggle(s.session_id)}>
        <span className="live-id" title={s.session_id}>
          {s.session_id.slice(0, ID_CHARS)}
        </span>
        <span className="live-name" title={s.project}>
          {s.project}
        </span>
        {s.stage !== null && (
          <span className="live-stage">{s.stage_skill ? `Stage ${s.stage} · ${s.stage_skill}` : `Stage ${s.stage}`}</span>
        )}
        {states.map((a, i) => (
          <span key={`${a.state}-${i}`} className="live-pill" data-state={a.state} title={pillText(a)}>
            {pillText(a)}
          </span>
        ))}
        <span className="count">{`${idle ? 'idle' : 'active'} · ${ago(s.last_ts, now)}`}</span>
      </button>
      <span className="live-bars">
        <Bar label="tokens" text={noSpace(tokens)} ratio={tokensPct} />
        <Bar
          label="cost"
          text={unknown ? 'unknown price' : `$${s.cost.toFixed(2)}${s.unpriced ? ' + unknown price' : ''}`}
          ratio={unknown ? null : costPct}
        />
        <Bar label="tool calls" text={fmt(s.tool_calls)} ratio={callsPct} />
      </span>
    </li>
  )
})

/** The one open session: its totals and its recent tool calls. `s` is undefined once the session left the list. */
function SessionPanel({ s, onClose }) {
  return (
    <section id={PANEL_ID} className="card live-panel" aria-labelledby="live-panel-title">
      <div className="card-head">
        <h2 id="live-panel-title">{s ? `Session ${s.session_id.slice(0, ID_CHARS)}` : 'Session'}</h2>
        <button type="button" className="live-close" onClick={onClose}>
          Close
        </button>
      </div>
      {s ? (
        <>
          <p className="live-name" title={s.project}>
            {s.project}
          </p>
          <Totals total={s} />
          <Feed items={s.feed} />
        </>
      ) : (
        <p className="muted">This session is no longer active.</p>
      )}
    </section>
  )
}

const STATUS = {
  connecting: 'Connecting to the live stream',
  live: 'Live stream connected',
  reconnecting: 'Reconnecting: the stream dropped, showing what we had',
  closed: 'The live stream closed. Check that the dashboard is still running.',
}

/** The stream: one EventSource on /api/live. The browser reconnects on its own and resumes with Last-Event-ID.
 * `live` events wait in a queue and go to the reducer as one batch every BATCH_MS; a snapshot flushes the queue first, so order holds. */
function useLive(attempt) {
  const [state, dispatch] = useReducer(reduce, EMPTY)
  const [link, setLink] = useState('connecting')
  useEffect(() => {
    setLink('connecting')
    const source = new EventSource(`/api/live?token=${encodeURIComponent(readToken())}`)
    let queue = []
    let timer = null
    const flush = () => {
      clearTimeout(timer)
      timer = null
      if (queue.length === 0) return
      const events = queue
      queue = []
      dispatch({ type: 'batch', events })
    }
    const parse = (event) => {
      try {
        const data = JSON.parse(event.data)
        return data && typeof data === 'object' ? data : undefined
      } catch {
        return undefined // a message that is not JSON is skipped
      }
    }
    source.addEventListener('open', () => setLink('live'))
    source.addEventListener('error', () => setLink(source.readyState === CLOSED ? 'closed' : 'reconnecting'))
    source.addEventListener('snapshot', (event) => {
      const data = parse(event)
      if (!data) return
      flush()
      dispatch({ type: 'snapshot', data })
    })
    source.addEventListener('live', (event) => {
      const data = parse(event)
      if (!data) return
      queue.push(data)
      timer ??= setTimeout(flush, BATCH_MS)
    })
    return () => {
      clearTimeout(timer)
      source.close()
    }
  }, [attempt])
  return [state, link]
}

/** `// work` › Live: active Claude Code sessions as rows with three bars; one open session shows its totals and tool feed. Metadata only. */
export default function LiveView() {
  const [attempt, setAttempt] = useState(0)
  const [state, link] = useLive(attempt)
  const [now, setNow] = useState(Date.now())
  const [openId, setOpenId] = useState(null) // not kept across a reload
  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), TICK_MS)
    return () => clearInterval(timer)
  }, [])
  const onToggle = useCallback((id) => setOpenId((current) => (current === id ? null : id)), [])
  const sessions = useMemo(() => Object.values(state.sessions), [state.sessions])
  const groups = useMemo(() => group(sessions), [sessions])
  const needing = sessions.filter(needsYou).length
  const maxTokens = Math.max(0, ...sessions.map((s) => s.tokens_in + s.tokens_out))
  const maxCost = Math.max(0, ...sessions.map((s) => s.cost))
  const maxCalls = Math.max(0, ...sessions.map((s) => s.tool_calls))
  const closePanel = () => {
    document.getElementById(`live-row-${openId}`)?.focus() // keep the keyboard where it was
    setOpenId(null)
  }
  return (
    <main className="live">
      <div className="live-head">
        <h1>Live</h1>
        {needing > 0 && (
          <p role="status" className="live-need">
            {needing === 1 ? '1 session needs you' : `${needing} sessions need you`}
          </p>
        )}
      </div>
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
          API-equivalent cost is tokens times the API price table, not what a subscription pays. Only tool names, skill, sub-agent, MCP and command names, counts and times are shown, never message or tool content.
        </p>
      </section>
      {sessions.length === 0 && (
        <p className="muted">{`No active sessions. A session appears here when it wrote a record in the last ${Math.round(state.windowS / 60)} minutes.`}</p>
      )}
      {(sessions.length > 0 || openId !== null) && (
        <div className="live-body" data-panel={openId !== null || undefined}>
          <div className="live-sessions">
            {groups.map((a, i) => (
              <section key={a.account} aria-labelledby={`live-acc-${i}`}>
                <h2 id={`live-acc-${i}`}>{a.account}</h2>
                <ul className="live-list">
                  {a.sessions.map((s) => (
                    <SessionRow
                      key={s.session_id}
                      s={s}
                      now={now}
                      windowS={state.windowS}
                      open={s.session_id === openId}
                      onToggle={onToggle}
                      tokensPct={share(s.tokens_in + s.tokens_out, maxTokens)}
                      costPct={share(s.cost, maxCost)}
                      callsPct={share(s.tool_calls, maxCalls)}
                    />
                  ))}
                </ul>
              </section>
            ))}
          </div>
          {openId !== null && <SessionPanel s={state.sessions[openId]} onClose={closePanel} />}
        </div>
      )}
    </main>
  )
}
