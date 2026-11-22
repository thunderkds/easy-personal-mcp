import { createContext, lazy, memo, Suspense, useCallback, useContext, useEffect, useMemo, useReducer, useRef, useState } from 'react'
import { accountLabel, endedWords, getAppSessions, getTermSessions, isExited, readToken } from './api.js'
import Cost from './Cost.jsx'
import { ago, compact, fmt } from './format.js'
import FilterBar, { kindOf, loadFilters, passes, saveFilters } from './LiveFilters.jsx'
import Loading from './Loading.jsx'
import PastSessions from './PastSessions.jsx'
import NewSessionSheet from './NewSessionSheet.jsx'
import { useNotices } from './PendingAnswer.jsx'
import { readPrompt } from './PromptCard.jsx'
import SessionPanel, { STATES, folderName } from './SessionPanel.jsx'

// The terminal panel carries xterm (about 85 KB gzip): it loads when the first terminal session is opened, not with the page (T134, FR-138).
const TermPanel = lazy(() => import('./TermPanel.jsx'))

export const FEED_MAX = 50 // oldest feed items beyond this are dropped
const DEFAULT_WINDOW_S = 300
const CLOSED = 2 // EventSource.readyState: the browser gave up and will not retry
const TICK_MS = 15_000 // how often "active" and "idle" are worked out again
const BATCH_MS = 250 // live events wait at most this long, then reach the page as one update (NFR-015)
const ID_CHARS = 8
const QUIET_S = 120 // the server's resume check (live.rs RESUME_QUIET_S): a newer record than this is active

const EMPTY = { sessions: {}, lastId: 0, windowS: DEFAULT_WINDOW_S }

const priced = (cost) => cost !== null && cost !== undefined
const hasTokens = (tokensIn, tokensOut) => (tokensIn ?? 0) + (tokensOut ?? 0) > 0

const SAME_KEYS = ['account', 'project', 'last_ts', 'tool_calls', 'tokens_in', 'tokens_out', 'cost', 'unpriced', 'stage', 'stage_skill', 'replyMarked', 'replyBridge', 'name', 'branch', 'entrypoint']

const text = (v) => (typeof v === 'string' && v !== '' ? v : null)
/** The process file's part of a snapshot row (T116, FR-124): `{busy, name}` or null. The server's word, shown as it came. */
function readProc(p) {
  return p && typeof p === 'object' ? { busy: p.busy === true, name: text(p.name) } : null
}
/** The text queued for a busy terminal session (T116, FR-125): `{text}` or null. */
function readQueued(q) {
  return q && typeof q === 'object' && typeof q.text === 'string' ? { text: q.text } : null
}

/** The hold of a terminal session at your turn (T111's `pending_reply`, FR-121): kept only when well formed. `until` is epoch seconds. */
function readReply(r) {
  if (!r || typeof r.id !== 'string' || typeof r.until !== 'number') return null
  return { id: r.id, until: r.until, last: typeof r.last === 'string' ? r.last : '' }
}

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
    pending: readPrompt(s.pending_answer), // the prompt a terminal session waits on (T107, FR-114): the server's, shown as it came
    reply: readReply(s.pending_reply), // the hold a terminal session at your turn waits in for the owner's text (T112, FR-122)
    replyMarked: s.reply_marked === true, // the owner marked it *Reply from the app*: the server's word
    replyBridge: s.reply_bridge === true, // all three hook entries of its account are on (T112): the server's word, never worked out here
    proc: readProc(s.proc), // the session's process file: busy or idle, and Claude Code's own name (T116)
    queued: readQueued(s.queued), // the text waiting for this session's turn end (T116)
    name: text(s.name), // the owner's own name for the session (T117, FR-127); the page only shows it
    branch: text(s.branch),
    entrypoint: text(s.entrypoint), // Claude Code's entrypoint, when the snapshot carries it: `sdk…` is a headless run
    feed: old?.feed ?? [],
  }
  // A session the snapshot did not change keeps its object, so its memoized row does not draw again.
  const same = old && SAME_KEYS.every((k) => old[k] === next[k]) && JSON.stringify(old.attention) === JSON.stringify(next.attention) && JSON.stringify(old.pending) === JSON.stringify(next.pending) && JSON.stringify(old.reply) === JSON.stringify(next.reply) && JSON.stringify(old.proc) === JSON.stringify(next.proc) && JSON.stringify(old.queued) === JSON.stringify(next.queued)
  return same ? old : next
}

