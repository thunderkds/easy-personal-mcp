import { useEffect, useId, useMemo, useRef, useState } from 'react'
import { startAppSession } from './api.js'

// The New session form (T104, FR-109, FR-116). The lists are the server's fixed lists (`app_session.rs` MODELS, MODES); the server checks
// each field again. Nothing starts without the confirm step, and the bypass mode needs a second, explicit one. The first prompt is React
// state only: it is sent once and never kept (FR-115).
export const MODELS = ['default', 'opus', 'sonnet', 'haiku']
export const MODES = ['default', 'acceptEdits', 'auto', 'plan', 'dontAsk', 'bypassPermissions']
export const BYPASS = 'bypassPermissions'
export const MAX_BYTES = 100_000 // the server's limit for a prompt or message: bytes of UTF-8, not characters
export const SIZE_HINT = 'The size limit is 100,000 bytes (about 100 KB). English letters take 1 byte each, most other scripts 2 to 4.'
export const byteLength = (text) => new TextEncoder().encode(text).length
export const NOTE =
  "This starts Claude Code on this machine with this account's subscription. It can run tools and change files in this folder as your Claude Code settings allow, and it counts against the account's plan limits. Permission requests come to you in the session panel."
export const BYPASS_NOTE = 'Bypass permissions runs every tool without asking you. No request will wait for your Allow, and nothing in this session can be refused from here except by Stop. It can change or delete files in this folder.'
export const FORK_NOTE = 'The open window does not see a fork: it becomes a new session.'
export const STILL_ACTIVE = 'this session is still active in another window; fork it instead' // the server's fixed 409 text (T105)
const FAILED = 'Could not start the session. Try again.'
const ID_CHARS = 8

/** The stored repos of the workspaces (orphan has none to start in), flat, in page order. */
export function repoChoices(workspaces) {
  return workspaces.filter((w) => !w.is_orphan).flatMap((w) => (w.repos ?? []).map((repo) => ({ workspace: w.name, repo })))
}

/**
 * `workspaces`: from `/api/workspaces`. `accounts`: the confirmed Claude accounts. `defaultAccount`: the agent run account id ('' = the
 * default Claude Code account). `initialWorkspace`: the workspace whose first repo starts selected. `onStarted(id)` / `onCancel()`.
 * `resume` (T105): `{id, project, account, fork}` opens the form for a Claude session that is already stored: no repo or worktree, the server
 * takes the folder from the session; `account` is the live view's account name, chosen when it is one of the confirmed accounts.
 * `continueIn`: `{workspace, repo}` opens it to continue the newest conversation of that repo (no worktree).
 */
