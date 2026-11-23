import { useCallback, useEffect, useRef, useState } from 'react'
import { getTermSessions } from './api.js'
import Loading from './Loading.jsx'
import { folderName } from './SessionPanel.jsx'
import TermPanel from './TermPanel.jsx'
import { termChannel } from './termChannel.js'

const CLOSE_WAIT_MS = 300

/**
 * A terminal in its own window (T135): the page opened as `/?term=<id>#<token>` by the panel's Pop out. It shows only that session's terminal,
 * filling the window, under a slim bar that says it belongs to the app: the app's name, that closing the window does not stop the session, and
 * Dock back (Live opens the terminal in its panel again, then this window closes). It is one more attached page of the same session (FR-137).
 * Over the channel (`termChannel.js`) it tells Live it is open, answers Live's `who`, and says `closed` when the window goes.
 */
export default function TermWindow({ id }) {
  const [term, setTerm] = useState(undefined) // undefined: reading; null: not listed
  const [error, setError] = useState('')
  const [docked, setDocked] = useState(false) // Dock back was pressed and the window did not close
  const shown = useRef(false) // a first read answered: a later failed refresh keeps the terminal
  const channel = useRef(null)
  const read = useCallback(async () => {
    try {
      const list = await getTermSessions()
      setTerm((list ?? []).find((t) => t && t.id === id) ?? null)
      shown.current = true
      setError('')
    } catch {
      // A refresh after Stop or the exit frame that fails keeps the terminal shown: only a first read that fails has nothing to show.
      if (!shown.current) setError('Could not read the terminal sessions. Close this window and open the terminal from Live again.')
    }
  }, [id])
  useEffect(() => {
    read()
  }, [read])
  useEffect(() => {
    if (term) document.title = `${folderName(term.repo) || 'terminal'} · terminal · Easy Personal`
  }, [term])
  useEffect(() => {
    const ch = termChannel((m) => {
      if (m.t === 'who') ch.postMessage({ t: 'open', id })
    })
    channel.current = ch
    ch?.postMessage({ t: 'open', id })
    const gone = () => ch?.postMessage({ t: 'closed', id })
    window.addEventListener('pagehide', gone)
    return () => {
      window.removeEventListener('pagehide', gone)
      gone()
      ch?.close()
      channel.current = null
    }
  }, [id])

  const dockBack = () => {
    channel.current?.postMessage({ t: 'dock', id })
    channel.current?.postMessage({ t: 'closed', id })
    window.close()
    setTimeout(() => setDocked(true), CLOSE_WAIT_MS) // still here: the browser keeps a window it did not open by script
  }

  let body
  if (error)
    body = (
      <p role="alert" className="field-error">
        {error}
      </p>
    )
  else if (term === undefined) body = <Loading label="Loading the terminal" />
  else if (term === null) body = <p className="muted">This terminal session is not running here any more. Close this window; Live lists the sessions.</p>
  else body = <TermPanel term={term} popped onChanged={read} />
  return (
    <main className="term-window">
      <header className="term-window-bar">
        <span className="term-window-app">Easy Personal · Terminal</span>
        <span className="muted">{docked ? 'Live shows the terminal again if the app window is open. You can close this window.' : 'Closing this window does not stop the session.'}</span>
        {term && !docked && (
          <button type="button" onClick={dockBack} title="Show this terminal in the app window again and close this one">
            Dock back
          </button>
        )}
      </header>
      {body}
    </main>
  )
}
