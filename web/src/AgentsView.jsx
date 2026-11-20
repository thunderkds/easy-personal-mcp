import { useState } from 'react'
import { getJson, sendJson } from './api.js'
import { useLoad } from './useLoad.js'
import { ago, fmt, limitTone, localTime, until } from './format.js'
import { Progress } from './Widgets.jsx'
import Loading from './Loading.jsx'
import { useNotify } from './Toast.jsx'
import { SignInCommand } from './CopyCommand.jsx'

const NOT_APPLIED = ['refused', 'skipped']

/**
 * Name and status actions of one account. Detected: name it or ignore it. Ignored: choose it again.
 * Confirmed: rename it, or remove it (the same `ignore` call; not the default account: the server refuses that).
 */
function AccountActions({ account, onChanged }) {
  const { status, is_default: isDefault } = account
  const [editing, setEditing] = useState(status === 'detected')
  const [label, setLabel] = useState(account.label ?? '')
  const [busy, setBusy] = useState(false)
  const notify = useNotify()
  async function act(path, body) {
    if (busy) return
    setBusy(true)
    try {
      await sendJson('POST', `/api/accounts/${account.id}/${path}`, body)
      onChanged(account, path)
    } catch (e) {
      notify('error', `Could not complete: ${e.message}`)
      setBusy(false)
    }
  }
  const ignore = (
    <button type="button" disabled={busy} onClick={() => act('ignore', {})}>
      {status === 'confirmed' ? 'Remove' : 'Ignore'}
    </button>
  )
  return (
    <>
      {editing ? (
        <>
          <div className="field">
            <label htmlFor={`name-${account.id}`} className="required">
              Name
            </label>
            <input
              id={`name-${account.id}`}
              value={label}
              aria-required="true"
              aria-describedby={`name-${account.id}-hint`}
              autoComplete="off"
              onChange={(e) => setLabel(e.target.value)}
            />
            <span id={`name-${account.id}-hint`} className="hint">
              Shown on the account card, for example Claude company.
            </span>
          </div>
          <div className="actions">
            <button type="button" disabled={busy || !label.trim()} onClick={() => act('confirm', { label: label.trim() })}>
              {busy ? 'Saving…' : status === 'confirmed' ? 'Save name' : 'Confirm & name'}
            </button>
            {status === 'detected' ? (
              ignore
            ) : (
              <button type="button" disabled={busy} onClick={() => setEditing(false)}>
                Cancel
              </button>
            )}
          </div>
        </>
      ) : (
        <div className="actions">
          {status === 'ignored' && (
            <button type="button" onClick={() => setEditing(true)}>
              Use this account
            </button>
          )}
          {status === 'confirmed' && (
            <>
              <button type="button" onClick={() => setEditing(true)}>
                Rename
              </button>
              {!isDefault && ignore}
            </>
          )}
        </div>
      )}
    </>
  )
}

