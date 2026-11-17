import { useCallback, useEffect, useRef, useState } from 'react'
import { answerPrompt, releasePrompt } from './api.js'
import PromptCard from './PromptCard.jsx'

// The Live view's answer card for a terminal session (T107, FR-114): the session's `pending_answer` as a prompt card, the two routes
// behind it, and the one line that says how it ended. The answer goes by option position; the page never sees a secret-shaped text.
export const NOTICE_MS = 10_000 // a "how it ended" line stays this long
export const SENT = 'Answer sent to the terminal session.'
export const HANDED_BACK = 'Handed back: the terminal asks now.'
export const ALREADY = 'Already answered.'
export const EXPIRED = 'The terminal asks now.'
const ALREADY_SERVER = 'already answered' // T106's exact 409 text for a second answer; the other 409 (an allow of an input that was cut) keeps the server's words in the card
const FAILED = 'Could not send the answer. Try again.'
const FAILED_RELEASE = 'Could not hand the prompt back. Try again.'

/** The one line per session that says how its last prompt ended, and what to call when that happens. Ends on its own after NOTICE_MS. */
export function useNotices(sessions) {
  const [notices, setNotices] = useState({}) // session id -> text
  const seen = useRef({}) // session id -> the prompt id last shown for it
  const settled = useRef(new Set()) // prompt ids the owner answered or handed back here
  const say = useCallback((sessionId, promptId, text) => {
    settled.current.add(promptId)
    setNotices((n) => ({ ...n, [sessionId]: text }))
  }, [])
  useEffect(() => {
    const now = {}
    for (const s of sessions) if (s.pending) now[s.session_id] = s.pending.id
    const listed = new Set(sessions.map((s) => s.session_id))
    const expired = []
    for (const [sessionId, promptId] of Object.entries(seen.current)) {
      if (now[sessionId] === promptId) continue
      if (settled.current.delete(promptId)) continue // we ended it ourselves and said so
      if (listed.has(sessionId)) expired.push(sessionId) // gone without us: it ran out, or the terminal took it
    }
    seen.current = now
    setNotices((n) => {
      if (expired.length === 0 && !Object.keys(now).some((id) => id in n)) return n // nothing to change: no new draw
      const next = { ...n }
      for (const id of Object.keys(now)) delete next[id] // a new prompt replaces the line of the last one
      for (const id of expired) next[id] = EXPIRED
      return next
    })
  }, [sessions])
  useEffect(() => {
    if (Object.keys(notices).length === 0) return undefined
    const timer = setTimeout(() => setNotices({}), NOTICE_MS)
    return () => clearTimeout(timer)
  }, [notices])
  return [notices, say]
}

/** The answer for a failed request: a prompt that is gone says so, a refused answer says why, anything else asks to try again. */
const whyNot = (error, fallback) => ((error.status === 400 || error.status === 409) && error.message ? error.message : fallback)

/** The prompt card of one session. `onEnd(sessionId, promptId, text)`: the prompt ended here, and this is the line to show. */
export function PendingAnswer({ sessionId, prompt, onEnd }) {
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  const open = useRef(false) // one request at a time: a second click while one is out is dropped
  const run = async (request, fallback, sent) => {
    if (open.current) return
    open.current = true
    setBusy(true)
    setError('')
    try {
      await request()
      onEnd(sessionId, prompt.id, sent)
    } catch (e) {
      if (e.status === 409 && e.message === ALREADY_SERVER) onEnd(sessionId, prompt.id, ALREADY)
      else if (e.status === 404) onEnd(sessionId, prompt.id, EXPIRED)
      else setError(whyNot(e, fallback))
    } finally {
      open.current = false
      setBusy(false)
    }
  }
  return (
    <PromptCard
      prompt={prompt}
      busy={busy}
      error={error}
      onAnswer={(fields) => run(() => answerPrompt(prompt.id, fields), FAILED, SENT)}
      onRelease={() => run(() => releasePrompt(prompt.id), FAILED_RELEASE, HANDED_BACK)}
    />
  )
}
