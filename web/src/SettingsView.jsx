import { useEffect, useId, useRef, useState } from 'react'
import AnswerBridge from './AnswerBridge.jsx'
import { getAgentRun, getJson, setAgentRunAccount } from './api.js'
import Loading from './Loading.jsx'
import { useNotify } from './Toast.jsx'
import { useLoad } from './useLoad.js'

// Settings (FR-104 parts 1 and 2): the agent run account. The note below is the one the owner confirms before an account is saved.
const NOTE = "Agent runs use this account's Claude subscription through Claude Code on this machine. They count against its plan limits."
const MARK = 'easy-personal-agent-run-asked' // "the first-open prompt was answered", in localStorage
const SAVE_ERRORS = new Map([[400, 'This account cannot be used for agent runs. Pick another.']])
const SAVE_FAILED = 'Could not save the setting. Try again.'

const isChoice = (a) => a.provider === 'claude' && a.status === 'confirmed'
const folderName = (folder) => folder.split('/').filter(Boolean).pop() ?? folder
const saveError = (error) => SAVE_ERRORS.get(error.status) ?? SAVE_FAILED

/** "5h 43% · 7d 75%" from the account's plan-limit snapshot; '' when it has no fresh window. */
function limitLine(account) {
  const limits = account?.status_line?.limits
  if (!limits) return ''
  const part = (name, w) => (w && !w.stale && typeof w.used_percentage === 'number' ? `${name} ${Math.round(w.used_percentage)}%` : null)
  return [part('5h', limits.five_hour), part('7d', limits.seven_day)].filter(Boolean).join(' · ')
}

// localStorage may be missing or blocked: no mark available means the prompt may show again, and the page still works.
function readMark() {
  try {
    return window.localStorage.getItem(MARK) === '1'
  } catch {
    return false
  }
}
function writeMark() {
  try {
    window.localStorage.setItem(MARK, '1')
  } catch {
    // asking again is allowed
  }
}

/**
 * A modal dialog: opened with showModal, focus moves in (and again when `focusKey` changes), Escape calls `onCancel`,
 * and when it goes away the focus returns to `returnTo` (a ref), else to what had it before.
 */
function Modal({ labelId, onCancel, focusKey, returnTo, children }) {
  const dialog = useRef(null)
  const opener = useRef(document.activeElement)
  useEffect(() => {
    if (!dialog.current.open) dialog.current.showModal()
    return () => {
      const target = returnTo?.current ?? opener.current
      if (target?.isConnected) target.focus()
    }
  }, [])
  useEffect(() => {
    dialog.current.focus()
  }, [focusKey])
  return (
    <dialog
      ref={dialog}
      className="folder-picker agent-dialog"
      aria-labelledby={labelId}
      tabIndex={-1}
      onCancel={(e) => {
        e.preventDefault() // Esc: the parent takes the dialog away
        onCancel()
      }}
    >
      {children}
    </dialog>
  )
}

/** The radio list of the confirmed Claude accounts and the Choose button; `children` are the extra buttons next to it. */
function Picker({ accounts, initial = '', chooseRef, onChoose, children }) {
  const [picked, setPicked] = useState(initial)
  const group = useId()
  return (
    <div className="account-choice">
      <p id={group} className="muted">
        Claude accounts
      </p>
      <div role="radiogroup" aria-labelledby={group} className="account-options">
        {accounts.map((a) => (
          <label key={a.id} className="account-option">
            <input type="radio" name={group} value={a.id} checked={picked === String(a.id)} onChange={() => setPicked(String(a.id))} />
            <span className="account-text">
              <strong>{a.label}</strong>
              <span className="folder">{folderName(a.folder)}</span>
              {limitLine(a) && <span className="muted">{limitLine(a)}</span>}
            </span>
          </label>
        ))}
      </div>
      <div className="actions">
        <button type="button" className="primary" ref={chooseRef} disabled={!picked} onClick={() => onChoose(accounts.find((a) => String(a.id) === picked))}>
          Choose
        </button>
        {children}
      </div>
    </div>
  )
}

