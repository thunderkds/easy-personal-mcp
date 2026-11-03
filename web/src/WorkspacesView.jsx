import { useEffect, useRef, useState } from 'react'
import { getJson, sendJson } from './api.js'
import FolderPicker from './FolderPicker.jsx'
import Loading from './Loading.jsx'
import { useNotify } from './Toast.jsx'

const optional = (promise, fallback) => promise.catch(() => fallback)
const emptyToNull = (text) => (text.trim() === '' ? null : text.trim())
const HEX_COLOR = /^#[0-9a-fA-F]{6}$/
const EMAIL = /^[^\s@]+@[^\s@]+\.[^\s@]+$/
const accountName = (a) => a.label || a.folder.split('/').filter(Boolean).pop() || a.folder
/** Ids of the hint and error spans of a field that its input names in `aria-describedby`. */
const described = (id, hint, error) => [hint && `${id}-hint`, error && `${id}-error`].filter(Boolean).join(' ') || undefined

/** The hint (what format) and the error (what is wrong) under a field. */
function FieldNotes({ id, hint, error }) {
  return (
    <>
      {hint && (
        <span id={`${id}-hint`} className="hint">
          {hint}
        </span>
      )}
      {error && (
        <span id={`${id}-error`} className="field-error">
          {error}
        </span>
      )}
    </>
  )
}

/** The first problem of each field that has one; empty colour and email are allowed. Mirrors the server rules. */
function check({ name, color, email }) {
  const errors = {}
  if (!name.trim()) errors.name = 'Name is required'
  if (color.trim() && !HEX_COLOR.test(color.trim())) errors.color = 'Use #rrggbb, for example #0b5fd6'
  if (email.trim() && !EMAIL.test(email.trim())) errors.email = 'Enter a valid email, for example name@example.com'
  return errors
}

/**
 * The lists the page shows. Workspaces and the catalog are required; accounts are optional. The legacy scan is slow
 * (it asks `claude` once per connector), so it loads on its own: the page renders without it and the banner follows.
 */
function useWorkspaceData() {
  const [state, setState] = useState({ status: 'loading' })
  const [legacy, setLegacy] = useState([])
  const alive = useRef(true)
  const legacyRun = useRef(0)
  function reload() {
    const run = ++legacyRun.current
    optional(getJson('/api/connectors/legacy'), { legacy: [] }).then((l) => {
      if (alive.current && run === legacyRun.current) setLegacy(l.legacy ?? [])
    })
    return Promise.all([
      getJson('/api/workspaces'),
      getJson('/api/connectors'),
      optional(getJson('/api/accounts'), { accounts: [] }),
    ]).then(
      ([w, c, a]) =>
        alive.current &&
        setState({
          status: 'ready',
          data: {
            workspaces: w.workspaces,
            catalog: c.connectors,
            connectors: c.workspace_connectors ?? [],
            accounts: a.accounts ?? [],
          },
        }),
      (e) => alive.current && setState((s) => (s.status === 'ready' ? s : { status: 'error', message: e.message })),
    )
  }
  useEffect(() => {
    alive.current = true
    reload()
    return () => {
      alive.current = false
    }
  }, [])
  return [state.status === 'ready' ? { ...state, data: { ...state.data, legacy } } : state, reload]
}

/**
 * One write at a time for the whole page: ask for the plan (confirm false), show it, send confirm true.
 * The answer and any error float as toasts. `review(send)` resolves 'plan' (waiting for Confirm), 'final' (answered) or 'error'.
 */
