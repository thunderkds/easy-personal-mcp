import { useEffect, useRef, useState } from 'react'
import { answerAppPrompt, closeAppSession, interruptAppSession, stopAppSession, stopSession } from './api.js'
import Composer, { why } from './Composer.jsx'
import { ago } from './format.js'
import { kindOf } from './LiveFilters.jsx'
import { FORK_NOTE } from './NewSession.jsx'
import { PendingAnswer } from './PendingAnswer.jsx'
import PromptCard, { readPrompt } from './PromptCard.jsx'
import SessionName, { titleOf } from './SessionName.jsx'
import Transcript from './Transcript.jsx'

// The one panel of the Live view (T118, FR-128): header, the conversation, the cards pinned above the composer, the composer and the controls,
// for an app session and for a terminal session alike. The text is React text, never HTML; it is held in React state only: nothing is written to
// `localStorage`, a URL or the console, and an app session's text is dropped on the server when an ended session is closed (FR-115, FR-116).
export const STATES = {
  working: 'Working',
  idle: 'Waiting for your message',
  waiting: 'Waiting for you: answer the prompt below',
  stopping: 'Stopping',
  ended: 'Ended',
}
const SETTLED = { answered: 'answered', cancelled: 'closed (the turn or the session ended)', pending: 'waiting for you, below the conversation' }
export const STOP_SENT = 'Asked the session to stop. Claude Code ends it when it can.'
const ID_CHARS = 8

const isEnded = (a) => a.state === 'ended'

/** The call label as one line of text: a name, or "server · tool" for an MCP call; anything else gives none. */
function labelText(label) {
  if (typeof label === 'string') return label
  if (label && typeof label === 'object' && typeof label.server === 'string' && typeof label.tool === 'string') return `${label.server} · ${label.tool}`
  return ''
}

/** The answer fields of a prompt card as the route takes them: a question's answers by its text, else allow or deny with the note. */
export function answerBody(prompt, fields) {
  if (prompt.kind !== 'question')
    return {
      behavior: fields.decision,
      ...(fields.message ? { message: fields.message } : {}),
      ...(Number.isInteger(fields.suggestion) ? { suggestion: fields.suggestion } : {}), // FR-132: the index of a suggestion the card showed
      ...(fields.accept_edits === true ? { accept_edits: true } : {}),
    }
  const answers = {}
  prompt.questions.forEach((q, i) => {
    const { selected, other } = fields.answers[i]
    answers[q.question] = [...selected.map((p) => q.options[p].label), ...(other ? [other] : [])].join(', ')
  })
  return { behavior: 'allow', updatedInput: { answers } }
}

/** One pending prompt card of an app session and the route behind it. Give it `key={prompt.id}`: a new prompt starts with a clean card and no error. */
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

/** A pending prompt of an app session, drawn in normal page flow above the composer (the log is a scroll box: the whole input must be in view). */
function PendingPrompt({ item, sessionId, onChange }) {
  const prompt = readPrompt(item)
  return prompt ? <AppPrompt key={prompt.id} sessionId={sessionId} prompt={prompt} onChange={onChange} /> : <p className="prompt-cut">A prompt this page cannot show is waiting. Stop the session to end it.</p>
}

/** One item of an app session's own conversation (shown before its Claude session id is known). A pending prompt is one line here. */
function Turn({ item }) {
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
  const what = { question: 'Question', plan: 'Plan', tool: 'Permission request' }[item.type] ?? 'Prompt'
  return (
    <li className="turn turn-tool">
      <span className="muted">{`${what} · ${SETTLED[item.status] ?? 'closed'}`}</span>
    </li>
  )
}

export const STILL_RUNNING = 'Asked to stop, still running.'
export const FORCE_WAIT_MS = 5000 // a Stop that answered 200 and a session still listed after this long: Force stop is offered
const FORCE_NOTE = 'Force stop ends it at once and may leave the last transcript line half written.'

/**
 * Stop of a terminal session: one confirm that names the project and the short id, then the server's checks (SIGTERM). If the session is still
 * listed 5 s after a Stop that was accepted, it says so and offers Force stop, with its own confirm. A refusal shows the server's one line.
 * When the session leaves the list the panel says it is no longer active and Stop goes, which ends the wait (T121).
 */
