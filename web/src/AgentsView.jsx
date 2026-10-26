import { useState } from 'react'
import { getJson, sendJson } from './api.js'
import { useLoad } from './useLoad.js'

const NOT_APPLIED = ['refused', 'skipped']

/** A detected folder: name it (confirm) or ignore it. */
function Detected({ account, onChanged }) {
  const [label, setLabel] = useState('')
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState(null)
  async function act(path, body) {
    if (busy) return
    setBusy(true)
    setError(null)
    try {
      await sendJson('POST', `/api/accounts/${account.id}/${path}`, body)
      onChanged()
    } catch (e) {
      setError(e.message)
      setBusy(false)
    }
  }
  return (
    <>
      <div className="field">
        <label htmlFor={`name-${account.id}`}>Name</label>
        <input id={`name-${account.id}`} value={label} autoComplete="off" onChange={(e) => setLabel(e.target.value)} />
      </div>
      <div className="actions">
        <button type="button" disabled={busy || !label.trim()} onClick={() => act('confirm', { label: label.trim() })}>
          Confirm &amp; name
        </button>
        <button type="button" disabled={busy} onClick={() => act('ignore', {})}>
          Ignore
        </button>
      </div>
      {error && (
        <p role="alert" className="error">
          Could not complete: {error}
        </p>
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
  const [error, setError] = useState(null)
  const [result, setResult] = useState(null)
  const path = `/api/accounts/${account.id}/workspaces`
  const names = workspaces.filter((w) => !w.is_orphan).map((w) => w.name)

  if (names.length === 0) return <p className="muted">No workspaces yet: create a workspace first.</p>

  const toggle = (name) => {
    setResult(null)
    setPicks(picks.includes(name) ? picks.filter((n) => n !== name) : [...picks, name])
  }

  async function review() {
    if (busy) return
    setBusy(true)
    setError(null)
    setResult(null)
    try {
      await sendJson('PUT', path, { workspaces: picks })
      const plan = await getJson(`/api/sync?account=${account.id}`)
      setSteps(plan.steps)
    } catch (e) {
      setError(e.message)
    } finally {
      setBusy(false)
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
    setError(null)
    try {
      const body = await sendJson('POST', '/api/sync', { account: account.id, confirm: true })
      setSaved(picks)
      setSteps(null)
      setResult({ ok: body.status === 0, lines: body.lines ?? [] })
    } catch (e) {
      setError(e.message)
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
            Review changes
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
      {error && (
        <p role="alert" className="error">
          Could not complete: {error}
        </p>
      )}
      {result && (
        <div role="status" className={result.ok ? undefined : 'error'}>
          <pre className="plan">{result.lines.join('\n')}</pre>
        </div>
      )}
    </>
  )
}

function AccountRow({ account, workspaces, onChanged }) {
  const { provider, label, folder, status } = account
  const name = label || folder.split('/').filter(Boolean).pop() || folder
  return (
    <li aria-label={name}>
      <div className="entry-head">
        <strong>{name}</strong>
        <span className={`label ${status === 'confirmed' ? 'label-improve' : status === 'missing' ? 'label-warning' : 'label-aware'}`}>
          {status === 'confirmed' ? 'confirmed ✓' : status}
        </span>
        <span className="muted">{provider}</span>
      </div>
      <p className="muted">{folder}</p>
      {status === 'detected' && <Detected account={account} onChanged={onChanged} />}
      {status === 'confirmed' && provider === 'codex' && <p className="muted">listed only</p>}
      {status === 'confirmed' && provider === 'claude' && <Picker account={account} workspaces={workspaces} />}
    </li>
  )
}

function Accounts({ by, workspaces, onChanged }) {
  const state = useLoad(() => getJson(`/api/accounts?by=${by}`))
  if (state.status === 'loading') return <p role="status">Loading agent accounts</p>
  if (state.status === 'error')
    return (
      <p role="alert" className="error">
        Could not load agent accounts: {state.message}
      </p>
    )
  const groups =
    by === 'provider'
      ? Object.entries(state.data.providers).map(([title, accounts]) => ({ title, accounts }))
      : [{ title: null, accounts: state.data.accounts }]
  return groups.map((g) => (
    <section key={g.title ?? 'all'} aria-label={g.title ?? 'Agent accounts'}>
      {g.title && <h2>{g.title}</h2>}
      <ul className="entries">
        {g.accounts.map((a) => (
          <AccountRow key={a.id} account={a} workspaces={workspaces} onChanged={onChanged} />
        ))}
      </ul>
    </section>
  ))
}

export default function AgentsView() {
  const [by, setBy] = useState('account')
  const [version, setVersion] = useState(0)
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
      {ws.status !== 'loading' && (
        <Accounts key={`${by}-${version}`} by={by} workspaces={workspaces} onChanged={() => setVersion((v) => v + 1)} />
      )}
    </main>
  )
}