function useAction(onDone) {
  const notify = useNotify()
  const [plan, setPlan] = useState(null)
  const [busy, setBusy] = useState(false)
  const busyRef = useRef(false)
  const doneRef = useRef(onDone)
  doneRef.current = onDone

  async function go(send, confirm) {
    if (busyRef.current) return 'error'
    busyRef.current = true
    setBusy(true)
    try {
      const body = await send(confirm)
      if (!confirm && body.pending) {
        setPlan({ lines: body.lines, send })
        return 'plan'
      }
      const ok = body.status === 0
      const lines = body.lines?.length ? body.lines : [ok ? 'Done.' : 'Nothing was changed.']
      setPlan(null)
      notify(ok ? 'success' : 'error', lines.join('\n'))
      await doneRef.current(body)
      return 'final'
    } catch (e) {
      notify('error', `Could not complete: ${e.message}`)
      return 'error'
    } finally {
      busyRef.current = false
      setBusy(false)
    }
  }

  return {
    plan,
    busy,
    locked: busy || plan !== null,
    review: (send) => go(send, false),
    confirm: () => go(plan.send, true),
    cancel: () => setPlan(null),
    reset: () => setPlan(null),
  }
}

/** The plan with Confirm / Cancel. It takes focus when it appears. The answer is a toast, not part of this block. */
function Changes({ act }) {
  const ref = useRef(null)
  useEffect(() => {
    if (act.plan) ref.current?.focus()
  }, [act.plan])
  if (!act.plan) return null
  return (
    <section role="group" aria-label="Changes" tabIndex={-1} ref={ref}>
      <p>This will change:</p>
      <pre className="plan">{act.plan.lines.join('\n')}</pre>
      <div className="actions">
        <button type="button" onClick={act.confirm} disabled={act.busy}>
          Confirm
        </button>
        <button type="button" onClick={act.cancel} disabled={act.busy}>
          Cancel
        </button>
      </div>
    </section>
  )
}

/** One stored connector: alias edit, Move to… (Assign… in orphan) and Delete. Every button asks for a plan first. */
function ConnectorRow({ connector: c, workspaces, orphan, act }) {
  const [alias, setAlias] = useState(c.alias)
  const [target, setTarget] = useState('')
  const targets = workspaces.filter((w) => w.name !== c.workspace && !(orphan && w.is_orphan))
  const url = `/api/connectors/${encodeURIComponent(c.id)}`
  const verb = orphan ? 'Assign' : 'Move'
  const placeholder = `${verb} to…`
  return (
    <li aria-label={c.alias}>
      <div className="entry-head">
        <strong>{c.alias}</strong>
        <span className="muted">{c.provider}</span>
      </div>
      <div className="field">
        <label htmlFor={`alias-${c.id}`}>Alias</label>
        <input
          id={`alias-${c.id}`}
          value={alias}
          aria-describedby={`alias-${c.id}-hint`}
          autoComplete="off"
          spellCheck={false}
          onChange={(e) => setAlias(e.target.value)}
        />
        <FieldNotes id={`alias-${c.id}`} hint="Leave empty for the default alias." />
      </div>
      <div className="actions">
        <button
          type="button"
          disabled={act.locked || alias === c.alias}
          onClick={() => act.review((confirm) => sendJson('PUT', `${url}/alias`, { alias: emptyToNull(alias), confirm }))}
        >
          Save alias
        </button>
      </div>
      <div className="actions">
        <select aria-label={placeholder} value={target} onChange={(e) => setTarget(e.target.value)} disabled={act.locked}>
          <option value="">{placeholder}</option>
          {targets.map((w) => (
            <option key={w.id} value={w.name}>
              {w.is_orphan ? 'orphan (unassign)' : w.name}
            </option>
          ))}
        </select>
        <button
          type="button"
          disabled={act.locked || target === ''}
          onClick={() => act.review((confirm) => sendJson('POST', `${url}/move`, { workspace: target, confirm }))}
        >
          {verb}
        </button>
        <button type="button" disabled={act.locked} onClick={() => act.review((confirm) => sendJson('DELETE', url, { confirm }))}>
          Delete
        </button>
      </div>
    </li>
  )
}

