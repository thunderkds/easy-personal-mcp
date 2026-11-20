import { useCallback, useEffect, useRef, useState } from 'react'
import { getPastSessions } from './api.js'
import { why } from './Composer.jsx'
import { ago, localTime } from './format.js'
import { isProjectFilter } from './LiveFilters.jsx'
import SessionName, { ID_CHARS, titleOf } from './SessionName.jsx'
import Conversation from './SubAgents.jsx'

// The Past switch of the Live view (T119, FR-126, FR-128): sessions from the store's metadata, newest first, 50 a page, with the filters of the Live list that
// fit a past session (project, kind), a date range and Load more. Opening one shows its conversation, read-only, from the same route as a live one (FR-123);
// Resume and Fork start an app session as the Live panel does (FR-112). Names, ids and times are React text. Nothing here is kept in `localStorage`
// but the two shared filters, which LiveView keeps.
export const PAST_KINDS = ['terminal', 'headless'] // the kinds the store records; an app session is stored as a headless run
export const ALL_TIME_FROM = '2000-01-01' // *Show older*: an explicit `from` that lifts the server's 30-day window (T117 debate)
export const WINDOW_NOTE = 'Showing the last 30 days.'
const TYPING_MS = 300 // the project box asks the server this long after the last key

const DAY_MS = 86_400_000
/** The day after a `YYYY-MM-DD` date: the route's `to` is the start of its day, and the owner means the picked day to be in. */
const nextDay = (day) => new Date(Date.parse(`${day}T00:00:00Z`) + DAY_MS).toISOString().slice(0, 10)
const isDay = (v) => /^\d{4}-\d{2}-\d{2}$/.test(v) && !Number.isNaN(Date.parse(`${v}T00:00:00Z`))

const text = (v) => (typeof v === 'string' && v !== '' ? v : null)
/** One session of a page, as far as this page uses it; an entry with no usable id is dropped. */
const readSession = (x) => (x && typeof x === 'object' && typeof x.id === 'string' && x.id !== '' ? { id: x.id, project: text(x.project) ?? '', branch: text(x.branch), first: text(x.first), last: text(x.last), kind: text(x.kind), account: text(x.account) ?? '', model: text(x.model), name: text(x.name) } : null)
const readPage = (page) =>
  page && typeof page === 'object' && Array.isArray(page.sessions)
    ? { sessions: page.sessions.map(readSession).filter(Boolean), next: text(page.next), window: page.window && typeof page.window === 'object' ? page.window : null }
    : null

const BAD_SHAPE = 'The list came in a shape this page cannot read.'
const LOADING = { status: 'loading', sessions: [], next: null, window: null, error: '', more: false, moreError: '' }

/** The pages of the list for one set of query values: the first on any change, the next with the cursor. A reply that belongs to an older query is dropped. */
function usePast(query) {
  const key = JSON.stringify(query)
  const [state, setState] = useState(LOADING)
  const generation = useRef(0)
  const queryRef = useRef(query)
  queryRef.current = query
  const latest = useRef(state)
  latest.current = state
  useEffect(() => {
    const mine = ++generation.current
    if (queryRef.current === null) return setState({ ...LOADING, status: 'ready' }) // no query to ask (the dates are the wrong way round)
    setState((old) => ({ ...old, status: 'loading', next: null, error: '', more: false, moreError: '' })) // the rows stay while the next list loads: an open panel is not torn down
    getPastSessions(queryRef.current)
      .then((raw) => {
        if (mine !== generation.current) return
        const page = readPage(raw)
        setState(page ? { ...LOADING, status: 'ready', ...page } : { ...LOADING, status: 'error', error: BAD_SHAPE })
      })
      .catch((e) => {
        if (mine === generation.current) setState({ ...LOADING, status: 'error', error: why(e, 'The past sessions cannot be listed. Try again.') })
      })
  }, [key])
  const loadMore = useCallback(async () => {
    const mine = generation.current
    const { next: before, more } = latest.current
    if (before === null || more || queryRef.current === null) return
    setState((s) => ({ ...s, more: true, moreError: '' }))
    try {
      const page = readPage(await getPastSessions({ ...queryRef.current, before }))
      if (mine !== generation.current) return
      if (!page) throw new Error(BAD_SHAPE)
      setState((s) => {
        const seen = new Set(s.sessions.map((x) => x.id))
        return { ...s, sessions: [...s.sessions, ...page.sessions.filter((x) => !seen.has(x.id))], next: page.next, more: false, moreError: '' }
      })
    } catch (e) {
      if (mine === generation.current) setState((s) => ({ ...s, more: false, moreError: why(e, 'Could not load more. Try again.') }))
    }
  }, [])
  const rename = useCallback((id, name) => setState((s) => ({ ...s, sessions: s.sessions.map((x) => (x.id === id ? { ...x, name } : x)) })), [])
  return [state, loadMore, rename]
}