const STATE_ORDER = ['failing', 'waiting', 'your_turn', 'needs_attention'] // the first one of a session names its border

/** A session's states, failing first. Unknown state names are left out. */
export const statesOf = (s) => STATE_ORDER.flatMap((state) => s.attention.filter((a) => a.state === state))

/** A session with no state that wrote nothing for the active window is idle; a session with a state is never idle. */
export const isIdle = (s, now, windowS) => statesOf(s).length === 0 && now - Date.parse(s.last_ts) > windowS * 1000

/** The words of one state. `tool: null` means the server could not name one (mixed or unknown tools). */
function pillText(a) {
  const agent = a.is_sidechain ? ' · agent' : ''
  if (a.state === 'failing') return a.tool ? `failing: ${a.tool} ×${a.count}${agent}` : `failing: ${a.count} in a row${agent}`
  if (a.state === 'waiting') return `waiting for you: question${agent}`
  if (a.state === 'your_turn') return `your turn${agent}`
  return `needs attention: ${a.tool ?? 'unknown tool'}${agent} · hung or waiting for permission`
}

/** The questions a waiting state carries (T094), as plain text: at most 4, each cut to 300 characters, blank ones dropped. Another state, or anything but a list of strings, gives none. */
const MAX_QUESTIONS = 4
const MAX_QUESTION_CHARS = 300
const questionsOf = (a) =>
  a.state === 'waiting' && Array.isArray(a.questions) && a.questions.every((q) => typeof q === 'string')
    ? a.questions
        .slice(0, MAX_QUESTIONS)
        .filter((q) => q.trim() !== '')
        .map((q) => [...q].slice(0, MAX_QUESTION_CHARS).join(''))
    : []

/** The question text of a session's states: React text under the badge, never HTML (DDR-0005). */
function Questions({ states }) {
  const list = states.flatMap(questionsOf)
  if (list.length === 0) return null
  return (
    <div className="live-questions">
      {list.map((q, i) => (
        <p key={i} className="live-question">
          {q}
        </p>
      ))}
    </div>
  )
}

const isName = (v) => typeof v === 'string' && v !== ''

const isMcp = (label) => Boolean(label) && typeof label === 'object' && isName(label.server) && isName(label.tool)

/** The call label as one line of text: a name, or "server · tool" for MCP. Any other shape gives no label. */
function labelText(label) {
  if (isName(label)) return label
  if (isMcp(label)) return `${label.server} · ${label.tool}`
  return null
}

/** Who made the call: the sub-agent type, "sub-agent" for a sidechain call whose type is unknown, else "main". */
const whoMade = (e) => (isName(e.agent) ? e.agent : e.is_sidechain ? 'sub-agent' : 'main')

/** A feed row is a tool call, or a tool result that failed. Everything else only moves the totals. */
const feedItem = (e) =>
  e.kind === 'tool_use' || (e.kind === 'tool_result' && e.is_error)
    ? [{ id: e.id, ts: e.ts, kind: e.kind, tool: e.tool ?? e.kind, is_error: Boolean(e.is_error), who: whoMade(e), label: labelText(e.label), mcp: isMcp(e.label) }]
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
const asksYou = (e) => needsYou(e.s) || e.s.pending !== null || e.card // a session with a prompt card is listed with the ones that need you, and counts for the *needs me* filter

/**
 * The state filter's word for one listed session (FR-128). Needs me: a state, or a prompt waiting in the panel. Otherwise working or idle: an app
 * session by its own state; a terminal session by its process file's busy flag, else by the active window the row's caption uses.
 */
export function statusOf(e, now, windowS) {
  if (asksYou(e)) return 'needs'
  if (e.app) return e.app.state === 'working' || e.app.state === 'stopping' ? 'working' : 'idle'
  if (e.s.proc) return e.s.proc.busy ? 'working' : 'idle'
  return isIdle(e.s, now, windowS) ? 'idle' : 'working'
}

/** The id a row keeps while its key changes from the Claude id to `app:<id>` (T121). */
const rowId = (e) => e.s.session_id ?? e.key

/** The focus came from the keyboard (`:focus-visible`); a browser that cannot tell is taken as yes. */
function keyboardFocus(el) {
  try {
    return el.matches(':focus-visible')
  } catch {
    return true
  }
}

/** account -> entries: the ones needing you first, then the newest. Accounts in name order. `held` (T121, F7): row id -> place while the pointer or
 * the focus is on the list; those rows keep their order, rows new to the list come after them. */
