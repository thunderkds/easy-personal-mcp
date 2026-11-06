import { useEffect, useRef, useState } from 'react'
import { getJson, sendJson } from './api.js'
import { costFigure } from './format.js'
import Loading from './Loading.jsx'
import { useNotify } from './Toast.jsx'

const localDate = (utc) => {
  const d = new Date(utc)
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`
}

/**
 * The repos with usage that no workspace covers (FR-070), read on its own so the page never waits for it.
 * `remove(path)` drops a row after it was added to a workspace.
 */
export function useUnassigned() {
  const [state, setState] = useState({ status: 'loading' })
  useEffect(() => {
    let current = true
    getJson('/api/workspaces/unassigned').then(
      (body) => current && setState({ status: 'ready', repos: body.repos }),
      (error) => current && setState({ status: 'error', message: error.message }),
    )
    return () => {
      current = false
    }
  }, [])
  const remove = (path) => setState((s) => (s.status === 'ready' ? { ...s, repos: s.repos.filter((r) => r.path !== path) } : s))
  return { ...state, remove }
}

/** One row: what the repo is, and (when a workspace exists) a select + Add. Only one Add runs at a time. */
function RepoRow({ repo, workspaces, busy, onAdd }) {
  const [target, setTarget] = useState('')
  const running = busy === repo.path
  return (
    <li aria-label={repo.name}>
      <div className="unassigned-info">
        <strong>{repo.name}</strong>
        <span className="mono-ellipsis" title={repo.path}>{repo.path}</span>
        <span className="muted">
          {`${repo.sessions === 1 ? '1 session' : `${repo.sessions} sessions`} · ${costFigure(repo.api_equivalent_cost)} · last used ${localDate(repo.last_used_utc)}`}
        </span>
      </div>
      {workspaces.length === 0 ? (
        <p className="muted">Add a workspace first</p>
      ) : (
        <div className="actions">
          <select aria-label="Add to workspace" value={target} disabled={busy !== null} onChange={(e) => setTarget(e.target.value)}>
            <option value="">Choose a workspace</option>
            {workspaces.map((w) => (
              <option key={w.id} value={w.id}>
                {w.name}
              </option>
            ))}
          </select>
          <button type="button" disabled={busy !== null || target === ''} onClick={() => onAdd(repo, target)}>
            {running ? 'Adding…' : 'Add'}
          </button>
        </div>
      )}
    </li>
  )
}

/**
 * The card "Repos not in a workspace (N)". Add reads the chosen workspace fresh, then sends its repos plus this path
 * through the existing `PATCH /api/workspaces/{id}` (no new write route). `onAdded` reloads the page's workspaces.
 */
export default function UnassignedRepos({ unassigned, workspaces, onAdded }) {
  const notify = useNotify()
  const [busy, setBusy] = useState(null)
  const busyRef = useRef(false)
  const targets = workspaces.filter((w) => !w.is_orphan)

  async function add(repo, workspaceId) {
    if (busyRef.current) return
    busyRef.current = true
    setBusy(repo.path)
    try {
      const fresh = (await getJson('/api/workspaces')).workspaces.find((w) => String(w.id) === String(workspaceId))
      if (!fresh) throw new Error('that workspace no longer exists')
      const repos = fresh.repos.includes(repo.path) ? fresh.repos : [...fresh.repos, repo.path]
      const answer = await sendJson('PATCH', `/api/workspaces/${fresh.id}`, { repos })
      if (answer.pending || (answer.status !== undefined && answer.status !== 0)) throw new Error('nothing was changed')
      unassigned.remove(repo.path)
      notify('success', `Added ${repo.name} to ${fresh.name}`)
      await onAdded()
    } catch (e) {
      notify('error', `Could not add ${repo.name}: ${e.message}`)
    } finally {
      busyRef.current = false
      setBusy(null)
    }
  }

  const count = unassigned.status === 'ready' ? ` (${unassigned.repos.length})` : ''
  return (
    <section aria-label="Repos not in a workspace">
      <h2>{`Repos not in a workspace${count}`}</h2>
      {unassigned.status === 'loading' && <Loading label="Loading repos not in a workspace" />}
      {unassigned.status === 'error' && (
        <p role="alert" className="error">
          Could not load repos not in a workspace: {unassigned.message}
        </p>
      )}
      {unassigned.status === 'ready' && unassigned.repos.length === 0 && <p className="muted">Every repo with usage is in a workspace.</p>}
      {unassigned.status === 'ready' && unassigned.repos.length > 0 && (
        <ul className="unassigned">
          {unassigned.repos.map((r) => (
            <RepoRow key={r.path} repo={r} workspaces={targets} busy={busy} onAdd={add} />
          ))}
        </ul>
      )}
    </section>
  )
}
