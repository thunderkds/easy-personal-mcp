import { memo, useCallback, useEffect, useRef, useState } from 'react'
import { getTranscript } from './api.js'

// The conversation of one open session (T118, FR-123, FR-128): read from the server on demand, every 2 s while this component is on the page, and
// held in React state only. Nothing of it goes to `localStorage`, a URL or the console (DDR-0007). Every text is React text, never HTML.
export const TRANSCRIPT_POLL_MS = 2000
export const CUT_NOTE = 'Cut at 16,000 characters'
const NEAR_BOTTOM = 48 // px: a log scrolled further up than this is not pulled down by a new entry

const isName = (v) => typeof v === 'string' && v !== ''
const textOf = (v) => (typeof v === 'string' ? v : '')

/** The call label as one line: a name, or "server · tool" for an MCP call; any other shape gives none. */
function labelText(label) {
  if (isName(label)) return label
  if (label && typeof label === 'object' && isName(label.server) && isName(label.tool)) return `${label.server} · ${label.tool}`
  return ''
}

const keep = (a, b) => (b === null || b === undefined ? a : b)
/** A tool entry merged with the one already listed under its id, field by field: a null never replaces a value (a result may come before its call, or alone). */
function joinTool(old, e) {
  const out = { ...old, ...e }
  for (const f of ['name', 'label', 'input', 'result']) out[f] = keep(old[f], e[f])
  out.orphan = old.orphan === true && e.orphan === true // the call was seen: no longer an orphan
  out.cut = old.cut === true || e.cut === true
  out.is_error = old.is_error === true || e.is_error === true
  // The entry sits where its call is: the smaller offset of the two, so a trim at it (and Load earlier from it) never starts after the call.
  const offsets = [old.offset, e.offset].filter((o) => typeof o === 'number')
  if (offsets.length > 0) out.offset = Math.min(...offsets)
  return out
}

/** New entries into the list. A tool entry whose id is already listed is merged in place, in either order of call and result. */
function merge(old, incoming, tag, front) {
  if (incoming.length === 0) return old
  const list = [...old]
  const fresh = []
  let at = null // tool id -> index in `list`, built once and only when a tool entry comes
  const byId = new Map() // tool id -> index in `fresh`
  for (const e of incoming) {
    if (e.kind !== 'tool' || !isName(e.id)) {
      fresh.push({ ...e, k: tag() })
      continue
    }
    if (at === null) {
      at = new Map()
      list.forEach((x, i) => x.kind === 'tool' && isName(x.id) && at.set(x.id, i))
    }
    if (at.has(e.id)) {
      const i = at.get(e.id)
      list[i] = { ...joinTool(list[i], e), k: list[i].k }
    } else if (byId.has(e.id)) {
      const i = byId.get(e.id)
      fresh[i] = { ...joinTool(fresh[i], e), k: fresh[i].k }
    } else {
      byId.set(e.id, fresh.length)
      fresh.push({ ...e, k: tag() })
    }
  }
  return front ? [...fresh, ...list] : [...list, ...fresh]
}

export const MAX_KEPT = 2000 // the newest entries kept on the page; Load earlier reaches the rest from `start`
/** The newest MAX_KEPT entries, cut at an entry that says its offset, and that offset: the new `start`. Without one the list is kept whole. */
function trim(list) {
  if (list.length <= MAX_KEPT) return null
  for (let i = list.length - MAX_KEPT; i < list.length; i += 1) if (typeof list[i].offset === 'number') return { list: list.slice(i), start: list[i].offset }
  return null
}

/** Where the page starts in the file: the answer's `start`, else the first entry's `offset`; null when neither is a number. */
const startOf = (page) => (typeof page.start === 'number' ? page.start : typeof page.entries[0]?.offset === 'number' ? page.entries[0].offset : null)
const readPage = (page) => (page && typeof page === 'object' && Array.isArray(page.entries) ? page : null)

