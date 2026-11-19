import { useEffect, useLayoutEffect, useRef, useState } from 'react'
import { queueReply, sendAppMessage } from './api.js'
import { MAX_BYTES, SIZE_HINT, byteLength } from './NewSession.jsx'
import ReplyBox, { MAX_CHARS, ReplyMark } from './ReplyBox.jsx'

// The one composer of the session panel (T118, FR-128): its action follows the session. An app session sends (FR-110); a terminal session that
// holds a reply replies (FR-122, the existing ReplyBox); a busy terminal session with the bridge on queues (FR-125); with the bridge off it says
// where to turn it on. Text is React text, held in state only: never HTML, never `localStorage`.
const FAILED = 'That did not go through. Try again.'
export const BRIDGE_OFF = 'Sending to this terminal session needs the answer bridge for its account. Turn it on in Settings.'
export const QUEUE_NOTE = 'sent at the turn end'
const COUNT_FROM = 7000

/** The server's own words for a refused request (400, 404, 409), else the fallback. */
export const why = (error, fallback = FAILED) => ((error.status === 400 || error.status === 404 || error.status === 409) && error.message ? error.message : fallback)

/** Enter sends, Shift+Enter makes a new line; an Enter that ends an input method composition does not send. */
const isSend = (e) => e.key === 'Enter' && !e.shiftKey && !e.nativeEvent.isComposing && e.keyCode !== 229 // 229: Safari ends a composition with keyCode 229 and isComposing false

/** The message box of an app session: Enter sends. Text that starts with `/` goes as typed (a slash command). */
export function MessageBox({ session, onChange, text: held, onText }) {
  const [own, setOwn] = useState('')
  const text = held ?? own // T121: the composer holds the text, so a change of state does not throw it away
  const setText = onText ?? setOwn
  const [error, setError] = useState('')
  const [busy, setBusy] = useState(false)
  const open = useRef(false)
  const off = session.state === 'ended' || session.state === 'stopping'
  const tooBig = byteLength(text) > MAX_BYTES // the server's limit is in bytes (UTF-8), not characters
  const send = async () => {
    if (open.current || off || tooBig || text.trim() === '') return
    const sent = text
    open.current = true
    setBusy(true)
    setError('')
    try {
      onChange(await sendAppMessage(session.id, sent))
      // Clear only what was sent: text typed while the request was out stays.
      setText((now) => (now === sent ? '' : now.startsWith(sent) ? now.slice(sent.length) : now))
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
            if (isSend(e)) {
              e.preventDefault()
              send()
            }
          }}
        />
      </label>
      <p className="muted">Enter sends, Shift+Enter adds a line. Text that starts with / is sent as typed, so slash commands work.</p>
      {tooBig && (
        <p role="alert" className="field-error">
          {`Too long to send. ${SIZE_HINT}`}
        </p>
      )}
      {error && (
        <p role="alert" className="field-error">
          {error}
        </p>
      )}
      <div className="actions">
        <button type="button" className="primary" disabled={off || busy || tooBig || text.trim() === ''} onClick={send}>
          Send
        </button>
      </div>
    </div>
  )
}

const chars = (text) => Array.from(text).length

/**
 * The queue of a busy terminal session (FR-125): one text, kept by the app in memory until the session's next turn end. `queued` is the snapshot's
 * `{text}` or null. After a request the box shows the answer at once and goes back to the snapshot when it changes.
 */
