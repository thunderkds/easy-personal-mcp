import { useEffect, useRef, useState } from 'react'
import { readToken } from './api.js'

/** One folder listing from `GET /api/dirs` (no `path` = where the dashboard was started). A non-2xx answer throws the server's `error` text. */
async function fetchDirs(path) {
  const query = path ? `?path=${encodeURIComponent(path)}` : ''
  const response = await fetch(`/api/dirs${query}`, { headers: { 'X-Dashboard-Token': readToken() } })
  const body = await response.json().catch(() => ({}))
  if (!response.ok) throw new Error(body.error || `Could not list folders (status ${response.status})`)
  return body
}

/** A modal folder browser: walk the folders the dashboard server lists, then pick the one shown. */
export default function FolderPicker({ onChoose, onClose }) {
  const dialog = useRef(null)
  const latest = useRef(0)
  const [listing, setListing] = useState(null) // the last folder that loaded
  const [error, setError] = useState(null)
  const [loading, setLoading] = useState(true)

  function go(path) {
    const mine = ++latest.current // an older answer arriving late is dropped
    setLoading(true)
    setError(null)
    fetchDirs(path).then(
      (body) => mine === latest.current && (setListing(body), setLoading(false)),
      (e) => mine === latest.current && (setError(e.message), setLoading(false)),
    )
  }

  useEffect(() => {
    if (!dialog.current.open) dialog.current.showModal()
    dialog.current.focus()
    go()
    return () => {
      latest.current++
    }
  }, [])

  return (
    <dialog
      ref={dialog}
      className="folder-picker"
      aria-labelledby="folder-picker-title"
      tabIndex={-1}
      onCancel={(e) => {
        e.preventDefault() // Esc: the parent unmounts the dialog and returns the focus
        onClose()
      }}
    >
      <h3 id="folder-picker-title">Choose a repo folder</h3>
      <p className="folder-path" aria-label="Current folder">
        {listing?.path ?? '…'}
      </p>
      <div className="actions">
        <button type="button" disabled={!listing?.parent} onClick={() => go(listing.parent)}>
          Up
        </button>
        <button type="button" disabled={!listing} onClick={() => go(listing.home)}>
          Home
        </button>
      </div>
      {error && (
        <p role="alert" className="field-error">
          {error}
        </p>
      )}
      {loading && (
        <p role="status" className="muted">
          Loading folders…
        </p>
      )}
      {listing && (
        <>
          {listing.dirs.length === 0 ? (
            <p className="muted">No sub-folders here.</p>
          ) : (
            <ul className="folder-list" aria-label="Sub-folders">
              {listing.dirs.map((d) => (
                <li key={d.name}>
                  <button type="button" onClick={() => go(`${listing.path === '/' ? '' : listing.path}/${d.name}`)}>
                    <span className="folder-name">{d.name}</span>
                    {d.repo && <span className="chip">git</span>}
                  </button>
                </li>
              ))}
            </ul>
          )}
          {listing.truncated && <p className="muted">Showing the first {listing.dirs.length} folders.</p>}
        </>
      )}
      <div className="actions folder-picker-actions">
        <button type="button" className="primary" disabled={!listing || loading} onClick={() => onChoose(listing.path)}>
          Choose this folder
        </button>
        <button type="button" onClick={onClose}>
          Cancel
        </button>
      </div>
    </dialog>
  )
}