function useTranscript(sessionId) {
  const [state, setState] = useState({ status: 'loading', entries: [], start: null, message: '', stale: false, earlier: false })
  const next = useRef(null) // the offset to poll from
  const counter = useRef(0)
  const tag = () => ++counter.current
  const earlierOut = useRef(false)
  const startRef = useRef(null)
  const listRef = useRef([]) // the entries as last drawn
  const loadedEarlier = useRef(false) // the owner asked for older entries: they are not trimmed away behind them
  useEffect(() => {
    let current = true
    let out = false
    let timer = null
    const fail = (e) => {
      if (e?.status === 404 && timer !== null) clearInterval(timer) // the file is gone: asking again changes nothing; a network error is retried
      setState((s) => (s.status === 'ready' ? { ...s, stale: true } : { ...s, status: 'error', message: e?.message || 'The conversation cannot be read.' }))
    }
    const tick = async () => {
      if (out) return
      out = true
      try {
        const first = next.current === null
        const page = readPage(await getTranscript(sessionId, first ? {} : { after: next.current }))
        if (!current) return
        if (!page) return fail(new Error('The conversation came in a shape this page cannot read.'))
        if (typeof page.next === 'number') next.current = page.next
        if (first) startRef.current = startOf(page)
        if (!first && page.entries.length === 0) {
          // nothing new: the drawn state stays as it is (no new array, no new draw); only a stale note is cleared
          setState((s) => (s.status === 'ready' && !s.stale ? s : { ...s, status: 'ready', message: '', stale: false }))
          return
        }
        let list = first ? merge([], page.entries, tag, false) : merge(listRef.current, page.entries, tag, false)
        const cut = loadedEarlier.current ? null : trim(list)
        if (cut) {
          list = cut.list
          startRef.current = cut.start
        }
        listRef.current = list
        setState((s) => ({ status: 'ready', message: '', stale: false, entries: list, start: startRef.current, earlier: s.earlier }))
      } catch (e) {
        if (current) fail(e)
      } finally {
        out = false
      }
    }
    tick()
    timer = setInterval(tick, TRANSCRIPT_POLL_MS)
    return () => {
      current = false
      clearInterval(timer)
    }
  }, [sessionId])
  /** *Load earlier*: the entries before the first one shown. Offered only when the server said where the page starts. */
  const loadEarlier = useCallback(async () => {
    if (earlierOut.current || startRef.current === null) return
    earlierOut.current = true
    setState((s) => ({ ...s, earlier: true }))
    try {
      const page = readPage(await getTranscript(sessionId, { before: startRef.current }))
      if (!page) throw new Error('The earlier part came in a shape this page cannot read.')
      const s = startOf(page)
      startRef.current = s !== null && s < startRef.current ? s : null
      loadedEarlier.current = true
      listRef.current = merge(listRef.current, page.entries, tag, true)
      setState((old) => ({ ...old, entries: listRef.current, start: startRef.current, earlier: false, stale: false }))
    } catch {
      setState((s) => ({ ...s, earlier: false, stale: true }))
    } finally {
      earlierOut.current = false
    }
  }, [sessionId])
  return [state, loadEarlier]
}

/** A text block of one tool call: pre-wrapped React text that scrolls inside itself. */
function Block({ text, label }) {
  return (
    <pre className="turn-pre" tabIndex={0} aria-label={label}>
      {text}
    </pre>
  )
}

const TODO_MARK = { pending: ['○', 'pending'], in_progress: ['◐', 'in progress'], completed: ['●', 'done'] }

/** The todos of a `TodoWrite` input (the server's pretty JSON text), or null when the text is anything else: the caller then shows the JSON (FR-133). */
function readTodos(name, input) {
  if (name !== 'TodoWrite' || typeof input !== 'string') return null
  try {
    const todos = JSON.parse(input)?.todos
    if (!Array.isArray(todos)) return null
    const ok = todos.every((t) => t && typeof t === 'object' && isName(t.content) && Object.hasOwn(TODO_MARK, t.status))
    return ok ? todos.map((t) => ({ content: t.content, status: t.status })) : null
  } catch {
    return null // cut or escaped text is not JSON
  }
}

