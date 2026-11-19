import { useEffect, useRef, useState } from 'react'
import { renameSession } from './api.js'
import { why } from './Composer.jsx'

// The name of a session in the panel header (T119, FR-127): the shown name, and an inline field to change it. Names are the owner's own typing, kept by
// the server in one owner-only file; the page holds nothing of them but the answer. React text only.
export const NAME_MAX = 80
export const ID_CHARS = 8

/** The shown name of a session: the owner's, else Claude's own, else the project and the short session id. */
export const titleOf = ({ name, claudeName, project, id }) => name || claudeName || [project, typeof id === 'string' && id ? id.slice(0, ID_CHARS) : null].filter(Boolean).join(' ')

/**
 * The heading and its Rename control. `name`: the owner's name now (null = none). `title`: what the heading shows. `id`: the session id (no id, no rename).
 * `onSaved(name)`: the server's answer, `null` when the name was removed. `headingRef`: the heading, so the panel can move the focus to it.
 * Enter saves (empty removes the name), Esc cancels without a request; Esc is not passed on, so it does not close the panel behind it.
 */
export default function SessionName({ id, name, title, headingRef, onSaved }) {
  const [editing, setEditing] = useState(false)
  const [value, setValue] = useState('')
  const [error, setError] = useState('')
  const [busy, setBusy] = useState(false)
  const out = useRef(false)
  const field = useRef(null)
  const button = useRef(null)
  const back = useRef(false) // the field was closed by the owner: the focus goes back to the Rename button
  useEffect(() => {
    if (editing) field.current?.focus()
    else if (back.current) {
      back.current = false
      button.current?.focus()
    }
  }, [editing])
  const edit = () => {
    setValue(name ?? '')
    setError('')
    setEditing(true)
  }
  const close = () => {
    back.current = true
    setEditing(false)
  }
  const save = async () => {
    if (out.current) return
    const next = value.trim()
    if (next === (name ?? '')) return close() // nothing changed: no request
    out.current = true
    setBusy(true)
    setError('')
    try {
      const answer = await renameSession(id, next === '' ? null : next)
      onSaved(typeof answer?.name === 'string' && answer.name !== '' ? answer.name : null)
      close()
    } catch (e) {
      setError(why(e, 'Could not save the name. Try again.'))
    } finally {
      out.current = false
      setBusy(false)
    }
  }
  if (editing)
    return (
      <div className="session-rename">
        <label className="field" htmlFor="session-name-field">
          Name
          <input
            id="session-name-field"
            ref={field}
            type="text"
            maxLength={NAME_MAX}
            value={value}
            disabled={busy}
            placeholder={title}
            aria-describedby="session-name-hint"
            aria-invalid={error ? 'true' : undefined}
            onChange={(e) => setValue(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Escape') {
                e.stopPropagation()
                close()
              } else if (e.key === 'Enter' && !e.nativeEvent.isComposing && e.keyCode !== 229) {
                e.preventDefault()
                save()
              }
            }}
          />
        </label>
        <span id="session-name-hint" className="muted session-rename-hint">
          Enter saves. Esc cancels. Empty removes the name.
        </span>
        {error && (
          <p role="alert" className="field-error">
            {error}
          </p>
        )}
      </div>
    )
  return (
    <>
      <h2 ref={headingRef} tabIndex={-1} className="session-title" title={title}>
        {title}
      </h2>
      {id && (
        <button ref={button} type="button" className="session-rename-button" onClick={edit}>
          Rename
        </button>
      )}
    </>
  )
}
