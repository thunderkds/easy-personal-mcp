import { useEffect, useState } from 'react'
import { getDiff, getJson } from './api.js'
import { fmt } from './format.js'
import Loading from './Loading.jsx'
import { NO_SCOPE } from './Scope.jsx'
import { folderName } from './SessionPanel.jsx'
import { useLoad } from './useLoad.js'
import './changes.css'

const MODES = [
  ['uncommitted', 'Uncommitted'],
  ['branch', 'Branch'],
  ['commit', 'Commit'],
  ['range', 'Two refs'],
]
const AREAS = [
  ['staged', 'Staged'],
  ['unstaged', 'Not staged'],
  ['untracked', 'Untracked'],
  ['committed', 'Committed'],
]
const REF_GROUPS = [
  ['branch', 'Branches', 'refs/heads/'],
  ['remote', 'Remote branches', 'refs/remotes/'],
  ['tag', 'Tags', 'refs/tags/'],
]
const SESSION = 'session'

/** The workspace repos the owner can pick, as `{key, query, label}`; `repo_no` is the repo's place in the workspace's list. */
function repoChoices(workspaces, scope) {
  return workspaces
    .filter((w) => !w.is_orphan && (!scope.workspace || w.name === scope.workspace))
    .flatMap((w) =>
      w.repos.map((path, n) => ({
        key: `${w.name}/${n}`,
        query: { workspace: w.name, repo_no: String(n) },
        label: `${folderName(path)} (${w.name})`,
      })),
    )
}

const url = (route, query) => `/api/diff/${route}?${new URLSearchParams(query)}`

/**
 * The answer of `path` (null: ask nothing), read again when it or `round` changes. While the same path is read again the
 * last answer stays on the page; another path's answer is never shown.
 */
function useDiff(path, round) {
  const [answer, setAnswer] = useState(null)
  useEffect(() => {
    if (path === null) return undefined
    let current = true
    getDiff(path).then(
      (body) => current && setAnswer({ path, status: 'ready', body }),
      (error) => current && setAnswer({ path, status: 'error', message: error.message }),
    )
    return () => {
      current = false
    }
  }, [path, round])
  if (path === null) return { status: 'idle' }
  return answer?.path === path ? answer : { status: 'loading' }
}

/** Each line of a patch with its kind: `meta` (file headers), `hunk`, `add`, `del` or `same`. A conflict has two mark columns. */
export function patchLines(patch) {
  if (!patch) return []
  let columns = 0 // 0: outside a hunk
  return patch
    .replace(/\n$/, '')
    .split('\n')
    .map((text) => {
      if (text.startsWith('@@')) {
        columns = text.startsWith('@@@') ? 2 : 1
        return { text, kind: 'hunk' }
      }
      if (text.startsWith('diff ') || text.startsWith('\\')) {
        if (text.startsWith('diff ')) columns = 0
        return { text, kind: 'meta' }
      }
      if (columns === 0) return { text, kind: 'meta' }
      const marks = text.slice(0, columns)
      return { text, kind: marks.includes('+') ? 'add' : marks.includes('-') ? 'del' : 'same' }
    })
}

function Patch({ file, state }) {
  if (state.status === 'loading') return <Loading label="Loading the changes" />
  if (state.status === 'error') {
    return (
      <p role="alert" className="error">
        {state.message}
      </p>
    )
  }
  const { patch, cut, binary, note } = state.body
  return (
    <>
      <p className="mono diff-name">{file.shown}</p>
      {note && <p className="muted">{note}</p>}
      {binary && <p className="muted">A binary file: its content is not shown.</p>}
      {cut && <p className="muted">The patch is cut: only its beginning is shown.</p>}
      {patch && (
        <div className="diff-patch" role="region" aria-label={`Changes in ${file.shown}`} tabIndex={0}>
          <div className="diff-lines">
            {patchLines(patch).map((line, i) => (
              <div key={i} className="diff-line" data-kind={line.kind}>
                {line.text || ' '}
              </div>
            ))}
          </div>
        </div>
      )}
    </>
  )
}