export default function NewSession({ workspaces, accounts, defaultAccount, initialWorkspace, resume = null, continueIn = null, onStarted, onCancel }) {
  const id = useId()
  const repos = useMemo(() => repoChoices(workspaces), [workspaces])
  const groups = useMemo(() => {
    const byName = new Map()
    repos.forEach((r, index) => byName.set(r.workspace, [...(byName.get(r.workspace) ?? []), { index, repo: r.repo }]))
    return [...byName].map(([name, items]) => ({ name, items }))
  }, [repos])
  const accountOptions = useMemo(() => [...(defaultAccount === '' ? [{ value: '', label: 'Default Claude Code account' }] : []), ...accounts.map((a) => ({ value: String(a.id), label: a.label }))], [accounts, defaultAccount])
  const kind = resume ? 'resume' : continueIn ? 'continue' : 'new'
  const [fork, setFork] = useState(Boolean(resume?.fork))
  const [fields, setFields] = useState(() => ({
    repo: Math.max(0, repos.findIndex((r) => r.workspace === initialWorkspace)),
    account: (resume && accountOptions.find((o) => o.label === resume.account)?.value) ?? (accountOptions.some((o) => o.value === defaultAccount) ? defaultAccount : (accountOptions[0]?.value ?? '')),
    model: 'default',
    mode: 'default',
    worktree: false,
    prompt: '',
  }))
  const [step, setStep] = useState('form') // 'form' | 'confirm' | 'bypass'
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  const open = useRef(false) // one request at a time: a second click while one is out is dropped
  const heading = useRef(null)
  useEffect(() => {
    if (step !== 'form') heading.current?.focus()
  }, [step])
  const set = (key) => (e) => setFields((f) => ({ ...f, [key]: e.target.type === 'checkbox' ? e.target.checked : e.target.value }))

  const picked = kind === 'continue' ? continueIn : repos[fields.repo]
  const account = accountOptions.find((o) => o.value === fields.account)
  const ready = (kind === 'resume' || Boolean(picked)) && Boolean(account) && fields.prompt.trim() !== '' && byteLength(fields.prompt) <= MAX_BYTES
  const tooBig = byteLength(fields.prompt) > MAX_BYTES
  const start = async () => {
    if (open.current) return
    open.current = true
    setBusy(true)
    setError('')
    try {
      const common = { account: fields.account === '' ? null : fields.account, model: fields.model, mode: fields.mode, prompt: fields.prompt }
      const answer = await startAppSession(
        kind === 'resume'
          ? { resume: resume.id, ...(fork ? { fork: true } : {}), ...common }
          : kind === 'continue'
            ? { continue: true, repo: picked.repo, workspace: picked.workspace, ...common }
            : { repo: picked.repo, workspace: picked.workspace, ...common, worktree: fields.worktree },
      )
      onStarted(answer.id)
    } catch (e) {
      if (kind === 'resume' && !fork && e.status === 409 && e.message === STILL_ACTIVE) {
        // opened in another window since the page last looked: switch to a fork, show its note, and let the owner confirm again
        setFork(true)
        setStep('confirm')
      }
      setError(e.status && e.message ? e.message : FAILED)
    } finally {
      open.current = false
      setBusy(false)
    }
  }
  const review = (e) => {
    e.preventDefault()
    if (ready) setStep('confirm')
  }

  const title = kind === 'resume' ? (fork ? 'Fork session' : 'Resume session') : kind === 'continue' ? 'Continue session' : 'New session'
  const session = resume && (
    <p className="muted">
      <span className="mono" title={resume.id}>
        {resume.id.slice(0, ID_CHARS)}
      </span>
      {` · ${resume.project}`}
    </p>
  )
  if (kind === 'new' && repos.length === 0)
    return (
      <section className="card" aria-labelledby={`${id}-h`}>
        <h2 id={`${id}-h`}>{title}</h2>
        <p className="muted">No repo is stored yet. Add a repo to a workspace in Workspaces first.</p>
        <div className="actions">
          <button type="button" onClick={onCancel}>
            Cancel
          </button>
        </div>
      </section>
    )

  if (step !== 'form') {
    const bypass = step === 'bypass'
    return (
      <section className="card new-session" aria-labelledby={`${id}-h`}>
        <h2 id={`${id}-h`} ref={heading} tabIndex={-1}>
          {bypass ? 'Skip every permission prompt?' : fork ? 'Start a fork of this session?' : 'Start this session?'}
        </h2>
        <dl className="session-facts">
          {kind === 'resume' ? (
            <>
              <dt>Session</dt>
              <dd>
                <span className="mono" title={resume.id}>
                  {resume.id.slice(0, ID_CHARS)}
                </span>
                {` · ${resume.project}`}
              </dd>
              <dt>Resume</dt>
              <dd>{fork ? 'a fork: a new session that starts from this conversation' : 'the same session, continued here'}</dd>
            </>
          ) : (
            <>
              <dt>Repo</dt>
              <dd className="mono">{picked.repo}</dd>
              {kind === 'continue' && (
                <>
                  <dt>Continue</dt>
                  <dd>the newest conversation of this folder</dd>
                </>
              )}
            </>
          )}
          <dt>Account</dt>
          <dd>{account.label}</dd>
          <dt>Mode</dt>
          <dd>{fields.mode}</dd>
          <dt>Model</dt>
          <dd>{fields.model}</dd>
          {kind === 'new' && (
            <>
              <dt>Worktree</dt>
              <dd>{fields.worktree ? 'a new worktree on a new branch' : 'no, the repo folder itself'}</dd>
            </>
          )}
        </dl>
        <p>{NOTE}</p>
        {fork && <p className="prompt-cut">{FORK_NOTE}</p>}
        {bypass && <p className="prompt-cut">{BYPASS_NOTE}</p>}
        {error && (
          <p role="alert" className="field-error">
            {error}
          </p>
        )}
        <div className="actions">
          {fields.mode === BYPASS && !bypass ? (
            <button type="button" className="primary" onClick={() => setStep('bypass')}>
              Continue
            </button>
          ) : (
            <button type="button" className={bypass ? 'danger' : 'primary'} disabled={busy} onClick={start}>
              {bypass ? 'Start without permission prompts' : 'Start session'}
            </button>
          )}
          <button type="button" disabled={busy} onClick={() => setStep(bypass ? 'confirm' : 'form')}>
            Back
          </button>
        </div>
      </section>
    )
  }

  return (
    <section className="card new-session" aria-labelledby={`${id}-h`}>
      <h2 id={`${id}-h`}>{title}</h2>
      {session}
      {fork && <p className="prompt-cut">{FORK_NOTE}</p>}
      <form className="new-session-form" onSubmit={review}>
        {kind === 'new' && (
          <label className="field" htmlFor={`${id}-repo`}>
            Repo
            <select id={`${id}-repo`} value={fields.repo} onChange={(e) => setFields((f) => ({ ...f, repo: Number(e.target.value) }))}>
              {groups.map((g) => (
                <optgroup key={g.name} label={g.name}>
                  {g.items.map((r) => (
                    <option key={r.index} value={r.index}>
                      {r.repo}
                    </option>
                  ))}
                </optgroup>
              ))}
            </select>
          </label>
        )}
        {kind === 'continue' && (
          <p className="field">
            Repo
            <span className="mono">{picked.repo}</span>
            <span className="muted">Continues the newest conversation of this folder.</span>
          </p>
        )}
        <label className="field" htmlFor={`${id}-account`}>
          Account
          <select id={`${id}-account`} value={fields.account} onChange={set('account')}>
            {accountOptions.map((o) => (
              <option key={o.value} value={o.value}>
                {o.label}
              </option>
            ))}
          </select>
        </label>
        <label className="field" htmlFor={`${id}-model`}>
          Model
          <select id={`${id}-model`} value={fields.model} onChange={set('model')}>
            {MODELS.map((m) => (
              <option key={m} value={m}>
                {m}
              </option>
            ))}
          </select>
        </label>
        <label className="field" htmlFor={`${id}-mode`}>
          Permission mode
          <select id={`${id}-mode`} value={fields.mode} onChange={set('mode')}>
            {MODES.map((m) => (
              <option key={m} value={m}>
                {m}
              </option>
            ))}
          </select>
        </label>
        {kind === 'new' && (
          <label className="check" htmlFor={`${id}-worktree`}>
            <input id={`${id}-worktree`} type="checkbox" checked={fields.worktree} onChange={set('worktree')} />
            New worktree (a new branch in its own folder; nothing is committed, merged or deleted for you)
          </label>
        )}
        <label className="field new-session-prompt" htmlFor={`${id}-prompt`}>
          First prompt
          <textarea id={`${id}-prompt`} rows={4} value={fields.prompt} onChange={set('prompt')} />
        </label>
        <p className="muted">{SIZE_HINT}</p>
        {tooBig && (
          <p role="alert" className="field-error">
            Too long to start with. Shorten the prompt.
          </p>
        )}
        <div className="actions">
          <button type="submit" className="primary" disabled={!ready}>
            Review
          </button>
          <button type="button" onClick={onCancel}>
            Cancel
          </button>
        </div>
      </form>
    </section>
  )
}