export function QueueBox({ sessionId, queued, text: held, onText }) {
  const [own, setOwn] = useState('')
  const text = held ?? own
  const setText = onText ?? setOwn
  const [error, setError] = useState('')
  const [busy, setBusy] = useState(false)
  const [local, setLocal] = useState(undefined) // undefined: follow the snapshot; null or a text: what our last request left
  const [editing, setEditing] = useState(false)
  const out = useRef(false)
  const snap = queued?.text ?? null
  useEffect(() => {
    setLocal(undefined) // the server's snapshot is the truth again once it changes
    if (snap === null) setEditing(false) // delivered or gone: no edit of a text that is no longer queued
  }, [snap])
  const shown = local !== undefined ? local : snap
  const count = chars(text)
  const tooLong = count > MAX_CHARS
  const run = async (value, done) => {
    if (out.current) return
    out.current = true
    setBusy(true)
    setError('')
    try {
      await queueReply(sessionId, value)
      setLocal(value)
      done()
    } catch (e) {
      setError(why(e, value === null ? 'Could not cancel the queued text. Try again.' : 'Could not queue the text. Try again.')) // the text stays
    } finally {
      out.current = false
      setBusy(false)
    }
  }
  const send = () => {
    if (tooLong || text.trim() === '') return
    const typed = text
    run(typed, () => {
      setText('')
      setEditing(false)
    })
  }
  const showBox = shown === null || editing
  return (
    <div className="queue-box">
      {shown !== null && (
        <div className="queue-shown">
          <span className="turn-who">Queued</span>
          <p className="turn-text">{shown}</p>
          <p className="muted queue-note">{`${QUEUE_NOTE}; kept in memory only, lost when the app exits`}</p>
          {!editing && (
            <div className="actions">
              <button
                type="button"
                disabled={busy}
                onClick={() => {
                  setText(shown)
                  setEditing(true)
                }}
              >
                Edit
              </button>
              <button type="button" disabled={busy} onClick={() => run(null, () => setEditing(false))}>
                Cancel queued text
              </button>
            </div>
          )}
        </div>
      )}
      {showBox && (
        <>
          <label className="field" htmlFor={`queue-${sessionId}`}>
            {shown !== null ? 'Edit the queued text' : `Queue a message for session ${sessionId.slice(0, 8)}`}
            <textarea
              id={`queue-${sessionId}`}
              rows={3}
              value={text}
              onChange={(e) => setText(e.target.value)}
              onKeyDown={(e) => {
                if (isSend(e)) {
                  e.preventDefault()
                  send()
                }
              }}
            />
          </label>
          {count >= COUNT_FROM && !tooLong && <p className="muted reply-count">{`${count.toLocaleString('en-US')} / ${MAX_CHARS.toLocaleString('en-US')}`}</p>}
          {tooLong && (
            <p role="alert" className="field-error">
              {`Too long to send: ${count.toLocaleString('en-US')} / ${MAX_CHARS.toLocaleString('en-US')}`}
            </p>
          )}
          <div className="actions">
            <button type="button" className="primary" disabled={busy || tooLong || text.trim() === ''} onClick={send}>
              {shown !== null ? 'Replace queued text' : 'Queue'}
            </button>
            {editing && (
              <button type="button" disabled={busy} onClick={() => setEditing(false)}>
                Keep the queued text
              </button>
            )}
          </div>
          <p className="muted">{`Enter queues, Shift+Enter adds a line. It is ${QUEUE_NOTE}.`}</p>
        </>
      )}
      {error && (
        <p role="alert" className="field-error">
          {error}
        </p>
      )}
    </div>
  )
}

/** A row of a bridge-enabled terminal session that is not at your turn can be marked *Reply from the app* (FR-122). */
const markable = (s) => s.replyBridge && !s.pending && (s.replyMarked || !s.attention.some((a) => a.state === 'your_turn'))

/**
 * One composer for either kind. `s` is the live session, `app` the app session's view (or null), `onChange` takes the app session's answer.
 * A terminal session with no process file is taken as busy unless the server says it waits for you.
 */
export default function Composer({ s, app, onChange, onOpenSettings }) {
  // T121 (F5): the text lives here, not in the box of the moment, so working -> idle -> your turn -> working keeps what the owner typed; the focus
  // follows it into the next box (or stays on the composer while no box is shown).
  const [text, setText] = useState('')
  const wrap = useRef(null)
  const mode = useRef(null)
  const hadFocus = useRef(false)
  const yourTurn = s.attention.some((a) => a.state === 'your_turn')
  const busy = s.proc ? s.proc.busy === true : !yourTurn
  const now = app ? 'app' : s.appOnly ? 'none' : s.reply ? 'reply' : !s.replyBridge ? 'off' : busy ? 'queue' : 'idle'
  if (now !== mode.current) {
    hadFocus.current = Boolean(wrap.current && wrap.current.contains(document.activeElement)) // read before the old box leaves the page
    mode.current = now
  }
  useLayoutEffect(() => {
    if (!hadFocus.current) return
    hadFocus.current = false
    const at = document.activeElement
    if (at && at !== document.body && !wrap.current?.contains(at)) return // the owner moved on: never take the focus back
    const box = wrap.current?.querySelector('textarea:not(:disabled)')
    ;(box ?? wrap.current)?.focus()
  }, [now])
  const box = { text, onText: setText }
  let body = null
  if (app) body = <MessageBox session={app} onChange={onChange} {...box} />
  else if (s.appOnly) return null
  else {
    const mark = markable(s) ? <ReplyMark sessionId={s.session_id} marked={s.replyMarked} hold={s.reply !== null} /> : null
    if (s.reply)
      body = (
        <>
          <ReplyBox key={s.reply.id} sessionId={s.session_id} reply={s.reply} open {...box} />
          {mark}
        </>
      )
    else if (!s.replyBridge)
      body = (
        <p className="muted composer-off">
          {BRIDGE_OFF}{' '}
          {onOpenSettings && (
            <button type="button" className="link-button" onClick={onOpenSettings}>
              Open Settings
            </button>
          )}
        </p>
      )
    else
      body = (
        <>
          {busy ? (
            <QueueBox sessionId={s.session_id} queued={s.queued} {...box} />
          ) : (
            <p className="muted">
              This session is idle. A reply goes in at its next turn end: mark it below.
              {text !== '' && ' Your text is kept: it comes back in the box when the session works again or waits for your reply.'}
            </p>
          )}
          {mark}
        </>
      )
  }
  return (
    <div className="composer" ref={wrap} tabIndex={-1}>
      {body}
    </div>
  )
}
