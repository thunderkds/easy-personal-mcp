import { useCallback, useEffect, useRef, useState } from 'react'
import { answerAppPrompt, closeAppSession, getAgentRun, getAppSessions, getJson, interruptAppSession, sendAppMessage, stopAppSession } from './api.js'
import Loading from './Loading.jsx'
import NewSession from './NewSession.jsx'
import PromptCard, { readPrompt } from './PromptCard.jsx'
import { useLoad } from './useLoad.js'

// The app session page (T104, FR-109..FR-111, FR-115, FR-116): sessions the app started with the owner's `claude`, the conversation as it
// streams, and the prompt cards. The text is React text, never HTML. It is held in React state only: nothing is written to `localStorage`,
// a URL or the console, and the server drops it when an ended session is closed. The page sends only the form's fields, a message, an answer
// or an id (FR-116).
export const POLL_MS = 1000
export const OFF = 'App sessions are off. Choose an agent run account in Settings.'
export const NO_CLAUDE = 'Claude Code was not found on this machine, so no session can start.'
const FAILED = 'That did not go through. Try again.'
const STATES = {
  working: 'Working',
  idle: 'Waiting for your message',
  waiting: 'Waiting for you: answer the prompt below',
  stopping: 'Stopping',
  ended: 'Ended',
}
const SETTLED = { answered: 'answered', cancelled: 'closed (the turn or the session ended)' }
const NEAR_BOTTOM = 48 // px: a log scrolled further up than this is not pulled down by a new item

const folderName = (folder) => String(folder ?? '').split('/').filter(Boolean).pop() ?? String(folder ?? '')
const isEnded = (s) => s.state === 'ended'
/** The server's own words for a refused request (400, 404, 409), else the fallback. */
const why = (error, fallback = FAILED) => ((error.status === 400 || error.status === 404 || error.status === 409) && error.message ? error.message : fallback)

/** The call label as one line of text: a name, or "server · tool" for an MCP call; anything else gives none. */
function labelText(label) {
  if (typeof label === 'string') return label
  if (label && typeof label === 'object' && typeof label.server === 'string' && typeof label.tool === 'string') return `${label.server} · ${label.tool}`
  return ''
}

/** The answer fields of a prompt card as the route takes them: a question's answers by its text, else allow or deny with the note. */
export function answerBody(prompt, fields) {
  if (prompt.kind !== 'question') return { behavior: fields.decision, ...(fields.message ? { message: fields.message } : {}) }
  const answers = {}
  prompt.questions.forEach((q, i) => {
    const { selected, other } = fields.answers[i]
    answers[q.question] = [...selected.map((p) => q.options[p].label), ...(other ? [other] : [])].join(', ')
  })
  return { behavior: 'allow', updatedInput: { answers } }
}

/** One pending prompt card and the route behind it. Give it `key={prompt.id}`: a new prompt starts with a clean card and no error. */
function AppPrompt({ sessionId, prompt, onChange }) {
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  const open = useRef(false) // one request at a time: a second click while one is out is dropped
  const send = async (fields) => {
    if (open.current) return
    open.current = true
    setBusy(true)
    setError('')
    try {
      onChange(await answerAppPrompt(sessionId, prompt.id, answerBody(prompt, fields)))
    } catch (e) {
      setError(why(e, 'Could not send the answer. Try again.'))
    } finally {
      open.current = false
      setBusy(false)
    }
  }
  return <PromptCard prompt={prompt} busy={busy} error={error} onAnswer={send} />
}