function StopTerminal({ s }) {
  const [step, setStep] = useState('idle') // idle | confirm | sent | still | force
  const [error, setError] = useState('')
  const [busy, setBusy] = useState(false)
  const out = useRef(false)
  const id = s.session_id.slice(0, ID_CHARS)
  useEffect(() => {
    if (step !== 'sent') return undefined
    const timer = setTimeout(() => setStep('still'), FORCE_WAIT_MS)
    return () => clearTimeout(timer)
  }, [step])
  const go = async (force) => {
    if (out.current) return
    out.current = true
    setBusy(true)
    setError('')
    try {
      await stopSession(s.session_id, { force })
      setStep('sent')
    } catch (e) {
      setStep(force ? 'force' : 'idle')
      setError(why(e, 'Could not ask the session to stop. Try again.'))
    } finally {
      out.current = false
      setBusy(false)
    }
  }
  return (
    <>
      {step === 'confirm' && (
        <div role="group" aria-label="Confirm stop" className="stop-confirm">
          <p className="prompt-text">{`Stop session ${id} in ${s.project}? Claude Code is asked to end it (SIGTERM); work not yet saved there is lost.`}</p>
          <div className="actions">
            <button type="button" className="danger" disabled={busy} onClick={() => go(false)}>
              {`Stop session ${id}`}
            </button>
            <button type="button" disabled={busy} onClick={() => setStep('idle')}>
              Keep it running
            </button>
          </div>
        </div>
      )}
      {step === 'force' && (
        <div role="group" aria-label="Confirm force stop" className="stop-confirm">
          <p className="prompt-text">{`Force stop session ${id} in ${s.project}? ${FORCE_NOTE}`}</p>
          <div className="actions">
            <button type="button" className="danger" disabled={busy} onClick={() => go(true)}>
              {`Force stop session ${id}`}
            </button>
            <button type="button" disabled={busy} onClick={() => setStep('still')}>
              Keep waiting
            </button>
          </div>
        </div>
      )}
      {(step === 'idle' || step === 'sent') && (
        <button type="button" className="danger" disabled={step === 'sent'} onClick={() => setStep('confirm')}>
          Stop
        </button>
      )}
      {step === 'still' && (
        <button type="button" className="danger" onClick={() => setStep('force')}>
          Force stop
        </button>
      )}
      {step === 'sent' && (
        <p role="status" className="muted">
          {STOP_SENT}
        </p>
      )}
      {step === 'still' && (
        <p role="status" className="muted">
          {STILL_RUNNING}
        </p>
      )}
      {error && (
        <p role="alert" className="field-error">
          {error}
        </p>
      )}
    </>
  )
}

/** The three controls of an app session. `onChange(view)`: the session as the server now has it. */
function AppControls({ app, onChange, onClosed }) {
  const [error, setError] = useState('')
  const [busy, setBusy] = useState(false)
  const open = useRef(false)
  const ended = isEnded(app)
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
  return (
    <>
      <button type="button" disabled={busy || ended || app.state === 'stopping' || app.state === 'idle'} onClick={() => act(() => interruptAppSession(app.id), onChange)}>
        Interrupt
      </button>
      <button type="button" className="danger" disabled={busy || ended || app.state === 'stopping'} onClick={() => act(() => stopAppSession(app.id), onChange)}>
        Stop
      </button>
      {ended && (
        <button type="button" disabled={busy} onClick={() => act(() => closeAppSession(app.id), () => onClosed(app.id))}>
          Close
        </button>
      )}
      {error && (
        <p role="alert" className="field-error">
          {error}
        </p>
      )}
    </>
  )
}

export const folderName = (folder) => String(folder ?? '').split('/').filter(Boolean).pop() ?? String(folder ?? '')

/**
 * `s`: the live session (an app session with no live row yet is built from its own view, `s.appOnly`). `app`: that app session's view, else null.
 * `now`: the clock the times are worked out against. `fallback`: drawn when the conversation cannot be read. `lead`: drawn above the conversation.
 * `onChange(view)` and `onClosed(id)` take an app session's answers; `onEnd` is the terminal prompt's; `onResume({id, project, account, fork})`.
 * `onRenamed(id, name)`: the server's answer to a rename (the owner's name, or null when it was removed).
 * `onBack` closes the panel (Esc and the Back button); the row takes the focus back. `autoFocus`: move the focus to the heading when the panel opens.
 * `gone` (T121, FR-072): the session left the live list; the panel stays with what it last had, says so, and drops its controls and composer.
 */
