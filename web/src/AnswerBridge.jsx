import { useEffect, useId, useRef, useState } from 'react'
import { getHooks, setHooks } from './api.js'
import Loading from './Loading.jsx'
import { useLoad } from './useLoad.js'

// Settings, *Answer terminal sessions from the app* (T107, FR-113): per confirmed Claude account, on / off / changed by hand / binary moved.
// Turn on shows the exact entries that will be added to the account's settings.json and writes only after the owner confirms.
const STATUS = {
  on: 'On',
  off: 'Off',
  changed_by_hand: 'Changed by hand',
  binary_moved: 'Binary moved',
  unreadable: 'Settings file cannot be read',
}
const EXPLAIN = {
  changed_by_hand: 'An entry of ours in this account’s settings.json was changed by hand, or only one of the two is there. Turn on adds what is missing; Turn off removes only the entries the app added.',
  binary_moved: 'The entries name a copy of this app that has moved. Turn on points them at this copy; Turn off removes them.',
  unreadable: 'The account’s settings.json is not a JSON object or cannot be read, so the app leaves it as it is.',
}
export const NOT_LISTENING = 'Turned on, but not listening: restart the app'
const FAILED = 'Could not change the setting. Try again.'
const SHOWN_ERRORS = new Set([400, 409, 503]) // the server says why in plain words (the file was left as it was)

const changeError = (e) => (SHOWN_ERRORS.has(e.status) && e.message ? e.message : FAILED)
const settingsFile = (account) => (account ? `${account.folder.replace(/\/+$/, '')}/settings.json` : 'the account’s settings.json')

/** The confirm step of Turn on: the exact text that is merged in, then Add these entries / Cancel. */
function Confirm({ account, file, entries, error, saving, onUse, onCancel, headingRef }) {
  const id = useId()
  return (
    <div className="bridge-confirm" role="group" aria-labelledby={id}>
      <h3 id={id} ref={headingRef} tabIndex={-1}>{`Turn on for ${account.label}?`}</h3>
      <p>
        These entries are added to <span className="folder">{file}</span>. Every other key and hook in the file stays, a copy of the old file is kept in <span className="folder">~/.easy-personal/backups/</span>, and Turn off removes only these entries.
      </p>
      <pre className="bridge-entries" tabIndex={0} aria-label="The entries that are added">
        {entries}
      </pre>
      {error && (
        <p role="alert" className="field-error">
          {error}
        </p>
      )}
      <div className="actions">
        <button type="button" className="primary" disabled={saving} onClick={onUse}>
          Add these entries
        </button>
        <button type="button" disabled={saving} onClick={onCancel}>
          Cancel
        </button>
      </div>
    </div>
  )
}

function Accounts({ initial, byId }) {
  const [listening, setListening] = useState(initial.listening) // the server says whether its socket is open; undefined (an older server) shows no note
  const [rows, setRows] = useState(initial.accounts)
  const [confirming, setConfirming] = useState(null) // the account id whose confirm step is open
  const [error, setError] = useState({}) // account id -> text of its last failure
  const [saving, setSaving] = useState(false)
  const open = useRef(false)
  const headingRef = useRef(null)
  useEffect(() => {
    if (confirming !== null) headingRef.current?.focus() // the confirm step takes the focus, so the exact text is read first
  }, [confirming])

  const change = async (account, on) => {
    if (open.current) return // one request at a time
    open.current = true
    setSaving(true)
    setError((e) => ({ ...e, [account.id]: '' }))
    try {
      const row = await setHooks(account.id, on)
      setRows((all) => all.map((r) => (r.id === account.id ? { ...r, ...row } : r)))
      if (typeof row.listening === 'boolean') setListening(row.listening)
      setConfirming(null)
    } catch (e) {
      setError((all) => ({ ...all, [account.id]: changeError(e) }))
    } finally {
      open.current = false
      setSaving(false)
    }
  }

  if (rows.length === 0) return <p className="muted">No confirmed Claude account yet. Confirm one in Agents first.</p>
  return (
    <ul className="bridge-accounts">
      {rows.map((a) => {
        const shown = confirming === a.id
        return (
          <li key={a.id} className="bridge-account" data-status={a.status}>
            <div className="bridge-row">
              <span className="account-text">
                <strong>{a.label}</strong>
                {byId[a.id] && <span className="folder">{byId[a.id].folder.split('/').filter(Boolean).pop()}</span>}
              </span>
              <span className="bridge-status" data-status={a.status}>
                {STATUS[a.status] ?? a.status}
              </span>
              {!shown && (
                <span className="actions">
                  {a.status !== 'on' && a.status !== 'unreadable' && (
                    <button type="button" disabled={saving} onClick={() => setConfirming(a.id)}>
                      Turn on
                    </button>
                  )}
                  {a.status !== 'off' && a.status !== 'unreadable' && (
                    <button type="button" disabled={saving} onClick={() => change(a, false)}>
                      Turn off
                    </button>
                  )}
                </span>
              )}
            </div>
            {a.status === 'on' && listening === false && <p className="bridge-warn">{NOT_LISTENING}</p>}
            {EXPLAIN[a.status] && <p className="muted">{EXPLAIN[a.status]}</p>}
            {error[a.id] && !shown && (
              <p role="alert" className="field-error">
                {error[a.id]}
              </p>
            )}
            {shown && (
              <Confirm
                account={a}
                file={settingsFile(byId[a.id])}
                entries={initial.entries_text}
                error={error[a.id]}
                saving={saving}
                headingRef={headingRef}
                onUse={() => change(a, true)}
                onCancel={() => {
                  setConfirming(null)
                  setError((e) => ({ ...e, [a.id]: '' }))
                }}
              />
            )}
          </li>
        )
      })}
    </ul>
  )
}

/**
 * The Settings block. `accounts`: the page's account list, only for each account's folder. Nothing is drawn when the server has no
 * such route (the Python server); a failed load names itself and leaves the other settings alone.
 */
export default function AnswerBridge({ accounts }) {
  const state = useLoad(getHooks, [])
  if (state.status === 'ready' && state.data === null) return null
  const byId = Object.fromEntries(accounts.map((a) => [String(a.id), a]))
  return (
    <section aria-labelledby="h-answer-bridge" className="card">
      <h2 id="h-answer-bridge">Answer terminal sessions from the app</h2>
      {state.status === 'loading' && <Loading label="Loading the answer bridge" />}
      {state.status === 'error' && (
        <p role="alert" className="error">
          Could not load the answer bridge: {state.message}
        </p>
      )}
      {state.status === 'ready' && (
        <>
          <p>Off by default. When it is on for an account, a Claude Code session open in a terminal that asks a question or asks for permission shows that prompt in the Live view, and your answer goes back to that session. With the app closed, the terminal asks as usual.</p>
          <p className="bridge-note">Sessions that were started before you turn this on are not covered. Start a new session.</p>
          <Accounts initial={state.data} byId={byId} />
        </>
      )}
    </section>
  )
}
