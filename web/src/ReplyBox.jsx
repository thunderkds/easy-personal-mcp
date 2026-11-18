import { useEffect, useRef, useState } from 'react'
import { markReply, releaseReply, sendReply, watchReply } from './api.js'

// The Live row's reply box (T112, FR-122): a Stop-hook hold of a terminal session that is at your turn, with the last text, a message
// box, Send and Hand back, and the time the hold has left. The text is React text, held in state only: never HTML, never `localStorage`.
export const MAX_CHARS = 8000 // the server's limit, counted as typed (characters, not bytes)
const COUNT_FROM = 7000 // the count shows from here
export const WATCH_MS = 20_000 // how often "the owner is looking" is sent while the row is open or the box has focus
export const GONE = 'This session is no longer waiting'
export const MARKED_NOTE = 'marked: it waits for you at its next turn end'
const FAILED_SEND = 'Could not send the reply. Try again.'
const FAILED_RELEASE = 'Could not hand the session back. Try again.'
const FAILED_MARK = 'Could not change the mark. Try again.'

const chars = (text) => Array.from(text).length
/** The server's one line for a failed request, else the fallback; a 409 is the session that is gone. */
const why = (error, fallback) => (error.status === 409 ? GONE : error.message || fallback)
/** "1m 20s", "54s". */
const left = (until, now) => {
  const s = Math.max(0, Math.ceil(until - now / 1000))
  return s >= 60 ? `${Math.floor(s / 60)}m ${s % 60}s` : `${s}s`
}

/** `reply`: the snapshot's `pending_reply` `{id, until, last}`. `open`: the row is open. */
export default function ReplyBox({ sessionId, reply, open }) {
  const [text, setText] = useState('')
  const [error, setError] = useState('')
  const [busy, setBusy] = useState(false)
  const [sent, setSent] = useState(false)
  const [focused, setFocused] = useState(false)
  const [now, setNow] = useState(Date.now())
  const out = useRef(false) // one request at a time
  const count = chars(text)
  const tooLong = count > MAX_CHARS
  const watching = (open || focused) && !sent
  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), 1000)
    return () => clearInterval(timer)
  }, [])
  useEffect(() => {
    if (!watching) return undefined
    const call = () => watchReply(sessionId).catch(() => {}) // a failed call only lets the hold run out on its own
    call()
    const timer = setInterval(call, WATCH_MS)
    return () => clearInterval(timer)
  }, [watching, sessionId])
  const run = async (request, fallback, done) => {
    if (out.current) return
    out.current = true
    setBusy(true)
    setError('')
    try {
      await request()
      done()
    } catch (e) {
      setError(why(e, fallback)) // the text stays
    } finally {
      out.current = false
      setBusy(false)
    }
  }
  const send = () => {
    if (sent || tooLong || text.trim() === '') return
    const typed = text
    run(() => sendReply(reply.id, typed), FAILED_SEND, () => {
      setSent(true)
      setText('') // the box is closed after "sent": what was typed while the request was out was not sent, so it is not kept
    })
  }
  const label = `Reply to session ${sessionId.slice(0, 8)}`
  return (
    <div className="reply-box">
      {reply.last && <p className="reply-last">{reply.last}</p>}
      <label className="field" htmlFor={`reply-${sessionId}`}>
        {label}
        <textarea
          id={`reply-${sessionId}`}
          rows={3}
          value={text}
          disabled={sent}
          onChange={(e) => setText(e.target.value)}
          onFocus={() => setFocused(true)}
          onBlur={() => setFocused(false)}
          onKeyDown={(e) => {
            if (e.key === 'Enter' && !e.shiftKey && !e.nativeEvent.isComposing && e.keyCode !== 229) { // 229: Safari ends a composition with keyCode 229 and isComposing false
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
      {error && (
        <p role="alert" className="field-error">
          {error}
        </p>
      )}
      {sent && <p role="status" className="reply-sent">sent</p>}
      <div className="actions reply-actions">
        <button type="button" className="primary" disabled={busy || sent || tooLong || text.trim() === ''} onClick={send}>
          Send
        </button>
        <button type="button" disabled={busy || sent} onClick={() => run(() => releaseReply(reply.id), FAILED_RELEASE, () => {})}>
          Hand back to terminal
        </button>
        <span className="muted reply-left">{`waits ${left(reply.until, now)} more`}</span>
      </div>
      <p className="muted">Enter sends, Shift+Enter adds a line.</p>
    </div>
  )
}

/** The toggle *Reply from the app*: the state is the server's (`marked`, from the snapshot); a click asks for the other one. */
export function ReplyMark({ sessionId, marked, hold }) {
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  const out = useRef(false)
  const toggle = async () => {
    if (out.current) return
    out.current = true
    setBusy(true)
    setError('')
    try {
      await markReply(sessionId, !marked)
    } catch (e) {
      setError(why(e, FAILED_MARK))
    } finally {
      out.current = false
      setBusy(false)
    }
  }
  return (
    <div className="reply-mark">
      <button type="button" className="reply-toggle" aria-pressed={marked} disabled={busy} onClick={toggle}>
        Reply from the app
      </button>
      {marked && !hold && <span className="muted reply-marked">{MARKED_NOTE}</span>}
      {error && (
        <p role="alert" className="field-error">
          {error}
        </p>
      )}
    </div>
  )
}