/** Add a connector to this workspace: pick a provider, fill the catalog form, see the plan, confirm. */
function AddConnector({ workspace, data, act }) {
  const [open, setOpen] = useState(false)
  const [providerId, setProviderId] = useState('')
  const [alias, setAlias] = useState('')
  const [values, setValues] = useState({})
  const [submitted, setSubmitted] = useState(false)
  const [reviewing, setReviewing] = useState(false)

  const held = data.connectors.filter((c) => c.workspace === workspace.name).map((c) => c.provider)
  const available = data.catalog.filter((e) => !held.includes(e.id))
  const entry = available.find((e) => e.id === providerId)

  function close() {
    // Typed values never outlive the form.
    setValues({})
    setAlias('')
    setProviderId('')
    setSubmitted(false)
    setOpen(false)
  }

  // After the owner answers the plan (Confirm or Cancel) the form is closed and emptied.
  useEffect(() => {
    if (submitted && !act.plan && !act.busy) close()
  }, [submitted, act.plan, act.busy])

  async function review(e) {
    e.preventDefault()
    const send = (confirm) =>
      sendJson('POST', '/api/connectors', {
        provider: entry.id,
        workspace: workspace.name,
        ...(alias.trim() ? { alias: alias.trim() } : {}),
        values,
        confirm,
      })
    setReviewing(true)
    const outcome = await act.review(send)
    setReviewing(false)
    if (outcome === 'plan') setSubmitted(true)
    else if (outcome === 'final') close()
  }

  if (!open)
    return (
      <div className="actions">
        <button type="button" disabled={act.locked || available.length === 0} onClick={() => setOpen(true)}>
          Add connector
        </button>
      </div>
    )
  const showFields = entry && !submitted
  return (
    <form onSubmit={review} aria-label="Add connector">
      <div className="field">
        <label htmlFor="add-provider">Provider</label>
        <select
          id="add-provider"
          value={providerId}
          disabled={submitted}
          onChange={(e) => {
            setProviderId(e.target.value)
            setValues({})
          }}
        >
          <option value="">Choose a provider</option>
          {available.map((e) => (
            <option key={e.id} value={e.id}>
              {e.provider}
            </option>
          ))}
        </select>
      </div>
      {showFields && entry.claude_ai_connector && (
        <p>
          Official claude.ai connector available:{' '}
          <a href={entry.claude_ai_link} target="_blank" rel="noopener noreferrer">
            add it in claude.ai
          </a>
          . Use the form below only if you want this launcher instead.
        </p>
      )}
      {showFields && (
        <div className="form-grid">
          {entry.fields.map((f) => (
            <div className="field" key={f.env}>
              <label htmlFor={`add-${f.env}`}>{f.label}</label>
              <input
                id={`add-${f.env}`}
                name={f.env}
                type={f.secret ? 'password' : 'text'}
                autoComplete="off"
                spellCheck={false}
                value={values[f.env] ?? ''}
                onChange={(e) => setValues({ ...values, [f.env]: e.target.value })}
              />
            </div>
          ))}
          <div className="field">
            <label htmlFor="add-alias">Alias (optional)</label>
            <input
              id="add-alias"
              value={alias}
              aria-describedby="add-alias-hint"
              autoComplete="off"
              spellCheck={false}
              onChange={(e) => setAlias(e.target.value)}
            />
            <FieldNotes id="add-alias" hint="Leave empty for the default alias." />
          </div>
        </div>
      )}
      <div className="actions">
        {showFields && (
          <button type="submit" disabled={act.locked}>
            {reviewing ? 'Reviewing…' : 'Review changes'}
          </button>
        )}
        <button type="button" onClick={close} disabled={act.busy}>
          Close
        </button>
      </div>
    </form>
  )
}