/** One row: the short id, the name (else the project), the branch, the kind and when it last wrote. */
function PastRow({ s, open, onToggle }) {
  return (
    <li className="live-session" data-open={open || undefined}>
      <button type="button" id={`past-row-${s.id}`} className="live-row" aria-expanded={open} aria-current={open ? 'true' : undefined} aria-controls="past-panel" onClick={() => onToggle(s.id)}>
        <span className="live-id" title={s.id}>
          {s.id.slice(0, ID_CHARS)}
        </span>
        <span className="live-name" title={s.project}>
          {s.name ?? (s.project || 'session')}
        </span>
        {s.branch && <span className="live-stage">{s.branch}</span>}
        <span className="count">{`${s.kind ?? 'unknown kind'} · ${s.last ? ago(s.last) : 'no time'}`}</span>
      </button>
    </li>
  )
}

/** The read-only panel of one past session: facts, Resume / Fork, and its conversation. Esc and Back close it. */
function PastPanel({ s, onBack, onResume, onRenamed, autoFocus }) {
  const heading = useRef(null)
  useEffect(() => {
    if (autoFocus) heading.current?.focus()
  }, [autoFocus])
  const title = titleOf({ name: s.name, project: s.project, id: s.id })
  const start = (fork) => onResume({ id: s.id, project: s.project, account: s.account, fork })
  return (
    <section
      id="past-panel"
      className="card session-panel"
      aria-label={`Past session in ${s.project || 'a project'}`}
      onKeyDown={(e) => {
        if (e.key !== 'Escape' || e.nativeEvent.isComposing) return
        if (typeof e.target.value === 'string' && e.target.value !== '') return
        onBack()
      }}
    >
      <div className="session-head">
        <button type="button" className="session-back" onClick={onBack}>
          Back to sessions
        </button>
        <SessionName id={s.id} name={s.name} title={title} headingRef={heading} onSaved={(name) => onRenamed(s.id, name)} />
      </div>
      <dl className="session-facts">
        <dt>Project</dt>
        <dd>{s.project || 'unknown'}</dd>
        <dt>Kind</dt>
        <dd>{s.kind ?? 'unknown'}</dd>
        <dt>Session</dt>
        <dd className="mono">{s.id}</dd>
        <dt>Account</dt>
        <dd>{s.account || 'unknown'}</dd>
        {s.branch && (
          <>
            <dt>Branch</dt>
            <dd className="mono">{s.branch}</dd>
          </>
        )}
        {s.model && (
          <>
            <dt>Model</dt>
            <dd>{s.model}</dd>
          </>
        )}
        <dt>First record</dt>
        <dd>{s.first ? localTime(s.first) : 'unknown'}</dd>
        <dt>Last record</dt>
        <dd>{s.last ? localTime(s.last) : 'unknown'}</dd>
      </dl>
      <p className="session-state" data-state="idle">
        Past session: read-only
      </p>
      {onResume && (
        <div className="actions">
          <button type="button" onClick={() => start(false)}>
            Resume in app
          </button>
          <button type="button" onClick={() => start(true)}>
            Fork in app
          </button>
        </div>
      )}
      <Conversation key={s.id} sessionId={s.id} />
    </section>
  )
}

/**
 * `filters`: the Live view's `{state, project, kind}`; the project and kind of it apply here (a state is a live thing). `onFilters(patch)` changes them.
 * `onResume({id, project, account, fork})`: start an app session in the app's own flow; none when the server has no app sessions.
 */