/** Workspaces picker of a confirmed Claude account: store picks, show the sync plan, apply only after Confirm. */
function Picker({ account, workspaces }) {
  const [saved, setSaved] = useState(account.workspaces)
  const [picks, setPicks] = useState(account.workspaces)
  const [steps, setSteps] = useState(null) // null = no plan shown
  const [busy, setBusy] = useState(false)
  const [reviewing, setReviewing] = useState(false)
  const notify = useNotify()
  const path = `/api/accounts/${account.id}/workspaces`
  const names = workspaces.filter((w) => !w.is_orphan).map((w) => w.name)

  if (names.length === 0) return <p className="muted">No workspaces yet: create a workspace first.</p>

  const toggle = (name) => {
    setPicks(picks.includes(name) ? picks.filter((n) => n !== name) : [...picks, name])
  }

  async function review() {
    if (busy) return
    setBusy(true)
    setReviewing(true)
    try {
      await sendJson('PUT', path, { workspaces: picks })
      const plan = await getJson(`/api/sync?account=${account.id}`)
      setSteps(plan.steps)
    } catch (e) {
      notify('error', `Could not complete: ${e.message}`)
    } finally {
      setBusy(false)
      setReviewing(false)
    }
  }

  async function revert() {
    setSteps(null)
    setPicks(saved)
    await sendJson('PUT', path, { workspaces: saved }).catch(() => {})
  }

  async function cancel() {
    if (busy) return
    setBusy(true)
    await revert()
    setBusy(false)
  }

  async function apply() {
    if (busy) return
    setBusy(true)
    try {
      const body = await sendJson('POST', '/api/sync', { account: account.id, confirm: true })
      const ok = body.status === 0
      setSaved(picks)
      setSteps(null)
      notify(ok ? 'success' : 'error', body.lines?.length ? body.lines.join('\n') : ok ? 'Done.' : 'Nothing was changed.')
    } catch (e) {
      notify('error', `Could not complete: ${e.message}`)
      await revert()
    } finally {
      setBusy(false)
    }
  }

  function close() {
    setSaved(picks)
    setSteps(null)
  }

  const changes = steps && steps.some((s) => !NOT_APPLIED.includes(s.action))
  return (
    <>
      <fieldset className="group">
        <legend>Workspaces</legend>
        <div className="picks">
          {names.map((n) => (
            <label key={n}>
              <input type="checkbox" checked={picks.includes(n)} disabled={steps !== null || busy} onChange={() => toggle(n)} />{' '}
              {n}
            </label>
          ))}
        </div>
      </fieldset>
      {steps === null && (
        <div className="actions">
          <button type="button" disabled={busy || picks.join() === saved.join()} onClick={review}>
            {reviewing ? 'Reviewing…' : 'Review changes'}
          </button>
        </div>
      )}
      {steps !== null && (
        <div role="group" aria-label={`Changes for ${account.label}`}>
          {steps.length > 0 ? (
            <>
              <p>This will change:</p>
              <pre className="plan">{steps.map((s) => s.line).join('\n')}</pre>
            </>
          ) : (
            <p>Nothing to change.</p>
          )}
          <div className="actions">
            {changes && (
              <button type="button" onClick={apply} disabled={busy}>
                Confirm
              </button>
            )}
            <button type="button" onClick={changes ? cancel : close} disabled={busy}>
              {changes ? 'Cancel' : 'Close'}
            </button>
          </div>
        </div>
      )}
    </>
  )
}

const readablePlan = (plan) => plan.replace(/^stripe_/, '').replace(/_/g, ' ')

/** One limit window of the plan-limits snapshot: a bar with its numbers, or the stale note. Hidden without a percentage. */
function LimitBar({ name, window, writtenAt }) {
  if (!window || window.used_percentage === null || window.used_percentage === undefined) return null
  if (window.stale) return <p className="muted">{`${name} · as of ${localTime(writtenAt)}, reset since`}</p>
  const pct = Math.round(window.used_percentage)
  const reset = until(window.resets_at)
  const text = `${name} ${pct}% · resets ${reset === 'soon' ? 'soon' : `in ${reset}`}`
  return (
    <div className={`limit limit-${limitTone(pct)}`}>
      <p>{text}</p>
      <Progress ratio={window.used_percentage / 100} label={`${name} limit ${pct}%, resets ${reset === 'soon' ? 'soon' : `in ${reset}`}`} />
    </div>
  )
}