/** The confirm step: the fixed note, **Use this account**, **Cancel**. */
function Confirm({ labelId, account, error, saving, onUse, onCancel }) {
  return (
    <>
      <h3 id={labelId}>{`Use ${account.label} for agent runs?`}</h3>
      <p>{NOTE}</p>
      {error && (
        <p role="alert" className="field-error">
          {error}
        </p>
      )}
      <div className="actions folder-picker-actions">
        <button type="button" className="primary" disabled={saving} onClick={onUse}>
          Use this account
        </button>
        <button type="button" onClick={onCancel}>
          Cancel
        </button>
      </div>
    </>
  )
}

/** One request at a time: a second click while one is open is dropped. */
function useSaving() {
  const open = useRef(false)
  const [saving, setSaving] = useState(false)
  const run = async (work) => {
    if (open.current) return undefined
    open.current = true
    setSaving(true)
    try {
      return await work()
    } finally {
      open.current = false
      setSaving(false)
    }
  }
  return [saving, run]
}

function AgentRunAccount({ initial, accounts, onOpenAgents }) {
  const [setting, setSetting] = useState(initial)
  const [changing, setChanging] = useState(false)
  const [pending, setPending] = useState(null) // the account whose confirm step is open
  const [error, setError] = useState('')
  const [saved, setSaved] = useState(0)
  const [saving, run] = useSaving()
  const chooseRef = useRef(null)
  const heading = useRef(null)
  const confirmId = useId()
  const choices = accounts.filter(isChoice)

  useEffect(() => {
    if (saved > 0) heading.current?.focus() // the button that was pressed is gone: the block's title takes the focus
  }, [saved])

  const save = (account) =>
    run(async () => {
      setError('')
      try {
        setSetting(await setAgentRunAccount(account ? account.id : null))
        setPending(null)
        setChanging(false)
        setSaved((n) => n + 1)
      } catch (e) {
        setError(saveError(e))
      }
    })
  const cancel = () => {
    setPending(null)
    setError('')
  }

  const chosen = setting.account && accounts.find((a) => String(a.id) === String(setting.account.id))
  const limit = limitLine(chosen)
  let body
  if (setting.enabled && !changing) {
    body = (
      <>
        {setting.account && (
          <p className="account-text">
            <strong>{setting.account.label}</strong>
            {chosen && <span className="folder">{folderName(chosen.folder)}</span>}
            {limit && <span className="muted">{limit}</span>}
          </p>
        )}
        {setting.account?.status === 'missing' && <p className="muted">Its folder is gone on this machine. Change the account or turn this off.</p>}
        <p>{setting.account ? 'The app uses this account for every agent run.' : 'The app uses the default Claude account for every agent run.'}</p>
        <div className="actions">
          <button type="button" onClick={() => setChanging(true)}>
            Change
          </button>
          <button type="button" disabled={saving} onClick={() => save(null)}>
            Turn off
          </button>
        </div>
        {error && !pending && (
          <p role="alert" className="field-error">
            {error}
          </p>
        )}
      </>
    )
  } else {
    body = (
      <>
        {!setting.enabled && <p>Off. The app starts no agent run.</p>}
        {choices.length === 0 ? (
          <p>
            No confirmed Claude account yet. Confirm one in{' '}
            <button type="button" className="link-button" onClick={onOpenAgents}>
              Agents
            </button>
            .
          </p>
        ) : (
          <Picker accounts={choices} chooseRef={chooseRef} onChoose={setPending}>
            {changing && (
              <button type="button" onClick={() => setChanging(false)}>
                Cancel
              </button>
            )}
          </Picker>
        )}
      </>
    )
  }
  return (
    <section aria-labelledby="h-agent-run-account" className="card">
      <h2 id="h-agent-run-account" ref={heading} tabIndex={-1}>
        Agent run account
      </h2>
      {body}
      {pending && (
        <Modal labelId={confirmId} onCancel={cancel} returnTo={chooseRef}>
          <Confirm labelId={confirmId} account={pending} error={error} saving={saving} onUse={() => save(pending)} onCancel={cancel} />
        </Modal>
      )}
    </section>
  )
}