export default function SessionPanel({ s, app, now, notice, fallback, lead, onChange, onClosed, onEnd, onResume, onOpenSettings, onRenamed, onBack, autoFocus = false, gone = false }) {
  const heading = useRef(null)
  useEffect(() => {
    if (autoFocus) heading.current?.focus()
  }, [autoFocus])
  const pending = app ? app.items.filter((item) => item.kind === 'prompt' && item.status === 'pending') : []
  const project = s.appOnly ? folderName(app?.folder) || 'session' : s.project
  const id = s.session_id ?? app?.session_id ?? null
  const title = titleOf({ name: s.name, claudeName: s.proc?.name, project, id }) // the owner's name, else Claude's, else the project and short id (FR-127)
  const ended = app ? isEnded(app) : false
  const appState = app ? (ended ? `${STATES.ended}${app.reason ? `: ${app.reason}` : ''}` : (STATES[app.state] ?? '')) : ''
  const termState = s.proc ? (s.proc.busy ? 'busy' : 'idle') : ''
  const branch = app ? (app.worktree ? (app.branch ?? 'not created') : null) : typeof s.branch === 'string' && s.branch ? s.branch : null
  const fork = !app && s.last_ts ? now - Date.parse(s.last_ts) < 120_000 : false // the server's resume check: a record in the last 2 minutes
  return (
    <section
      id="live-panel"
      className="card session-panel"
      aria-label={`Session in ${project}`}
      onKeyDown={(e) => {
        if (e.key !== 'Escape' || e.nativeEvent.isComposing) return
        if (typeof e.target.value === 'string' && e.target.value !== '') return // text in a box: Esc does not throw it away
        onBack()
      }}
    >
      <div className="session-head">
        <button type="button" className="session-back" onClick={onBack}>
          Back to sessions
        </button>
        <SessionName id={id} name={s.name} title={title} headingRef={heading} onSaved={(name) => id && onRenamed?.(id, name)} />
      </div>
      <dl className="session-facts">
        <dt>Project</dt>
        <dd>{project}</dd>
        <dt>Kind</dt>
        <dd>{kindOf(s, app)}</dd>
        <dt>Session</dt>
        <dd className="mono">{id ?? 'starting'}</dd>
        <dt>Account</dt>
        <dd>{app ? (app.account?.label ?? 'Default Claude Code account') : s.account}</dd>
        {branch !== null && (
          <>
            <dt>Branch</dt>
            <dd className="mono">{branch}</dd>
          </>
        )}
        {app && (
          <>
            <dt>Model</dt>
            <dd>{app.model}</dd>
            <dt>Mode</dt>
            <dd>{app.mode}</dd>
            <dt>Folder</dt>
            <dd className="mono">{app.folder}</dd>
            {app.worktree && (
              <>
                <dt>Changed files</dt>
                <dd>
                  {Array.isArray(app.changed_files) && app.changed_files.length > 0 ? (
                    <ul className="changed-files">
                      {app.changed_files.map((name) => (
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
          </>
        )}
        {!s.appOnly && (
          <>
            <dt>Last record</dt>
            <dd>{ago(s.last_ts, now)}</dd>
          </>
        )}
      </dl>
      {gone && (
        <p role="status" className="session-state session-gone">
          This session is no longer active: it left the live list. Go back to sessions to close this panel.
        </p>
      )}
      {!gone && (app ? appState : termState) && (
        <p role="status" className="session-state" data-state={app ? app.state : s.proc?.busy ? 'working' : 'idle'}>
          {app ? appState : termState}
        </p>
      )}
      <div className="actions">
        {app && !gone && <AppControls app={app} onChange={onChange} onClosed={onClosed} />}
        {!app && id && !gone && <StopTerminal s={s} />}
        {!app && id && onResume && (
          <button type="button" onClick={() => onResume({ id, project: s.project, account: s.account, fork })}>
            {fork ? 'Fork in app' : 'Resume in app'}
          </button>
        )}
      </div>
      {!app && id && onResume && fork && <p className="muted live-fork-note">{FORK_NOTE}</p>}
      {lead}
      {id ? (
        <Transcript key={id} sessionId={id}>
          {fallback}
        </Transcript>
      ) : (
        <ol role="log" aria-label="Conversation" className="session-log" tabIndex={0}>
          {(app?.items ?? []).length === 0 && <li className="muted">The conversation shows when Claude starts the session.</li>}
          {(app?.items ?? []).map((item, i) => (
            <Turn key={item.kind === 'prompt' && item.prompt_id ? `p-${item.prompt_id}` : i} item={item} />
          ))}
        </ol>
      )}
      {!gone && (s.pending || pending.length > 0 || notice) && (
        <div className="session-prompts">
          {s.pending && <PendingAnswer key={s.pending.id} sessionId={s.session_id} prompt={s.pending} onEnd={onEnd} />}
          {pending.map((item, i) => (
            <PendingPrompt key={item.prompt_id ?? `x-${i}`} item={item} sessionId={app.id} onChange={onChange} />
          ))}
          {notice && (
            <p role="status" className="live-notice">
              {notice}
            </p>
          )}
        </div>
      )}
      {ended && <p className="muted">The session has ended. Close it to drop its text from this page.</p>}
      {!gone && <Composer s={s} app={app} onChange={onChange} onOpenSettings={onOpenSettings} />}
    </section>
  )
}