/** The account's status line: who, what model, how much used, plan limits. */
function StatusLine({ account }) {
  const s = account.status_line
  if (!s) return null
  const who = [s.email, s.plan && readablePlan(s.plan), s.organization].filter(Boolean).join(' · ')
  const limits = s.limits
  const windows = limits && [limits.five_hour, limits.seven_day].some((w) => w && w.used_percentage !== null && w.used_percentage !== undefined)
  return (
    <div className="status-line">
      {who && (
        <p className="who">
          <svg width="13" height="13" viewBox="0 0 16 16" aria-hidden="true">
            <circle cx="8" cy="5.5" r="2.6" fill="none" stroke="currentColor" strokeWidth="1.3" />
            <path d="M3 13.5c.9-2.4 2.8-3.6 5-3.6s4.1 1.2 5 3.6" fill="none" stroke="currentColor" strokeWidth="1.3" strokeLinecap="round" />
          </svg>
          <span>{who}</span>
        </p>
      )}
      <p>{`${s.last_model ? `${s.last_model} · ` : ''}last active ${ago(s.last_active_utc)}`}</p>
      <p>{`Tokens: ${fmt(s.tokens_today)} today · ${fmt(s.tokens_7d)} last 7 days`}</p>
      {windows ? (
        <>
          <LimitBar name="5h" window={limits.five_hour} writtenAt={limits.written_at} />
          <LimitBar name="7d" window={limits.seven_day} writtenAt={limits.written_at} />
        </>
      ) : (
        account.provider === 'claude' && <p className="muted">Plan limits: add the status-line snippet (README)</p>
      )}
    </div>
  )
}

function AccountRow({ account, workspaces, onChanged, justConfirmed }) {
  const { provider, label, folder, status } = account
  const name = label || folder.split('/').filter(Boolean).pop() || folder
  return (
    <li aria-label={name} data-status={status}>
      <div className="entry-head">
        <strong>{name}</strong>
        <span className={`label ${status === 'confirmed' ? 'label-improve' : status === 'missing' ? 'label-warning' : 'label-aware'}`}>
          {status === 'confirmed' ? 'confirmed ✓' : status}
        </span>
        <span className="muted">{provider}</span>
      </div>
      <p className="muted">{folder}</p>
      {status === 'missing' ? <p className="muted">The folder is gone. Its stored usage stays in reports.</p> : <StatusLine account={account} />}
      {['detected', 'ignored', 'confirmed'].includes(status) && <AccountActions account={account} onChanged={onChanged} />}
      {status === 'confirmed' && justConfirmed && (
        <p role="status" className="muted">
          Usage from this account appears after the next ingest. Run <code>easy-personal ingest</code>.
        </p>
      )}
      {status === 'confirmed' && provider === 'codex' && <p className="muted">listed only</p>}
      {status === 'confirmed' && provider === 'claude' && <Picker account={account} workspaces={workspaces} />}
    </li>
  )
}

/** T132: the removed (ignored) accounts, collapsed. Use this account brings one back through the confirm flow. */
function Removed({ accounts, workspaces, onChanged }) {
  const [shown, setShown] = useState(false)
  if (accounts.length === 0) return null
  return (
    <div className="board-unlisted">
      <button type="button" className="board-toggle" aria-expanded={shown} onClick={() => setShown(!shown)}>
        {`Removed (${accounts.length})`}
      </button>
      {shown && (
        <ul className="entries agent-cards" aria-label="Removed accounts">
          {accounts.map((a) => (
            <AccountRow key={a.id} account={a} workspaces={workspaces} onChanged={onChanged} justConfirmed={false} />
          ))}
        </ul>
      )}
    </div>
  )
}

function Accounts({ by, workspaces, onChanged, confirmedIds }) {
  const state = useLoad(() => getJson(`/api/accounts?by=${by}`))
  if (state.status === 'loading') return <Loading label="Loading agent accounts" />
  if (state.status === 'error')
    return (
      <p role="alert" className="error">
        Could not load agent accounts: {state.message}
      </p>
    )
  const listed = (accounts) => accounts.filter((a) => a.status !== 'ignored')
  const all = by === 'provider' ? Object.values(state.data.providers).flat() : state.data.accounts
  const groups = (
    by === 'provider'
      ? Object.entries(state.data.providers).map(([title, accounts]) => ({ title, accounts: listed(accounts) }))
      : [{ title: null, accounts: listed(state.data.accounts) }]
  ).filter((g) => g.title === null || g.accounts.length > 0)
  return (
    <>
      {groups.map((g) => (
        <section key={g.title ?? 'all'} aria-label={g.title ?? 'Agent accounts'}>
          {g.title && <h2>{g.title}</h2>}
          <ul className="entries agent-cards">
            {g.accounts.map((a) => (
              <AccountRow key={a.id} account={a} workspaces={workspaces} onChanged={onChanged} justConfirmed={confirmedIds.includes(a.id)} />
            ))}
          </ul>
        </section>
      ))}
      <Removed accounts={all.filter((a) => a.status === 'ignored')} workspaces={workspaces} onChanged={onChanged} />
    </>
  )
}