/** One item of the conversation. */
function Turn({ item, sessionId, onChange }) {
  if (item.kind === 'user' || item.kind === 'assistant')
    return (
      <li className={`turn turn-${item.kind}`}>
        <span className="turn-who">{item.kind === 'user' ? 'You' : 'Claude'}</span>
        <p className="turn-text">{typeof item.text === 'string' ? item.text : ''}</p>
      </li>
    )
  if (item.kind === 'tool') {
    const label = labelText(item.label)
    return (
      <li className="turn turn-tool">
        <span className="live-label">{[typeof item.name === 'string' ? item.name : 'a tool', label].filter(Boolean).join(' · ')}</span>
        <span className="live-tag" data-outcome={item.outcome}>
          {typeof item.outcome === 'string' ? item.outcome : ''}
        </span>
      </li>
    )
  }
  if (item.kind !== 'prompt') return null
  if (item.status === 'pending') {
    const prompt = readPrompt(item)
    return (
      <li className="turn turn-prompt">
        {prompt ? <AppPrompt key={prompt.id} sessionId={sessionId} prompt={prompt} onChange={onChange} /> : <p className="prompt-cut">A prompt this page cannot show is waiting. Stop the session to end it.</p>}
      </li>
    )
  }
  const what = { question: 'Question', plan: 'Plan', tool: 'Permission request' }[item.type] ?? 'Prompt'
  return (
    <li className="turn turn-tool">
      <span className="muted">{`${what} · ${SETTLED[item.status] ?? 'closed'}`}</span>
    </li>
  )
}

/** The message box: Enter sends, Shift+Enter makes a new line. Text that starts with `/` goes as typed (a slash command). */
function MessageBox({ session, onChange }) {
  const [text, setText] = useState('')
  const [error, setError] = useState('')
  const [busy, setBusy] = useState(false)
  const open = useRef(false)
  const off = isEnded(session) || session.state === 'stopping'
  const send = async () => {
    if (open.current || off || text.trim() === '') return
    open.current = true
    setBusy(true)
    setError('')
    try {
      onChange(await sendAppMessage(session.id, text))
      setText('')
    } catch (e) {
      setError(why(e, 'Could not send the message. Try again.'))
    } finally {
      open.current = false
      setBusy(false)
    }
  }
  return (
    <div className="message-box">
      <label className="field" htmlFor={`message-${session.id}`}>
        Message to Claude
        <textarea
          id={`message-${session.id}`}
          rows={3}
          value={text}
          disabled={off}
          onChange={(e) => setText(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter' && !e.shiftKey && !e.nativeEvent.isComposing) {
              e.preventDefault()
              send()
            }
          }}
        />
      </label>
      <p className="muted">Enter sends, Shift+Enter adds a line. Text that starts with / is sent as typed, so slash commands work.</p>
      {error && (
        <p role="alert" className="field-error">
          {error}
        </p>
      )}
      <div className="actions">
        <button type="button" className="primary" disabled={off || busy || text.trim() === ''} onClick={send}>
          Send
        </button>
      </div>
    </div>
  )
}