function group(entries, held = null) {
  const accounts = {}
  for (const e of entries) (accounts[e.s.account] ??= []).push(e)
  const fresh = (a, b) => asksYou(b) - asksYou(a) || before(b.s.last_ts, a.s.last_ts) || before(a.key, b.key)
  const place = (e) => held?.get(rowId(e)) ?? Infinity
  return Object.keys(accounts)
    .sort(before)
    .map((account) => ({
      account,
      entries: accounts[account].sort((a, b) => (held ? place(a) - place(b) || 0 : 0) || fresh(a, b)),
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

const pad = (n) => String(n).padStart(2, '0')
/** Local time, 24 h, as HH:MM:SS. */
const clock = (ts) => {
  const d = new Date(ts)
  return `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`
}

/** One line per call: time, who, then the call. An MCP call shows only "server · tool"; any other shows the tool name and its label.
 * Every cell is cut with an ellipsis; the row's title holds the full text. */
function Feed({ items }) {
  return (
    <>
      {items.length === 0 && <p className="muted">No tool calls seen since this page opened.</p>}
      <ol className="live-feed" aria-live="polite" aria-relevant="additions" aria-label="Tool calls, newest first">
        {items.map((i) => {
          const time = clock(i.ts)
          const title = [time, i.who, i.tool, i.label, i.is_error ? 'error' : null].filter(Boolean).join(' · ')
          return (
            <li key={i.id} title={title}>
              <span className="live-time">{time}</span>
              <span className="live-who">{i.who}</span>
              <span className="live-call">
                {i.mcp ? (
                  <span className="live-label">{i.label}</span>
                ) : (
                  <>
                    <span className="live-tool">{i.tool}</span>
                    {i.label && (
                      <>
                        {' '}
                        <span className="live-label">{i.label}</span>
                      </>
                    )}
                  </>
                )}
              </span>
              {i.is_error && <span className="live-tag error">error</span>}
            </li>
          )
        })}
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

/** One session: a button (id, project, states, time) with its three bars. Selecting it opens its panel (FR-128); the row stays in the list.
 * Memoized: it draws again only when its own figures (or its open state) change. */
const SessionRow = memo(function SessionRow({ s, k, now, windowS, open, reveal, card, onToggle, tokensPct, costPct, callsPct }) {
  const states = statesOf(s)
  const idle = isIdle(s, now, windowS)
  const tokens = s.tokens_in + s.tokens_out
  const unknown = s.unpriced && s.cost === 0
  const caption = s.appOnly ? s.appState : `${idle ? 'idle' : 'active'} · ${ago(s.last_ts, now)}`
  const cardWaiting = (s.pending !== null || card) && !states.some((a) => a.state === 'waiting')
  const li = useRef(null)
  useEffect(() => {
    if (open && reveal) li.current?.scrollIntoView?.({ block: 'nearest' }) // only a session picked from the address is brought into view
  }, [open, reveal])
  return (
    <li
      ref={li}
      className="live-session"
      data-idle={idle || undefined}
      data-state={states[0]?.state}
      data-open={open || undefined}
      onClick={(e) => {
        if (String(window.getSelection?.() ?? '') !== '') return // the owner selected text on the card: that is not a click to open
        if (!e.target.closest('button, a, input, textarea, select, summary')) onToggle(k) // T121 (F20): the whole card opens the row; the button keeps the keyboard
      }}
      onKeyDown={(e) => {
        if (open && e.key === 'Escape') onToggle(k) // the focus is on this row: the panel closes and the row keeps it
      }}
    >
      <button type="button" id={`live-row-${k}`} className="live-row" aria-expanded={open} aria-current={open ? 'true' : undefined} aria-controls="live-panel" onClick={() => onToggle(k)}>
        <span className="live-id" title={s.session_id ?? 'starting'}>
          {(s.session_id ?? 'starting').slice(0, ID_CHARS)}
        </span>
        <span className="live-name" title={s.project}>
          {s.name ?? s.proc?.name ?? s.project}
        </span>
        {s.stage !== null && (
          <span className="live-stage">{s.stage_skill ? `Stage ${s.stage} · ${s.stage_skill}` : `Stage ${s.stage}`}</span>
        )}
        {states.map((a, i) => (
          <span key={`${a.state}-${i}`} className="live-pill" data-state={a.state} title={pillText(a)}>
            {pillText(a)}
          </span>
        ))}
        {cardWaiting && (
          <span className="live-pill" data-state="waiting" title="A question or permission waits for your answer in the panel">
            answer needed
          </span>
        )}
        <span className="count">{caption}</span>
      </button>
      {!open && <Questions states={states} />}
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
  const [loaded, setLoaded] = useState(false) // true once a snapshot has arrived: before that the counts are unknown, not 0
  useEffect(() => {
    if (typeof EventSource === 'undefined') return // no stream in this browser: the page keeps the "connecting" state
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
      setLoaded(true)
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
  return [state, link, loaded]
}

/** The page's one stream, when App holds it: `{ state, link, loaded, reconnect }`. Null outside a LiveProvider. */
export const LiveContext = createContext(null)

/** Opens the one EventSource on /api/live for the whole page, so the top bar chips and the Live view read the same stream (T068). */
export function LiveProvider({ children }) {
  const [attempt, setAttempt] = useState(0)
  const [state, link, loaded] = useLive(attempt)
  const reconnect = useCallback(() => setAttempt((n) => n + 1), [])
  const value = useMemo(() => ({ state, link, loaded, reconnect }), [state, link, loaded, reconnect])
  return <LiveContext.Provider value={value}>{children}</LiveContext.Provider>
}

/** `// work` › Live: every session, app and terminal, as rows with three bars; the selected one opens one panel beside the list (FR-128). Inside a
 * LiveProvider it shows the page's stream; alone it opens its own. `appSessions`: the server has app sessions, so they are listed with the rest. */
export default function LiveView(props) {
  const shared = useContext(LiveContext)
  return shared ? <Live {...shared} {...props} /> : <OwnStream {...props} />
}

const APPS_FAST_MS = 1000 // an app session's panel is open: its own stream is read again every second (FR-110)
const APPS_IDLE_MS = 5000 // otherwise the list of app sessions is read this often

/**
 * The app sessions as the server has them (T104, FR-109): read at once, then every second while an app session's panel is open and every 5 s
 * otherwise. A failed read keeps the last answer and sets `stale`. `null` until the first read answered, and for a server with no such route.
 */
function useApps(enabled, fast) {
  const [apps, setApps] = useState(null)
  const [stale, setStale] = useState(false)
  const last = useRef(null)
  const generation = useRef(0) // bumped by every answer of our own: a read that started before it is older than the state and is dropped
  const [again, setAgain] = useState(0) // bumped by `reload`: the effect below starts over and reads at once (T128: a session was just started)
  useEffect(() => {
    if (!enabled) return undefined
    let current = true
    let out = false
    const read = async () => {
      if (out) return
      out = true
      try {
        const seen = generation.current
        const list = await getAppSessions()
        if (!current || !list || seen !== generation.current) return
        setStale(false)
        const body = JSON.stringify(list.sessions)
        if (body !== last.current) {
          last.current = body
          setApps(list.sessions)
        }
      } catch {
        if (current) setStale(true)
      } finally {
        out = false
      }
    }
    read()
    const timer = setInterval(read, fast ? APPS_FAST_MS : APPS_IDLE_MS)
    return () => {
      current = false
      clearInterval(timer)
    }
  }, [enabled, fast, again])
  // An answer to our own request: take that session now, and let the next read replace the list.
  const replace = useCallback((view) => {
    generation.current += 1
    last.current = null
    setApps((list) => (list ?? []).some((a) => a.id === view.id) ? list.map((a) => (a.id === view.id ? view : a)) : [...(list ?? []), view])
  }, [])
  const remove = useCallback((id) => {
    generation.current += 1
    last.current = null
    setApps((list) => (list ?? []).filter((a) => a.id !== id))
  }, [])
  const reload = useCallback(() => setAgain((n) => n + 1), [])
  return { apps, stale, replace, remove, reload }
}

/**
 * The terminal sessions as the server has them (T134, FR-136): read at once and every 5 s. The panel's own socket tells it the rest; `reload` reads at once
 * (a session was just started, ended or stopped). A failed read keeps the last answer. `null` until the first read answered, and for a server with no
 * such route.
 */
function useTerms(enabled) {
  const [terms, setTerms] = useState(null)
  const last = useRef(null)
  const [again, setAgain] = useState(0)
  useEffect(() => {
    if (!enabled) return undefined
    let current = true
    const read = async () => {
      try {
        const list = await getTermSessions()
        if (!current || !list) return
        const body = JSON.stringify(list)
        if (body !== last.current) {
          last.current = body
          setTerms(list)
        }
      } catch {
        // keep what the page has; the next read tries again
      }
    }
    read()
    const timer = setInterval(read, APPS_IDLE_MS)
    return () => {
      current = false
      clearInterval(timer)
    }
  }, [enabled, again])
  const reload = useCallback(() => setAgain((n) => n + 1), [])
  return { terms, reload }
}

/** The row of a terminal session the live tail does not list yet (its process file is not there yet, or it ended): built from its own view. */
function termOnly(t, seenAt) {
  const base = fromSnapshot({ session_id: typeof t.session_id === 'string' ? t.session_id : null, account: accountLabel(t.account), project: folderName(t.repo) || 'terminal', last_ts: seenAt, tool_calls: 0, tokens_in: 0, tokens_out: 0, cost_usd: 0 })
  return { ...base, appOnly: true, appState: isExited(t) ? endedWords(t.code) : 'Running' }
}

const appCard = (a) => a.items.some((i) => i.kind === 'prompt' && i.status === 'pending')

/** The row of an app session the live tail does not list yet (it is starting, or it wrote nothing in the window): built from its own view. */
function appOnly(a, seenAt) {
  const base = fromSnapshot({ session_id: typeof a.session_id === 'string' ? a.session_id : null, account: a.account?.label ?? 'Default Claude Code account', project: folderName(a.folder) || 'session', last_ts: seenAt, tool_calls: 0, tokens_in: 0, tokens_out: 0, cost_usd: 0 })
  return { ...base, appOnly: true, appState: STATES[a.state] ?? '' }
}

function OwnStream(props) {
  const [attempt, setAttempt] = useState(0)
  const [state, link] = useLive(attempt)
  const reconnect = useCallback(() => setAttempt((n) => n + 1), [])
  return <Live state={state} link={link} reconnect={reconnect} {...props} />
}

function Live({ state, link, reconnect, wantedId = null, onWantedUsed, appSessions = false, wantedApp = null, onWantedAppUsed, onResumeInApp, onOpenSettings, target = null, onTargetUsed, onOpenAppSession }) {
  const [now, setNow] = useState(Date.now())
  const [openId, setOpenId] = useState(null) // the panel's key: a live session id, or `app:<id>` for an app session; not kept across a reload
  const openRef = useRef(null)
  openRef.current = openId
  const [opened, setOpened] = useState(0) // T121 (F10): the panel's key; it changes only when a session is opened, never when its row key changes
  const [focusPanel, setFocusPanel] = useState(false) // the panel was opened by the owner: the focus moves to its heading
  const [backTo, setBackTo] = useState(null) // the panel was closed: the focus goes back to this row
  const quietFocus = useRef(false)
  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), TICK_MS)
    return () => clearInterval(timer)
  }, [])
  const [revealId, setRevealId] = useState(null) // the session picked from the address: the one row that scrolls into view
  const [view, setView] = useState('live') // `live`: the list of running sessions; `past`: the sessions of the store (T119, FR-128)
  const [filters, setFilters] = useState(loadFilters) // state, project and kind of the list: kept in localStorage, those three values only
  const filtersRef = useRef(filters)
  const changeFilters = useCallback((patch) => {
    const next = { ...filtersRef.current, ...patch }
    filtersRef.current = next
    setFilters(next)
    saveFilters(next)
  }, [])
  const [renamed, setRenamed] = useState({}) // session id -> the name the server just accepted (null: removed), until the snapshot says the same (FR-127)
  const onRenamed = useCallback((id, name) => setRenamed((old) => ({ ...old, [id]: name })), [])
  const onToggle = useCallback(
    (key) => {
      if (openRef.current === key) {
        setOpenId(null)
        setBackTo(key)
      } else {
        setOpenId(key)
        setOpened((n) => n + 1)
        setFocusPanel(true)
      }
      setRevealId(null)
      onWantedUsed?.() // the owner's own choice ends the wish: a wanted session listed later does not take the selection
    },
    [onWantedUsed],
  )
  useEffect(() => {
    if (backTo === null) return
    quietFocus.current = true // our own focus move: it does not hold the list order (T121 round 2)
    document.getElementById(`live-row-${backTo}`)?.focus()
    quietFocus.current = false
    setBackTo(null)
  }, [backTo])
  const sessions = useMemo(() => Object.values(state.sessions), [state.sessions])
  const selectedApp = openId !== null && openId.startsWith('app:')
  const { apps, stale, replace, remove, reload } = useApps(appSessions, selectedApp)
  const { terms, reload: reloadTerms } = useTerms(appSessions)
  // The New session sheet (T128, FR-133): `{workspace}`, `{resume}` or `{continueIn}` for NewSession, drawn in place above the list, never as a dialog (FR-128).
  const [form, setForm] = useState(null)
  const [formKey, setFormKey] = useState(0) // each opening is a new form: its fields are read once
  const openForm = useCallback((next) => {
    setForm(next)
    setFormKey((n) => n + 1)
  }, [])
  useEffect(() => {
    // What another page asked for (Workspaces: New session / Continue, a panel's Resume / Fork): the sheet opens with it, once.
    if (!target) return
    openForm({ workspace: target.newFor, resume: target.resume, continueIn: target.continueIn })
    onTargetUsed?.()
  }, [target, openForm, onTargetUsed])
  const seen = useRef({}) // app session id -> when this page first saw it
  const cache = useRef(new Map()) // app session id -> its row, kept while its words are the same, so its memoized row does not draw again
  const entries = useMemo(() => {
    const list = apps ?? []
    const byClaudeId = new Map(list.filter((a) => typeof a.session_id === 'string').map((a) => [a.session_id, a]))
    const used = new Set()
    const termByClaudeId = new Map((terms ?? []).filter((t) => typeof t.session_id === 'string').map((t) => [t.session_id, t]))
    const usedTerms = new Set()
    const out = sessions.map((live) => {
      const s = live.session_id in renamed ? { ...live, name: renamed[live.session_id] } : live
      const a = byClaudeId.get(s.session_id)
      if (a) used.add(a.id)
      const t = a ? undefined : termByClaudeId.get(s.session_id) // a terminal session the app started: the live row is its row
      if (t) usedTerms.add(t.id)
      return { s, key: a ? `app:${a.id}` : t ? `term:${t.id}` : s.session_id, app: a ?? null, term: t ?? null, card: a ? appCard(a) : false }
    })
    for (const a of list) {
      if (used.has(a.id)) continue
      const sig = JSON.stringify([a.account?.label, a.folder, a.state, a.session_id])
      let row = cache.current.get(a.id)
      if (!row || row.sig !== sig) {
        seen.current[a.id] ??= new Date().toISOString()
        row = { sig, s: appOnly(a, seen.current[a.id]) }
        cache.current.set(a.id, row)
      }
      const s = typeof a.session_id === 'string' && a.session_id in renamed ? { ...row.s, name: renamed[a.session_id] } : row.s
      out.push({ s, key: `app:${a.id}`, app: a, term: null, card: appCard(a) })
    }
    for (const t of terms ?? []) {
      if (usedTerms.has(t.id)) continue
      const key = `term:${t.id}`
      const sig = JSON.stringify([accountLabel(t.account), t.repo, t.state, t.code, t.session_id])
      let row = cache.current.get(key)
      if (!row || row.sig !== sig) {
        seen.current[key] ??= new Date().toISOString()
        row = { sig, s: termOnly(t, seen.current[key]) }
        cache.current.set(key, row)
      }
      out.push({ s: row.s, key, app: null, term: t, card: false })
    }
    return out
  }, [sessions, apps, terms, renamed])
  useEffect(() => {
    // A name the snapshot now carries is no longer held here: the stream is the page's one source again. A name held for a session that is in neither
    // the stream nor the app session list is let go too, so it cannot come back with the session after the server changed it.
    setRenamed((old) => {
      const byId = new Map(sessions.map((x) => [x.session_id, x.name]))
      const appIds = new Set((apps ?? []).map((a) => a.session_id))
      const keep = Object.entries(old).filter(([id, name]) => (byId.has(id) ? byId.get(id) !== name : appIds.has(id)))
      return keep.length === Object.keys(old).length ? old : Object.fromEntries(keep)
    })
  }, [sessions, apps])
  const kept = useRef(null) // T121 (F6): the open session as last listed, so its panel stays when the session leaves the list (FR-072)
  const last = kept.current && (kept.current.key === openId || kept.current.s.session_id === openId) ? kept.current : null
  const listed = openId === null ? null : (entries.find((e) => e.key === openId || e.s.session_id === openId || (last?.s.session_id && e.s.session_id === last.s.session_id)) ?? null)
  if (listed) kept.current = listed
  const gone = openId !== null && !listed && last !== null // it left the list: the panel says so, read-only, until Back
  const selected = listed ?? (gone ? last : null)
  useEffect(() => {
    if (openId !== null && !selected && (apps !== null || !selectedApp)) setOpenId(null) // never listed while open: nothing to draw
    else if (selected && selected.key !== openId) setOpenId(selected.key) // a row opened before the app list answered: its key became `app:<id>`; the open id follows, so Back closes it and the list is read each second
  }, [openId, selected, apps, selectedApp])
  useEffect(() => {
    // The session the app asked for (the page's start address): select it once, as soon as it is in the list. The page gives up after 10 s.
    const hit = wantedId && entries.find((e) => e.s.session_id === wantedId)
    if (hit) {
      setView('live')
      setOpenId(hit.key)
      setOpened((n) => n + 1)
      setRevealId(hit.key)
      setFocusPanel(false)
      onWantedUsed?.()
    }
  }, [wantedId, entries, onWantedUsed])
  useEffect(() => {
    // The app session just started from the New session sheet: its panel opens here, once it is in the list.
    const wantedKey = wantedApp && (wantedApp.startsWith('term:') ? wantedApp : `app:${wantedApp}`) // a started terminal session comes as `term:<id>`
    if (wantedKey && entries.some((e) => e.key === wantedKey)) {
      setView('live')
      setOpenId(wantedKey)
      setOpened((n) => n + 1)
      setFocusPanel(true)
      onWantedAppUsed?.()
    }
  }, [wantedApp, entries, onWantedAppUsed])
  const facts = useMemo(() => entries.map((e) => ({ state: statusOf(e, now, state.windowS), project: e.s.project ?? '', kind: kindOf(e.s, e.app) })), [entries, now, state.windowS])
  const openKey = selected?.key ?? null
  // T121 (F7, F8): while the pointer or the focus is on the rows, the rows already shown keep their place and stay listed whatever the filters say.
  const [hold, setHold] = useState({ pointer: false, focus: false })
  const holding = hold.pointer || hold.focus
  const shownIds = useRef([])
  const groups = useMemo(() => {
    const held = holding ? new Map(shownIds.current.map((id, i) => [id, i])) : null
    return group(entries.filter((e, i) => e.key === openKey || held?.has(rowId(e)) || passes(facts[i], filters)), held) // the open session's row stays listed, whatever the filters say
  }, [entries, facts, filters, openKey, holding])
  shownIds.current = groups.flatMap((g) => g.entries.map(rowId))
  const rowsBox = useRef(null)
  useEffect(() => {
    // A focused row that left the page sends no blur: let go of the hold once the focus is no longer on the rows (T121 round 2).
    if (hold.focus && !rowsBox.current?.contains(document.activeElement)) setHold((h) => ({ ...h, focus: false }))
  }, [groups, hold.focus])
  const shown = groups.reduce((n, g) => n + g.entries.length, 0)
  const [notices, endPrompt] = useNotices(sessions)
  const needing = entries.filter(asksYou).length // the rule of the *Needs me* filter: a state, a prompt in the panel, or a prompt card
  const all = entries.map((e) => e.s)
  const maxTokens = Math.max(0, ...all.map((s) => s.tokens_in + s.tokens_out))
  const maxCost = Math.max(0, ...all.map((s) => s.cost))
  const maxCalls = Math.max(0, ...all.map((s) => s.tool_calls))
  const closePanel = useCallback(() => selected && onToggle(selected.key), [selected, onToggle])
  const onClosed = useCallback(
    (id) => {
      remove(id)
      if (openRef.current === `app:${id}`) setOpenId(null) // the owner closed it: the panel goes, it does not stay as "no longer active"
    },
    [remove],
  )
  return (
    <main className="live">
      <div className="live-head">
        <h1>Live</h1>
        <div className="periods" role="group" aria-label="View">
          <button type="button" aria-pressed={view === 'live'} onClick={() => setView('live')}>
            Live
          </button>
          <button type="button" aria-pressed={view === 'past'} onClick={() => setView('past')}>
            Past
          </button>
        </div>
        {appSessions && apps !== null && (
          <button type="button" className="primary" disabled={form !== null} onClick={() => openForm({})}>
            New session
          </button>
        )}
        <p role="status" className="live-need">
          {needing === 0 ? '' : needing === 1 ? '1 session needs you' : `${needing} sessions need you`}
        </p>
      </div>
      {form && (
        <NewSessionSheet
          key={formKey}
          form={form}
          onStarted={(id, kind) => {
            setForm(null)
            setView('live')
            reload() // the new session is listed now, not at the next read
            if (kind === 'terminal') reloadTerms()
            onOpenAppSession?.(kind === 'terminal' ? `term:${id}` : id)
          }}
          onCancel={() => setForm(null)}
          onOpenSettings={onOpenSettings}
        />
      )}
      {view === 'past' && <PastSessions filters={filters} onFilters={changeFilters} onResume={appSessions && apps !== null ? onResumeInApp : undefined} />}
      {view === 'live' && (
        <>
      <div className="live-status">
        <p role="status">{STATUS[link]}</p>
        {link === 'closed' && (
          <button type="button" onClick={reconnect}>
            Reconnect
          </button>
        )}
      </div>
      <section className="card" aria-labelledby="h-live-all">
        <h2 id="h-live-all">All sessions</h2>
        <Totals total={sum(all)} />
        <details className="live-about">
          <summary>What these numbers and this page show</summary>
          <p className="footnote">
          API-equivalent cost is tokens times the API price table, not what a subscription pays. The list shows only tool names, skill, sub-agent, MCP and command names, counts and times, and the question a waiting session asks. Opening a session shows its conversation: your messages, Claude's text and each tool call with its input and result, read from Claude Code's own file while the panel is open and never stored.
          </p>
        </details>
      </section>
      {entries.length === 0 && (
        <p className="muted">{`No active sessions. A session appears here when it wrote a record in the last ${Math.round(state.windowS / 60)} minutes.`}</p>
      )}
      {(entries.length > 0 || selected) && (
        <div className="live-split" data-open={selected ? 'true' : undefined}>
          <div className="live-sessions">
            <FilterBar facts={facts} filters={filters} onChange={changeFilters} />
            {shown === 0 && (
              <p role="status" className="muted">
                No sessions match these filters.
              </p>
            )}
            <div
              className="live-rows"
              onPointerEnter={() => setHold((h) => ({ ...h, pointer: true }))}
              onPointerLeave={() => setHold((h) => ({ ...h, pointer: false }))}
              ref={rowsBox}
              onFocus={(e) => {
                if (quietFocus.current || !keyboardFocus(e.target)) return // only the keyboard's focus holds the order; a click or Back does not
                setHold((h) => (h.focus ? h : { ...h, focus: true }))
              }}
              onBlur={(e) => {
                if (!e.currentTarget.contains(e.relatedTarget)) setHold((h) => ({ ...h, focus: false }))
              }}
            >
            {groups.map((a, i) => (
              <section key={a.account} aria-labelledby={`live-acc-${i}`}>
                <h2 id={`live-acc-${i}`}>{a.account}</h2>
                <ul className="live-list">
                  {a.entries.map((e) => (
                    <SessionRow
                      key={rowId(e)}
                      s={e.s}
                      k={e.key}
                      now={now}
                      windowS={state.windowS}
                      open={e.key === selected?.key}
                      reveal={e.key === revealId}
                      card={e.card}
                      onToggle={onToggle}
                      tokensPct={share(e.s.tokens_in + e.s.tokens_out, maxTokens)}
                      costPct={share(e.s.cost, maxCost)}
                      callsPct={share(e.s.tool_calls, maxCalls)}
                    />
                  ))}
                </ul>
              </section>
            ))}
            </div>
          </div>
          {selected && (
            <div className="live-panel-col">
              {stale && selected.app && <p role="status" className="muted">Cannot reach the app right now. Showing the last answer.</p>}
              {selected.term ? (
                <Suspense fallback={<Loading label="Loading the terminal" />}>
                  <TermPanel key={opened} term={selected.term} onBack={closePanel} onChanged={reloadTerms} autoFocus={focusPanel} />
                </Suspense>
              ) : (
              <SessionPanel
                key={opened}
                s={selected.s}
                gone={gone}
                app={selected.app}
                now={now}
                notice={notices[selected.s.session_id]}
                lead={selected.s.pending === null ? <Questions states={statesOf(selected.s)} /> : null}
                fallback={<Feed items={selected.s.feed} />}
                onChange={replace}
                onClosed={onClosed}
                onEnd={endPrompt}
                onResume={appSessions && apps !== null ? onResumeInApp : undefined}
                onOpenSettings={onOpenSettings}
                onRenamed={onRenamed}
                onBack={closePanel}
                autoFocus={focusPanel}
              />
              )}
            </div>
          )}
        </div>
      )}
        </>
      )}
    </main>
  )
}