const NAME_RULE = /^[a-z0-9][a-z0-9-]{0,29}$/

/** T132: make a new account folder (the app creates it), then show how to sign in. The new account is listed as detected: name it there. */
function NewAccount({ onCreated }) {
  const [value, setValue] = useState('')
  const [busy, setBusy] = useState(false)
  const [made, setMade] = useState(null) // the row the server answered
  const notify = useNotify()
  const name = value.trim()
  const invalid = name !== '' && !NAME_RULE.test(name)
  async function create(e) {
    e.preventDefault()
    if (busy || !NAME_RULE.test(name)) return
    setBusy(true)
    try {
      const row = await sendJson('POST', '/api/accounts/create', { name })
      setMade(row)
      setValue('')
      onCreated()
    } catch (err) {
      notify('error', `Could not complete: ${err.message}`)
    } finally {
      setBusy(false)
    }
  }
  return (
    <section aria-label="New account">
      <h2>New account</h2>
      <form className="new-account" onSubmit={create}>
        <div className="field">
          <label htmlFor="new-account-name">New account name</label>
          <input
            id="new-account-name"
            value={value}
            autoComplete="off"
            aria-invalid={invalid}
            aria-describedby={invalid ? 'new-account-error' : 'new-account-hint'}
            onChange={(e) => setValue(e.target.value)}
          />
          <span id="new-account-hint" className="hint">
            Lower-case letters, digits and dashes, up to 30 characters. The folder is named .claude-name.
          </span>
          {invalid && (
            <span id="new-account-error" className="field-error">
              Use only lower-case letters, digits and dashes, starting with a letter or digit, up to 30 characters.
            </span>
          )}
        </div>
        <div className="actions">
          <button type="submit" disabled={busy || name === '' || invalid}>
            {busy ? 'Creating…' : 'Create'}
          </button>
        </div>
      </form>
      {made && <SignInCommand folder={made.folder} />}
    </section>
  )
}

export default function AgentsView({ onAccountsChanged }) {
  const [by, setBy] = useState('account')
  const [version, setVersion] = useState(0)
  const [confirmedIds, setConfirmedIds] = useState([]) // accounts the owner chose in this visit: the reload keeps the ingest hint
  const ws = useLoad(() => getJson('/api/workspaces'))
  const workspaces = ws.status === 'ready' ? ws.data.workspaces : []
  return (
    <main>
      <h1>Agents</h1>
      <p className="muted">Agent accounts found on this machine. Every change to Claude Code is shown before it is written.</p>
      <div className="periods" role="group" aria-label="Group by">
        {[
          ['account', 'By account'],
          ['provider', 'By provider'],
        ].map(([id, text]) => (
          <button key={id} type="button" aria-pressed={by === id} onClick={() => setBy(id)}>
            {text}
          </button>
        ))}
      </div>
      {ws.status === 'error' && (
        <p role="alert" className="error">
          Could not load workspaces: {ws.message}
        </p>
      )}
      <NewAccount
        onCreated={() => {
          setVersion((v) => v + 1)
          onAccountsChanged?.()
        }}
      />
      {ws.status !== 'loading' && (
        <Accounts
          key={`${by}-${version}`}
          by={by}
          workspaces={workspaces}
          confirmedIds={confirmedIds}
          onChanged={(account, action) => {
            if (action === 'confirm' && account.status !== 'confirmed') setConfirmedIds((ids) => [...ids, account.id])
            setVersion((v) => v + 1)
            onAccountsChanged?.()
          }}
        />
      )}
    </main>
  )
}