function FileButton({ file, pressed, onOpen }) {
  return (
    <button type="button" className="diff-file" aria-pressed={pressed} disabled={!file.openable} onClick={onOpen}>
      <span className="diff-status" data-status={file.status}>
        {file.status}
      </span>
      <span className="diff-path">{file.shown_old ? `${file.shown_old} → ${file.shown}` : file.shown}</span>
      <span className="diff-count">{file.binary ? 'binary' : file.added === null ? '' : `+${fmt(file.added)} −${fmt(file.removed)}`}</span>
    </button>
  )
}

function RefSelect({ label, value, branches, commits, onChange }) {
  return (
    <label className="field">
      {label}
      <select value={value} onChange={(e) => onChange(e.target.value)}>
        {REF_GROUPS.map(([kind, title, prefix]) => {
          const found = branches.filter((b) => b.kind === kind)
          return found.length === 0 ? null : (
            <optgroup key={kind} label={title}>
              {found.map((b) => (
                <option key={b.name} value={prefix + b.name}>
                  {b.shown}
                </option>
              ))}
            </optgroup>
          )
        })}
        {commits.length > 0 && (
          <optgroup label="Commits">
            {commits.map((c) => (
              <option key={c.id} value={c.id}>
                {commitLabel(c)}
              </option>
            ))}
          </optgroup>
        )}
      </select>
    </label>
  )
}

const commitLabel = (c) => `${c.id.slice(0, 7)} ${c.subject}`