/** The session panel: header, the conversation, the message box, Interrupt and Stop. `onChange(view)`: the session as the server now has it. */
function SessionPanel({ session: s, onChange, onClosed }) {
  const [error, setError] = useState('')
  const [busy, setBusy] = useState(false)
  const open = useRef(false)
  const log = useRef(null)
  const stick = useRef(true) // a log that is at the bottom follows new items; one scrolled up stays where the owner left it
  const ended = isEnded(s)
  useEffect(() => {
    const el = log.current
    if (el && stick.current) el.scrollTop = el.scrollHeight
  }, [s.items])
  const act = async (request, done) => {
    if (open.current) return
    open.current = true
    setBusy(true)
    setError('')
    try {
      done(await request())
    } catch (e) {
      setError(why(e))
    } finally {
      open.current = false
      setBusy(false)
    }
  }
  const state = ended ? `${STATES.ended}${s.reason ? `: ${s.reason}` : ''}` : (STATES[s.state] ?? '')
  return (
    <section className="card session-panel" aria-label={`Session in ${folderName(s.folder)}`}>
      <dl className="session-facts">
        <dt>Session</dt>
        <dd className="mono">{s.session_id ?? 'starting'}</dd>
        <dt>Account</dt>
        <dd>{s.account?.label ?? 'Default Claude Code account'}</dd>
        <dt>Model</dt>
        <dd>{s.model}</dd>
        <dt>Mode</dt>
        <dd>{s.mode}</dd>
        <dt>Folder</dt>
        <dd className="mono">{s.folder}</dd>
        {s.worktree && (
          <>
            <dt>Branch</dt>
            <dd className="mono">{s.branch ?? 'not created'}</dd>
            <dt>Changed files</dt>
            <dd>
              {Array.isArray(s.changed_files) && s.changed_files.length > 0 ? (
                <ul className="changed-files">
                  {s.changed_files.map((name) => (
                    <li key={name} className="mono">
                      {name}
                    </li>
                  ))}
                </ul>
              ) : (
                'none yet'
              )}
            </dd>
          </>
        )}
      </dl>
      <p role="status" className="session-state" data-state={s.state}>
        {state}
      </p>
      <div className="actions">
        <button type="button" disabled={busy || ended || s.state === 'stopping' || s.state === 'idle'} onClick={() => act(() => interruptAppSession(s.id), onChange)}>
          Interrupt
        </button>
        <button type="button" className="danger" disabled={busy || ended || s.state === 'stopping'} onClick={() => act(() => stopAppSession(s.id), onChange)}>
          Stop
        </button>
        {ended && (
          <button type="button" disabled={busy} onClick={() => act(() => closeAppSession(s.id), () => onClosed(s.id))}>
            Close
          </button>
        )}
      </div>
      {error && (
        <p role="alert" className="field-error">
          {error}
        </p>
      )}
      <ol
        ref={log}
        role="log"
        aria-label="Conversation"
        className="session-log"
        tabIndex={0}
        onScroll={(e) => {
          const el = e.currentTarget
          stick.current = el.scrollHeight - el.scrollTop - el.clientHeight < NEAR_BOTTOM
        }}
      >
        {s.items.length === 0 && <li className="muted">Nothing yet.</li>}
        {s.items.map((item, i) => (
          <Turn key={item.kind === 'prompt' && item.prompt_id ? `p-${item.prompt_id}` : i} item={item} sessionId={s.id} onChange={onChange} />
        ))}
      </ol>
      {ended && <p className="muted">The session has ended. Close it to drop its text from this page.</p>}
      <MessageBox session={s} onChange={onChange} />
    </section>
  )
}

/** The sessions as the server has them, read again every second (no push stream). A failed read keeps the last answer and sets `stale`. */
function usePolled(initial) {
  const [data, setData] = useState(initial)
  const [stale, setStale] = useState(false)
  const last = useRef(JSON.stringify(initial))
  useEffect(() => {
    let current = true
    let out = false
    const tick = async () => {
      if (out) return
      out = true
      try {
        const next = await getAppSessions()
        if (!current || !next) return
        setStale(false)
        const text = JSON.stringify(next)
        if (text !== last.current) {
          last.current = text
          setData(next)
        }
      } catch {
        if (current) setStale(true)
      } finally {
        out = false
      }
    }
    const timer = setInterval(tick, POLL_MS)
    return () => {
      current = false
      clearInterval(timer)
    }
  }, [])
  // An answer to our own request: take that session now, and let the next read replace the list.
  const replace = useCallback((view) => {
    last.current = null
    setData((d) => (d.sessions.some((s) => s.id === view.id) ? { ...d, sessions: d.sessions.map((s) => (s.id === view.id ? view : s)) } : { ...d, sessions: [...d.sessions, view] }))
  }, [])
  const remove = useCallback((id) => {
    last.current = null
    setData((d) => ({ ...d, sessions: d.sessions.filter((s) => s.id !== id) }))
  }, [])
  return { data, stale, replace, remove }
}