/** Add or edit one workspace: fields, repos, its connectors. Not offered for orphan. */
function WorkspaceForm({ workspace, data, act, onBack }) {
  const isNew = workspace === null
  const [name, setName] = useState(workspace?.name ?? '')
  const [color, setColor] = useState(workspace?.color ?? '')
  const [icon, setIcon] = useState(workspace?.icon ?? '')
  const [email, setEmail] = useState(workspace?.email ?? '')
  const [description, setDescription] = useState(workspace?.description ?? '')
  const [repos, setRepos] = useState(workspace?.repos ?? [])
  const [errors, setErrors] = useState({})
  const [pending, setPending] = useState(false)
  const [browsing, setBrowsing] = useState(false)
  const browseButton = useRef(null)
  const refs = { name: useRef(null), color: useRef(null), email: useRef(null) }
  const mine = isNew ? [] : data.connectors.filter((c) => c.workspace === workspace.name)
  const clear = (field) => setErrors((e) => (e[field] ? { ...e, [field]: undefined } : e))
  const colorValue = HEX_COLOR.test(color.trim()) ? color.trim().toLowerCase() : '#000000'

  const closeBrowser = () => setBrowsing(false)
  const wasBrowsing = useRef(false)
  useEffect(() => {
    // The dialog is gone from the page once `browsing` is false: only then can the focus go back to Browse…
    if (browsing) wasBrowsing.current = true
    else if (wasBrowsing.current) {
      wasBrowsing.current = false
      browseButton.current?.focus()
    }
  }, [browsing])

  async function save(e) {
    e.preventDefault()
    const found = check({ name, color, email })
    setErrors(found)
    const first = ['name', 'color', 'email'].find((k) => found[k])
    if (first) {
      refs[first].current?.focus()
      return
    }
    const fields = {
      name: name.trim(),
      color: emptyToNull(color),
      icon: emptyToNull(icon),
      email: emptyToNull(email),
      description: emptyToNull(description),
      repos: repos.map((r) => r.trim()).filter(Boolean),
    }
    setPending(true)
    try {
      if (isNew)
        await act.review(async () => {
          const body = await sendJson('POST', '/api/workspaces', fields)
          return { status: 0, pending: false, lines: [`Workspace '${body.workspace.name}' created.`], workspace: body.workspace }
        })
      else await act.review((confirm) => sendJson('PATCH', `/api/workspaces/${workspace.id}`, { ...fields, confirm }))
    } finally {
      setPending(false)
    }
  }

  return (
    <section aria-label="Workspace form">
      <h2>{isNew ? 'New workspace' : `Edit ${workspace.name}`}</h2>
      <form onSubmit={save} noValidate>
        <div className="form-grid">
          <div className="field">
            <label htmlFor="ws-name" className="required">
              Name
            </label>
            <input
              id="ws-name"
              ref={refs.name}
              value={name}
              aria-required="true"
              aria-invalid={errors.name ? 'true' : undefined}
              aria-describedby={described('ws-name', null, errors.name)}
              autoComplete="off"
              spellCheck={false}
              onChange={(e) => {
                setName(e.target.value)
                clear('name')
              }}
            />
            <FieldNotes id="ws-name" error={errors.name} />
          </div>
          <div className="field">
            <label htmlFor="ws-color">Color</label>
            <div className="color-row">
              <input
                id="ws-color"
                ref={refs.color}
                value={color}
                placeholder="#0b5fd6"
                aria-invalid={errors.color ? 'true' : undefined}
                aria-describedby={described('ws-color', 'Optional. Format #rrggbb, or pick one.', errors.color)}
                autoComplete="off"
                spellCheck={false}
                onChange={(e) => {
                  setColor(e.target.value)
                  clear('color')
                }}
              />
              <input
                type="color"
                aria-label="Color picker"
                value={colorValue}
                onChange={(e) => {
                  setColor(e.target.value)
                  clear('color')
                }}
              />
            </div>
            <FieldNotes id="ws-color" hint="Optional. Format #rrggbb, or pick one." error={errors.color} />
          </div>
          <div className="field">
            <label htmlFor="ws-icon">Icon</label>
            <input
              id="ws-icon"
              value={icon}
              maxLength={32}
              aria-describedby="ws-icon-hint"
              autoComplete="off"
              onChange={(e) => setIcon(e.target.value)}
            />
            <FieldNotes id="ws-icon" hint="Optional. Short text, up to 32 characters." />
          </div>
          <div className="field">
            <label htmlFor="ws-email">Email</label>
            <input
              id="ws-email"
              ref={refs.email}
              type="email"
              value={email}
              aria-invalid={errors.email ? 'true' : undefined}
              aria-describedby={described('ws-email', 'Optional. The main email of this workspace.', errors.email)}
              autoComplete="off"
              onChange={(e) => {
                setEmail(e.target.value)
                clear('email')
              }}
            />
            <FieldNotes id="ws-email" hint="Optional. The main email of this workspace." error={errors.email} />
          </div>
          <div className="field wide">
            <label htmlFor="ws-description">Description</label>
            <textarea id="ws-description" rows={2} value={description} onChange={(e) => setDescription(e.target.value)} />
          </div>
          <fieldset className="group wide" aria-describedby="ws-repos-hint">
            <legend>Repos</legend>
            <FieldNotes id="ws-repos" hint="Optional. One absolute path per repo, for example /home/me/projects/acme." />
            {repos.map((r, i) => (
              <div className="repo-row" key={i}>
                <input
                  aria-label={`Repo ${i + 1}`}
                  value={r}
                  placeholder="/absolute/path/to/repo"
                  autoComplete="off"
                  spellCheck={false}
                  onChange={(e) => setRepos(repos.map((x, j) => (j === i ? e.target.value : x)))}
                />
                <button type="button" aria-label={`Remove repo ${i + 1}`} onClick={() => setRepos(repos.filter((_, j) => j !== i))}>
                  ✕
                </button>
              </div>
            ))}
            <div className="actions">
              <button type="button" ref={browseButton} onClick={() => setBrowsing(true)}>
                Browse…
              </button>
              <button type="button" onClick={() => setRepos([...repos, ''])}>
                + Type a path
              </button>
            </div>
          </fieldset>
        </div>
        <div className="actions form-actions">
          <button type="submit" disabled={act.locked}>
            {pending ? 'Saving…' : 'Save'}
          </button>
          <button type="button" onClick={onBack} disabled={act.busy}>
            Back to workspaces
          </button>
        </div>
      </form>
      {browsing && (
        <FolderPicker
          onChoose={(path) => {
            setRepos((list) => (list.some((r) => r.trim() === path) ? list : [...list, path]))
            closeBrowser()
          }}
          onClose={closeBrowser}
        />
      )}
      <Changes act={act} />

      {!isNew && (
        <>
          <h3>Connectors</h3>
          {mine.length === 0 ? (
            <p className="muted">No connectors in this workspace yet.</p>
          ) : (
            <ul className="entries">
              {mine.map((c) => (
                <ConnectorRow key={c.id} connector={c} workspaces={data.workspaces} orphan={false} act={act} />
              ))}
            </ul>
          )}
          <AddConnector workspace={workspace} data={data} act={act} />
          <h3>Delete workspace</h3>
          <p className="muted">
            Deleting this workspace keeps its connectors: connectors move to orphan (active in no agent account) and can be assigned
            again later.
          </p>
          <div className="actions">
            <button type="button" disabled={act.locked} onClick={() => act.review((confirm) => sendJson('DELETE', `/api/workspaces/${workspace.id}`, { confirm }))}>
              Delete workspace
            </button>
          </div>
        </>
      )}
    </section>
  )
}