const load = () => Promise.all([getAgentRun(), getJson('/api/accounts?by=account')])

/** The Settings view: *Agent run account* (T092) and *Answer terminal sessions from the app* (T107). */
export default function SettingsView({ onOpenAgents }) {
  const state = useLoad(load, [])
  const [agentRun, accounts] = state.status === 'ready' ? state.data : []
  return (
    <main className="settings">
      <h1>Settings</h1>
      {state.status === 'loading' && <Loading label="Loading settings" />}
      {state.status === 'error' && (
        <p role="alert" className="error">
          Could not load the agent run account: {state.message}
        </p>
      )}
      {state.status === 'ready' && agentRun === null && <p className="muted">No settings are available in this version of the app.</p>}
      {state.status === 'ready' && agentRun && <AgentRunAccount initial={agentRun} accounts={accounts.accounts} onOpenAgents={onOpenAgents} />}
      {state.status === 'ready' && <AnswerBridge accounts={accounts.accounts.filter(isChoice)} />}
    </main>
  )
}

/**
 * The first-open prompt (FR-104 part 2), mounted once by the page shell. When the setting is off, the browser holds no mark,
 * and there is a confirmed Claude account to choose, a dialog offers the picker and the confirm step, with **Not now**.
 * Either answer sets the mark, so it never comes back. `accounts`: the confirmed Claude accounts (undefined or null while unknown);
 * `suppressed`: the First run view is open, which the prompt never covers.
 */
export function FirstOpenPrompt({ accounts, suppressed }) {
  const notify = useNotify()
  const [asked, setAsked] = useState(readMark)
  const [off, setOff] = useState(false)
  const [pending, setPending] = useState(null)
  const [last, setLast] = useState(null) // the account chosen last: the picker keeps it selected after Cancel
  const [backs, setBacks] = useState(0)
  const [error, setError] = useState('')
  const [saving, run] = useSaving()
  const chooseRef = useRef(null)
  const labelId = useId()

  useEffect(() => {
    if (backs > 0) chooseRef.current?.focus() // after the dialog's own focus move: back on the button that opened the confirm step
  }, [backs])

  useEffect(() => {
    if (asked) return undefined // marked: no request at all
    let current = true
    getAgentRun().then(
      (data) => current && setOff(data !== null && data.enabled === false),
      () => {}, // a failed read shows nothing
    )
    return () => {
      current = false
    }
  }, [asked])

  const choices = Array.isArray(accounts) ? accounts.filter(isChoice) : []
  if (asked || !off || suppressed || choices.length === 0) return null

  const answer = () => {
    writeMark()
    setAsked(true)
  }
  const back = () => {
    setPending(null)
    setError('')
    setBacks((n) => n + 1)
  }
  const choose = (account) => {
    setLast(account)
    setPending(account)
  }
  const use = () =>
    run(async () => {
      setError('')
      try {
        await setAgentRunAccount(pending.id)
        notify('success', `Agent runs now use ${pending.label}.`)
        answer()
      } catch (e) {
        setError(saveError(e))
      }
    })

  return (
    <Modal labelId={labelId} focusKey={pending ? 'confirm' : 'pick'} onCancel={pending ? back : answer}>
      {pending ? (
        <Confirm labelId={labelId} account={pending} error={error} saving={saving} onUse={use} onCancel={back} />
      ) : (
        <>
          <h3 id={labelId}>Choose an account for agent runs</h3>
          <p>The app can start Claude Code for you to write an investigate report. It is off until you choose an account. You can change this later in Settings.</p>
          <Picker accounts={choices} initial={last ? String(last.id) : ''} chooseRef={chooseRef} onChoose={choose}>
            <button type="button" onClick={answer}>
              Not now
            </button>
          </Picker>
        </>
      )}
    </Modal>
  )
}