function Loaded({ initial, agentRun, workspaces, accounts, target, onTargetUsed, onOpenSettings }) {
  const { data, stale, replace, remove } = usePolled(initial)
  const [form, setForm] = useState(() => (target?.newFor !== undefined ? { workspace: target.newFor } : null))
  const [picked, setPicked] = useState(target?.open ?? null)
  useEffect(() => {
    if (target) onTargetUsed?.()
  }, [])
  const sessions = data.sessions
  const selected = sessions.find((s) => s.id === picked) ?? sessions[0] ?? null
  const going = sessions.filter((s) => !isEnded(s)).length
  const full = going >= data.max
  const off = !agentRun.enabled
  const noClaude = agentRun.claude_found === false
  const defaultAccount = agentRun.account ? String(agentRun.account.id) : ''
  const started = (id) => {
    setForm(null)
    setPicked(id)
    getAppSessions().then(
      (next) => {
        const mine = next?.sessions.find((s) => s.id === id)
        if (mine) replace(mine)
      },
      () => {},
    )
  }
  return (
    <>
      {off && (
        <p className="muted">
          {OFF}{' '}
          {onOpenSettings && (
            <button type="button" className="link-button" onClick={onOpenSettings}>
              Open Settings
            </button>
          )}
        </p>
      )}
      {!off && noClaude && <p role="alert" className="error">{NO_CLAUDE}</p>}
      {stale && <p role="status" className="muted">Cannot reach the app right now. Showing the last answer.</p>}
      <div className="actions">
        <button type="button" className="primary" disabled={off || noClaude || full || form !== null} onClick={() => setForm({ workspace: undefined })}>
          New session
        </button>
        {full && <span className="muted">{`${data.max} sessions are going; stop one first.`}</span>}
      </div>
      {form && !off && !noClaude && (
        <NewSession workspaces={workspaces} accounts={accounts} defaultAccount={defaultAccount} initialWorkspace={form.workspace} onStarted={started} onCancel={() => setForm(null)} />
      )}
      {sessions.length === 0 && !form && <p className="muted">No app sessions. Start one with New session.</p>}
      {sessions.length > 0 && (
        <nav aria-label="App sessions">
          <ul className="session-tabs">
            {sessions.map((s) => (
              <li key={s.id}>
                <button type="button" aria-current={selected?.id === s.id ? 'true' : undefined} data-state={s.state} onClick={() => setPicked(s.id)}>
                  <span className="session-tab-name">{folderName(s.folder) || 'session'}</span>
                  <span className="session-tab-state">{STATES[s.state] ? s.state : ''}</span>
                </button>
              </li>
            ))}
          </ul>
        </nav>
      )}
      {selected && <SessionPanel key={selected.id} session={selected} onChange={replace} onClosed={remove} />}
    </>
  )
}

const load = () => Promise.all([getAppSessions(), getAgentRun(), getJson('/api/workspaces'), getJson('/api/accounts?by=account')])

/**
 * The page. `target`: `{newFor: <workspace name>}` opens the form for that workspace (Workspaces page), `{open: <app session id>}` selects
 * that session (Live view); `onTargetUsed` is called once it was read.
 */
export default function AppSessions({ target = null, onTargetUsed, onOpenSettings }) {
  const state = useLoad(load, [])
  const [list, agentRun, workspaces, accounts] = state.status === 'ready' ? state.data : []
  return (
    <main className="app-sessions">
      <h1>App sessions</h1>
      <p className="muted">
        A session here is Claude Code on this machine, started and answered from this page. What you type and what Claude answers is shown here and kept in memory only: it is dropped when you close an ended session and when the app exits.
      </p>
      {state.status === 'loading' && <Loading label="Loading app sessions" />}
      {state.status === 'error' && (
        <p role="alert" className="error">
          Could not load app sessions: {state.message}
        </p>
      )}
      {state.status === 'ready' && (list === null || agentRun === null) && <p className="muted">App sessions are not available in this version of the app.</p>}
      {state.status === 'ready' && list && agentRun && (
        <Loaded
          initial={list}
          agentRun={agentRun}
          workspaces={workspaces.workspaces}
          accounts={accounts.accounts.filter((a) => a.provider === 'claude' && a.status === 'confirmed')}
          target={target}
          onTargetUsed={onTargetUsed}
          onOpenSettings={onOpenSettings}
        />
      )}
    </main>
  )
}