function WorkspaceCard({ workspace: w, data, act, onOpen }) {
  const mine = data.connectors.filter((c) => c.workspace === w.name)
  const users = data.accounts.filter((a) => a.provider === 'claude' && a.workspaces?.includes(w.name)).map(accountName)
  return (
    <section aria-label={w.name}>
      <div className="entry-head">
        <span className="swatch" data-testid="swatch" aria-hidden="true" style={w.color ? { backgroundColor: w.color } : undefined}>
          {w.icon}
        </span>
        <h2>{w.name}</h2>
      </div>
      {w.is_orphan ? (
        <>
          <p className="muted">Not active in any agent account. Assign a connector to a workspace to use it.</p>
          {mine.length === 0 ? (
            <p className="muted">No connectors in orphan.</p>
          ) : (
            <ul className="entries">
              {mine.map((c) => (
                <ConnectorRow key={c.id} connector={c} workspaces={data.workspaces} orphan act={act} />
              ))}
            </ul>
          )}
        </>
      ) : (
        <>
          {w.email && <p>{w.email}</p>}
          <p>
            <span>{w.repos.length === 1 ? '1 repo' : `${w.repos.length} repos`}</span>
          </p>
          {mine.length === 0 ? (
            <p className="muted">No connectors.</p>
          ) : (
            <ul className="aliases">
              {mine.map((c) => (
                <li key={c.id}>{c.alias}</li>
              ))}
            </ul>
          )}
          <p className="muted">Used by: {users.length ? users.join(', ') : 'no agent account'}</p>
          <div className="actions">
            <button type="button" onClick={onOpen} disabled={act.locked}>
              Open
            </button>
          </div>
        </>
      )}
    </section>
  )
}