export default function PastSessions({ filters, onFilters, onResume }) {
  const [typed, setTyped] = useState(filters.project)
  const [from, setFrom] = useState('')
  const [to, setTo] = useState('')
  const [older, setOlder] = useState(false)
  const [openId, setOpenId] = useState(null)
  const [focusPanel, setFocusPanel] = useState(false)
  const [backTo, setBackTo] = useState(null)
  useEffect(() => {
    const clean = typed.trim()
    if (clean === filters.project || !isProjectFilter(clean)) return undefined
    const timer = setTimeout(() => onFilters({ project: clean }), TYPING_MS)
    return () => clearTimeout(timer)
  }, [typed, filters.project, onFilters])
  const kind = PAST_KINDS.includes(filters.kind) ? filters.kind : '' // the kind asked of the store; the Live list's `app` is no kind here
  const backwards = isDay(from) && isDay(to) && from > to
  const query = { project: filters.project, kind, from: isDay(from) ? from : older ? ALL_TIME_FROM : '', to: isDay(to) ? nextDay(to) : '' }
  const [list, loadMore, rename] = usePast(backwards ? null : query)
  const listed = openId === null ? null : (list.sessions.find((x) => x.id === openId) ?? null)
  const held = useRef(null) // the open session as last listed: the panel stays up while the next list loads or fails
  if (listed) held.current = listed
  const selected = listed ?? (openId !== null && list.status !== 'ready' && held.current?.id === openId ? held.current : null)
  useEffect(() => {
    if (openId !== null && list.status === 'ready' && !listed) setOpenId(null) // the filters moved on: a list that came back has not the open session
  }, [openId, listed, list.status])
  useEffect(() => {
    if (focusPanel) setFocusPanel(false) // the panel has taken the focus once (its own effect ran first): a later draw of it never takes it again
  }, [focusPanel])
  useEffect(() => {
    if (backTo === null) return
    document.getElementById(`past-row-${backTo}`)?.focus()
    setBackTo(null)
  }, [backTo])
  const onToggle = useCallback((id) => {
    setOpenId((now) => {
      if (now === id) {
        setBackTo(id)
        return null
      }
      setFocusPanel(true)
      return id
    })
  }, [])
  const filtered = filters.project !== '' || filters.kind !== '' || from !== '' || to !== '' || older
  const clear = () => {
    setTyped('')
    setFrom('')
    setTo('')
    setOlder(false)
    onFilters({ project: '', kind: '' })
  }
  return (
    <div className="past" aria-label="Past sessions" role="region">
      <div className="live-split" data-open={selected ? 'true' : undefined}>
        <div className="live-sessions">
          <div className="live-filters" role="group" aria-label="Past filters">
            <label className="board-filter">
              Project
              <input type="text" value={typed} maxLength={255} placeholder="folder name" onChange={(e) => setTyped(e.target.value)} />
            </label>
            <label className="board-filter">
              Kind
              <select value={filters.kind === 'app' ? 'app' : kind} onChange={(e) => onFilters({ kind: e.target.value })}>
                <option value="">All kinds</option>
                {filters.kind === 'app' && <option value="app">App (kept from the Live list; no filter here)</option>}
                {PAST_KINDS.map((k) => (
                  <option key={k} value={k}>
                    {k}
                  </option>
                ))}
              </select>
            </label>
            <label className="board-filter">
              From
              <input type="date" value={from} onChange={(e) => setFrom(e.target.value)} />
            </label>
            <label className="board-filter">
              To
              <input type="date" value={to} onChange={(e) => setTo(e.target.value)} />
            </label>
            {filtered && (
              <button type="button" className="link-button" onClick={clear}>
                Clear filters
              </button>
            )}
          </div>
          {backwards && (
            <p role="alert" className="field-error">
              From is after To. Change one of the dates.
            </p>
          )}
          {!backwards && list.status === 'ready' && list.window?.default === true && (
            <p role="status" className="muted past-window">
              {to !== '' && isDay(to) ? `Showing the 30 days up to and including ${to}.` : WINDOW_NOTE}{' '}
              <button type="button" className="link-button" onClick={() => setOlder(true)}>
                Show older
              </button>
            </p>
          )}
          {!backwards && older && from === '' && (
            <p role="status" className="muted past-window">
              {`Showing sessions since ${ALL_TIME_FROM}.`}{' '}
              <button type="button" className="link-button" onClick={() => setOlder(false)}>
                Last 30 days
              </button>
            </p>
          )}
          {!backwards && list.status === 'loading' && (
            <p role="status" className="muted">
              Loading past sessions
            </p>
          )}
          {!backwards && list.status === 'error' && (
            <p role="alert" className="field-error">
              {list.error}
            </p>
          )}
          {!backwards && list.status === 'ready' && list.sessions.length === 0 && <p className="muted">No past sessions match.</p>}
          {list.sessions.length > 0 && (
            <ul className="live-list" aria-label="Past sessions, newest first">
              {list.sessions.map((s) => (
                <PastRow key={s.id} s={s} open={s.id === selected?.id} onToggle={onToggle} />
              ))}
            </ul>
          )}
          {list.next !== null && (
            <button type="button" disabled={list.more} onClick={loadMore}>
              Load more
            </button>
          )}
          {list.moreError && (
            <p role="alert" className="field-error">
              {list.moreError}
            </p>
          )}
        </div>
        {selected && (
          <div className="live-panel-col">
            <PastPanel key={selected.id} s={selected} onBack={() => onToggle(selected.id)} onResume={onResume} onRenamed={rename} autoFocus={focusPanel} />
          </div>
        )}
      </div>
    </div>
  )
}