/** One repository: its work tree, compare mode, file list and the open file's patch. `source` is `{key, query}`. */
function Repo({ source, round }) {
  const [tree, setTree] = useState(null)
  const [mode, setMode] = useState('uncommitted')
  const [picked, setPicked] = useState({}) // the refs the owner chose: {base, head, commit}
  const [open, setOpen] = useState(null) // {list, area, path}: the open file belongs to one list
  const at = { ...source.query, ...(tree ? { tree } : {}) }
  const repo = useDiff(url('repo', at), round)

  let refs = null // the query part of the compare, once the overview says what can be picked
  let overview = null
  if (repo.status === 'ready') {
    overview = repo.body
    const { branches, commits, base, worktrees } = overview
    const full = (name) => (branches.some((b) => b.kind === 'branch' && b.name === name) ? `refs/heads/${name}` : name)
    const here = worktrees.find((w) => w.id === overview.tree)
    const fallback = base ? full(base) : ''
    const values = {
      base: picked.base ?? fallback,
      head: picked.head ?? (here?.branch ? full(here.branch) : (commits[0]?.id ?? '')),
      commit: picked.commit ?? commits[0]?.id ?? '',
    }
    if (mode === 'uncommitted') refs = {}
    if (mode === 'branch') refs = values.base ? { base: values.base } : {}
    if (mode === 'commit' && values.commit) refs = { head: values.commit }
    if (mode === 'range' && values.head) refs = { base: values.base || values.head, head: values.head }
    overview = { ...overview, values: { ...values, base: values.base || values.head } }
  }
  const query = refs === null ? null : { ...at, compare: mode, ...refs }
  const listPath = query === null ? null : url('files', query)
  const list = useDiff(listPath, round)
  const files = list.status === 'ready' ? list.body.files : []
  const file = open?.list === listPath ? files.find((f) => f.area === open.area && f.path === open.path) : undefined
  const one = useDiff(file ? url('file', { ...query, area: file.area, path: file.path }) : null, round)

  if (repo.status === 'error') {
    return (
      <p role="alert" className="error">
        {repo.message}
      </p>
    )
  }
  if (!overview) return <Loading label="Loading the repository" />
  const { worktrees, branches, commits, values } = overview
  const pick = (key) => (value) => setPicked((p) => ({ ...p, [key]: value }))
  return (
    <>
      <div className="picks">
        {worktrees.length > 1 && (
          <label className="field">
            Work tree
            <select
              value={tree ?? overview.tree ?? ''}
              onChange={(e) => {
                setTree(e.target.value)
                setPicked({})
              }}
            >
              {worktrees.map((w) => (
                <option key={w.id} value={w.id}>
                  {`${w.folder} · ${w.branch ?? 'detached'}`}
                </option>
              ))}
            </select>
          </label>
        )}
        <div className="periods" role="group" aria-label="Compare">
          {MODES.map(([id, label]) => (
            <button key={id} type="button" aria-pressed={mode === id} onClick={() => setMode(id)}>
              {label}
            </button>
          ))}
        </div>
      </div>
      {mode === 'branch' && (
        <div className="picks">
          <RefSelect label="Base" value={values.base} branches={branches} commits={commits} onChange={pick('base')} />
          <p className="muted">Committed changes only, from where this branch left the base. Uncommitted work is under Uncommitted.</p>
        </div>
      )}
      {mode === 'commit' &&
        (commits.length === 0 ? (
          <p className="muted">This repository has no commit yet.</p>
        ) : (
          <div className="picks">
            <label className="field">
              Commit
              <select value={values.commit} onChange={(e) => pick('commit')(e.target.value)}>
                {commits.map((c) => (
                  <option key={c.id} value={c.id}>
                    {`${commitLabel(c)} — ${c.author}`}
                  </option>
                ))}
              </select>
            </label>
          </div>
        ))}
      {mode === 'range' && (
        <div className="picks">
          <RefSelect label="From" value={values.base} branches={branches} commits={commits} onChange={pick('base')} />
          <RefSelect label="To" value={values.head} branches={branches} commits={commits} onChange={pick('head')} />
        </div>
      )}
      {list.status === 'loading' && <Loading label="Loading the files" />}
      {list.status === 'error' && (
        <p role="alert" className="error">
          {list.message}
        </p>
      )}
      {list.status === 'ready' && (
        <>
          {list.body.truncated && <p className="muted">{`Showing ${fmt(files.length)} of ${fmt(list.body.total)} files.`}</p>}
          {list.body.filters_off && <p className="muted">This repository's git filters are off here, so a file kept through a filter can show as changed.</p>}
          {files.length === 0 ? (
            <p className="muted">No changes.</p>
          ) : (
            <div className="diff-body">
              <div className="diff-files">
                {AREAS.map(([area, title]) => {
                  const found = files.filter((f) => f.area === area)
                  return found.length === 0 ? null : (
                    <section key={area}>
                      <h3>
                        {title} <span className="count">{fmt(found.length)}</span>
                      </h3>
                      <ul>
                        {found.map((f) => (
                          <li key={f.path}>
                            <FileButton file={f} pressed={f === file} onOpen={() => setOpen({ list: listPath, area, path: f.path })} />
                          </li>
                        ))}
                      </ul>
                    </section>
                  )
                })}
              </div>
              <div className="diff-view">{file ? <Patch file={file} state={one} /> : <p className="muted">Pick a file to read its changes.</p>}</div>
            </div>
          )}
        </>
      )}
    </>
  )
}

/**
 * `// work` › Changes (T137, FR-141): the git changes of one repository, view only. The content is shown, never kept: it
 * lives in this view's state and goes when the view does. `changesTarget` (`{session, title}`): Live asked for a session's folder.
 */
export default function ChangesView({ scope = NO_SCOPE, changesTarget: target = null }) {
  const workspaces = useLoad(() => getJson('/api/workspaces'))
  const [chosen, setChosen] = useState(target ? SESSION : null)
  const [round, setRound] = useState(0) // Refresh reads everything again
  const repos = workspaces.status === 'ready' ? repoChoices(workspaces.data.workspaces, scope) : []
  const session = target ? { key: SESSION, query: { session: target.session }, label: `Session: ${target.title}` } : null
  const choices = session ? [session, ...repos] : repos
  const source = choices.find((c) => c.key === chosen) ?? choices[0]
  return (
    <main className="changes">
      <h1>Changes</h1>
      {workspaces.status === 'error' && !session && (
        <p role="alert" className="error">
          {workspaces.message}
        </p>
      )}
      {workspaces.status === 'loading' && !session && <Loading label="Loading the repositories" />}
      {workspaces.status === 'ready' && !source && <p className="muted">Add a repository to a workspace on the Workspaces page to read its changes here.</p>}
      {source && (
        <>
          <div className="picks">
            <label className="field">
              Repository
              <select value={source.key} onChange={(e) => setChosen(e.target.value)}>
                {choices.map((c) => (
                  <option key={c.key} value={c.key}>
                    {c.label}
                  </option>
                ))}
              </select>
            </label>
            <button type="button" className="diff-refresh" onClick={() => setRound((n) => n + 1)}>
              Refresh
            </button>
          </div>
          <Repo key={source.key} source={source} round={round} />
        </>
      )}
    </main>
  )
}