/** Connectors set up before workspaces: offer the one-time adoption into orphan. */
function LegacyBanner({ legacy, act }) {
  const foreign = legacy.filter((l) => l.foreign).map((l) => l.id)
  return (
    <section className="notice" aria-label="Legacy connectors">
      <p>
        {legacy.length === 1 ? '1 connector was' : `${legacy.length} connectors were`} set up before workspaces:{' '}
        {legacy.map((l) => l.id).join(', ')}. Adopt {legacy.length === 1 ? 'it' : 'them'} into the orphan workspace to manage {legacy.length === 1 ? 'it' : 'them'} here.
      </p>
      {foreign.length > 0 && (
        <p className="muted">
          A Claude Code entry with the same name as {foreign.join(', ')} was not created by easy-personal and stays untouched.
        </p>
      )}
      <div className="actions">
        <button type="button" disabled={act.locked} onClick={() => act.review((confirm) => sendJson('POST', '/api/connectors/legacy', { confirm }))}>
          Adopt
        </button>
      </div>
    </section>
  )
}

export default function WorkspacesView() {
  const [state, reload] = useWorkspaceData()
  const [mode, setMode] = useState(null) // null = cards; 'new'; or { id }
  const act = useAction(async (body) => {
    await reload()
    if (body.deleted != null) setMode(null)
    else if (mode === 'new' && body.workspace) setMode({ id: body.workspace.id })
  })

  const header = (
    <>
      <h1>Workspaces</h1>
      <p className="muted">
        A workspace groups repos and connectors. Secrets go to your OS keychain and are never shown again. Every change is shown before
        it is written.
      </p>
    </>
  )
  if (state.status === 'loading') return <main>{header}<Loading label="Loading workspaces" /></main>
  if (state.status === 'error')
    return (
      <main>
        {header}
        <p role="alert" className="error">
          Could not load workspaces: {state.message}
        </p>
      </main>
    )

  const data = state.data
  const editing = mode && mode !== 'new' ? data.workspaces.find((w) => w.id === mode.id && !w.is_orphan) : null
  const formOpen = mode === 'new' || editing
  const back = () => {
    act.reset()
    setMode(null)
  }
  return (
    <main className="workspaces">
      {header}
      {formOpen ? (
        <WorkspaceForm key={editing ? editing.id : 'new'} workspace={editing ?? null} data={data} act={act} onBack={back} />
      ) : (
        <>
          {data.legacy.length > 0 && <LegacyBanner legacy={data.legacy} act={act} />}
          <Changes act={act} />
          {data.workspaces.every((w) => w.is_orphan) && <p className="muted">No workspaces yet. Add one to group repos and connectors.</p>}
          <div className="actions">
            <button
              type="button"
              disabled={act.locked}
              onClick={() => {
                act.reset()
                setMode('new')
              }}
            >
              + Add workspace
            </button>
          </div>
          <div className="cards">
            {data.workspaces.map((w) => (
              <WorkspaceCard
                key={w.id}
                workspace={w}
                data={data}
                act={act}
                onOpen={() => {
                  act.reset()
                  setMode({ id: w.id })
                }}
              />
            ))}
          </div>
        </>
      )}
    </main>
  )
}
