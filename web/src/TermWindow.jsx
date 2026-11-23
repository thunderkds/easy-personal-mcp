import { useCallback, useEffect, useState } from 'react'
import { getTermSessions } from './api.js'
import Loading from './Loading.jsx'
import { folderName } from './SessionPanel.jsx'
import TermPanel from './TermPanel.jsx'

/**
 * A terminal in its own window (T135): the page opened as `/?term=<id>#<token>` by the panel's Pop out. It shows only that session's terminal,
 * filling the window, with no shell around it: no rail, no live stream, no Back. It is one more attached page of the same session (FR-137).
 */
export default function TermWindow({ id }) {
  const [term, setTerm] = useState(undefined) // undefined: reading; null: not listed
  const [error, setError] = useState('')
  const read = useCallback(async () => {
    try {
      const list = await getTermSessions()
      setTerm((list ?? []).find((t) => t && t.id === id) ?? null)
      setError('')
    } catch {
      setError('Could not read the terminal sessions. Close this window and open the terminal from Live again.')
    }
  }, [id])
  useEffect(() => {
    read()
  }, [read])
  useEffect(() => {
    if (term) document.title = `${folderName(term.repo) || 'terminal'} · terminal`
  }, [term])

  if (error)
    return (
      <main className="term-window">
        <p role="alert" className="field-error">
          {error}
        </p>
      </main>
    )
  if (term === undefined) return <Loading label="Loading the terminal" />
  if (term === null)
    return (
      <main className="term-window">
        <p className="muted">This terminal session is not running here any more. Close this window; Live lists the sessions.</p>
      </main>
    )
  return (
    <main className="term-window">
      <TermPanel term={term} popped onChanged={read} />
    </main>
  )
}