/** One tool call: folded to its label; a click opens its input and its result. A TodoWrite call with a readable list shows it as a checklist, always. */
function Tool({ e }) {
  const [open, setOpen] = useState(false)
  const todos = readTodos(e.name, e.input)
  const orphan = e.orphan === true && !isName(e.name)
  const label = orphan ? 'result of an earlier call' : [isName(e.name) ? e.name : 'a tool', labelText(e.label)].filter(Boolean).join(' · ')
  const input = textOf(e.input)
  const result = textOf(e.result)
  return (
    <li className="turn turn-tool-call" data-error={e.is_error === true || undefined}>
      <button type="button" className="turn-fold" aria-expanded={open} onClick={() => setOpen((o) => !o)}>
        <span className="live-arrow" aria-hidden="true">
          {open ? '▾' : '▸'}
        </span>
        <span className="live-label">{label}</span>
        {e.is_error === true && <span className="live-tag error">error</span>}
      </button>
      {todos && (
        <ul className="turn-todos" aria-label="Todo list">
          {todos.length === 0 && <li className="muted">The list is empty.</li>}
          {todos.map((t, i) => (
            <li key={i} data-status={t.status}>
              <span className="turn-todo-mark" role="img" aria-label={TODO_MARK[t.status][1]}>
                {TODO_MARK[t.status][0]}
              </span>
              <span className="turn-todo-text">{t.content}</span>
            </li>
          ))}
        </ul>
      )}
      {open && (
        <div className="turn-detail">
          {!todos && (
            <>
              <span className="turn-who">Input</span>
              {input ? <Block text={input} label={`Input of ${label}`} /> : <span className="muted">No input.</span>}
            </>
          )}
          <span className="turn-who">Result</span>
          {result ? <Block text={result} label={`Result of ${label}`} /> : <span className="muted">{e.result === null || e.result === undefined ? 'No result yet.' : 'Empty result.'}</span>}
          {e.cut === true && <span className="muted turn-cut">{CUT_NOTE}</span>}
        </div>
      )}
    </li>
  )
}

const WHO = { user: 'You', reply: 'Reply from the app', assistant: 'Claude' }

const Entry = memo(function Entry({ e }) {
  if (e.kind === 'user' || e.kind === 'reply' || e.kind === 'assistant')
    return (
      <li className={`turn turn-${e.kind}`}>
        <span className="turn-who">{WHO[e.kind]}</span>
        <p className="turn-text">{textOf(e.text)}</p>
        {e.cut === true && <span className="muted turn-cut">{CUT_NOTE}</span>}
      </li>
    )
  if (e.kind === 'tool') return <Tool e={e} />
  if (e.kind === 'skipped') {
    const n = Number.isInteger(e.count) && e.count > 0 ? e.count : 1
    return <li className="turn muted turn-skipped">{n === 1 ? '1 other record' : `${n} other records`}</li>
  }
  return null // a kind this page does not know shows nothing
})

/**
 * The log of one session. `children` is drawn while the conversation loads and when it cannot be read (a gone file, an older server): the panel gives the tool
 * calls seen since the page opened. Give it `key={sessionId}`.
 */
export default function Transcript({ sessionId, children }) {
  const [t, loadEarlier] = useTranscript(sessionId)
  const log = useRef(null)
  const stick = useRef(true) // a log at the bottom follows new entries; one scrolled up stays where the owner left it
  const last = t.entries.at(-1)?.k
  useEffect(() => {
    const el = log.current
    if (el && stick.current) el.scrollTop = el.scrollHeight
  }, [last])
  if (t.status === 'loading')
    return (
      <>
        <p role="status" className="muted">Loading the conversation</p>
        {children}
      </>
    )
  if (t.status === 'error')
    return (
      <>
        <p role="status" className="muted">{`The conversation cannot be shown: ${t.message}`}</p>
        {children}
      </>
    )
  return (
    <>
      {t.stale && <p role="status" className="muted">Cannot refresh the conversation right now. Showing the last answer.</p>}
      <ol
        ref={log}
        role="log"
        aria-label="Conversation"
        className="session-log"
        tabIndex={0}
        onScroll={(ev) => {
          const el = ev.currentTarget
          stick.current = el.scrollHeight - el.scrollTop - el.clientHeight < NEAR_BOTTOM
        }}
      >
        {t.start !== null && t.start > 0 && (
          <li className="turn-earlier">
            <button type="button" disabled={t.earlier} onClick={loadEarlier}>
              Load earlier
            </button>
          </li>
        )}
        {t.entries.length === 0 && <li className="muted">Nothing yet.</li>}
        {t.entries.map((e) => (
          <Entry key={e.k} e={e} />
        ))}
      </ol>
    </>
  )
}
