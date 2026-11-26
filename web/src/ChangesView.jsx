import { useEffect, useRef, useState } from 'react'
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
const NO_BASE = { value: '', label: 'Pick a base' }

/** The workspace repos the owner can pick, as `{key, query, label}`; `repo_no` is the repo's place in the workspace's list. */
function repoChoices(workspaces, scope) {
  const repos = workspaces
    .filter((w) => !w.is_orphan && (!scope.workspace || w.name === scope.workspace))
    .flatMap((w) => w.repos.map((path, n) => ({ workspace: w.name, n, path, short: `${folderName(path)} (${w.name})` })))
  return repos.map(({ workspace, n, path, short }) => ({
    key: `${workspace}/${n}`,
    query: { workspace, repo_no: String(n) },
    // two folders of one name are told apart by their whole paths
    label: repos.filter((r) => r.short === short).length > 1 ? `${path} (${workspace})` : short,
  }))
}

const url = (route, query) => `/api/diff/${route}?${new URLSearchParams(query)}`

/**
 * The answer of `path` (null: ask nothing), read again when it or `round` changes. While the same path is read again the
 * last answer stays on the page, marked `busy`; another path's answer is never shown. `at`: when the answer came.
 */
function useDiff(path, round) {
  const [answer, setAnswer] = useState(null)
  useEffect(() => {
    if (path === null) return undefined
    let current = true
    const keep = (answer) => current && setAnswer({ path, round, at: Date.now(), ...answer })
    getDiff(path).then(
      (body) => keep({ status: 'ready', body }),
      (error) => keep({ status: 'error', message: error.message }),
    )
    return () => {
      current = false
    }
  }, [path, round])
  if (path === null) return { status: 'idle' }
  return answer?.path === path ? { ...answer, busy: answer.round !== round } : { status: 'loading' }
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
      {file.openable ? (
        <span className="diff-count">{file.binary ? 'binary' : file.added === null ? '' : `+${fmt(file.added)} −${fmt(file.removed)}`}</span>
      ) : (
        <span className="diff-note">cannot open: the name is not text</span>
      )}
    </button>
  )
}

/**
 * A select over `choice` (`{value, label}`). A value that left the lists (a picked commit amended away, a default base
 * the repository no longer has) is offered as itself, under its label: a select never shows another ref than the one read.
 */
function Picker({ label, choice, listed, onPick, children }) {
  // The mark comes first: a closed select cuts a long label at its end.
  const pick = (e) => onPick({ value: e.target.value, label: `(no longer listed) ${e.target.selectedOptions[0].text}` })
  return (
    <label className="field">
      {label}
      <select value={choice.value} onChange={pick}>
        {!listed && <option value={choice.value}>{choice.label}</option>}
        {children}
      </select>
    </label>
  )
}

function RefSelect({ label, choice, branches, commits, onPick }) {
  const listed = commits.some((c) => c.id === choice.value) || REF_GROUPS.some(([kind, , prefix]) => branches.some((b) => b.kind === kind && prefix + b.name === choice.value))
  return (
    <Picker label={label} choice={choice} listed={listed} onPick={onPick}>
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
    </Picker>
  )
}

const commitLabel = (c) => `${c.id.slice(0, 7)} ${c.subject}`
const authored = (c) => `${commitLabel(c)} — ${c.author}`

/**
 * What the refs of a compare are until the owner picks others, each `{value, label}` or null. Refs go by their full name,
 * so a branch or tag of the same short name cannot take their place.
 */
function defaultRefs({ branches, commits, base: name, base_shown: shown, worktrees, tree }) {
  // The server's default base is a local `main` or `master`, else the remote's default branch.
  const prefix = name === 'main' || name === 'master' ? 'refs/heads/' : 'refs/remotes/'
  const base = name ? { value: name.startsWith('refs/') ? name : prefix + name, label: shown ?? name } : null
  const commit = commits[0] ? { value: commits[0].id, label: authored(commits[0]) } : null
  const here = worktrees.find((w) => w.id === tree)
  let head = commit && { value: commit.value, label: commitLabel(commits[0]) }
  if (commits.length === 0) {
    // This branch has no commit to name: start from one that has.
    const first = branches[0]
    head = base ?? (first ? { value: REF_GROUPS.find(([kind]) => kind === first.kind)[2] + first.name, label: first.shown } : null)
  } else if (here?.branch) {
    const mine = branches.find((b) => b.kind === 'branch' && b.shown === here.branch) // a work tree names its branch as it is shown
    head = { value: `refs/heads/${mine?.name ?? here.branch}`, label: here.branch }
  }
  return { base, head, commit }
}

/** One repository: its work tree, compare mode, file list and the open file's patch. `source` is `{key, query}`. */
function Repo({ source, round }) {
  const [tree, setTree] = useState(null)
  const [mode, setMode] = useState('uncommitted')
  const [picked, setPicked] = useState({}) // the refs the owner chose: {base, head, commit}, each {value, label}
  const [open, setOpen] = useState(null) // {list, area, path}: the open file belongs to one list
  const moved = useRef(false) // the owner picked a work tree: its select takes the focus back after the read
  const at = { ...source.query, ...(tree ? { tree } : {}) }
  const repo = useDiff(url('repo', at), round)

  let refs = null // the query part of the compare, once the overview says what can be picked
  let overview = null
  if (repo.status === 'ready') {
    overview = repo.body
    const { branches, commits } = overview
    const defaults = defaultRefs(overview)
    const head = picked.head ?? defaults.head
    const base = picked.base ?? defaults.base
    const from = base ?? defaults.head // two refs with no base start as one ref against itself; picking To leaves it
    const commit = picked.commit ?? defaults.commit
    // With no commit here there is nothing to compare but the uncommitted files; two refs need any ref at all.
    const unborn = commits.length === 0 && (mode !== 'range' || branches.length === 0)
    if (mode === 'uncommitted') refs = {}
    if (mode === 'branch' && !unborn) refs = base ? { base: base.value } : {}
    if (mode === 'commit' && !unborn && commit) refs = { head: commit.value }
    if (mode === 'range' && !unborn && head) refs = { base: from.value, head: head.value }
    overview = { ...overview, unborn: mode !== 'uncommitted' && unborn, chosen: { base, from, head, commit } }
  }
  // The list and the patch name the work tree the overview answered for: a session that moves cannot answer from another one.
  const query = refs === null ? null : { ...at, ...(overview.tree ? { tree: overview.tree } : {}), compare: mode, ...refs }
  const listPath = query === null ? null : url('files', query)
  const list = useDiff(listPath, round)
  const files = list.status === 'ready' ? list.body.files : []
  const file = open?.list === listPath ? files.find((f) => f.area === open.area && f.path === open.path) : undefined
  const one = useDiff(file ? url('file', { ...query, area: file.area, path: file.path }) : null, round)

  const toTree = (id) => {
    setTree(id)
    setPicked({})
  }
  // A read that changes nothing is still seen: the line says that it runs, then the time of the oldest answer shown.
  const answers = [repo, list, one]
  const readAt = Math.min(...answers.map((a) => a.at ?? Infinity))
  const read = (
    <p role="status" className="muted">
      {answers.some((a) => a.busy) ? 'Reading again…' : `Read at ${new Date(readAt).toLocaleTimeString()}`}
    </p>
  )
  if (repo.status === 'error') {
    return (
      <>
        <p role="alert" className="error">
          {repo.message}
        </p>
        {tree !== null && (
          <button type="button" onClick={() => toTree(null)}>
            Back to the default work tree
          </button>
        )}
        {read}
      </>
    )
  }
  if (!overview) return <Loading label="Loading the repository" />
  const { worktrees, branches, commits, chosen, unborn } = overview
  const pick = (key) => (choice) => setPicked((p) => ({ ...p, [key]: choice }))
  const shownTree = tree ?? overview.tree ?? ''
  // Runs when the select appears: the focus returns only if the read left it nowhere.
  const refocus = (select) => {
    if (!select || !moved.current) return
    moved.current = false
    if (document.activeElement === document.body) select.focus()
  }
  return (
    <>
      <div className="picks">
        {worktrees.length > 1 && (
          <label className="field">
            Work tree
            <select
              ref={refocus}
              value={shownTree}
              onChange={(e) => {
                moved.current = true
                toTree(e.target.value)
              }}
            >
              {!worktrees.some((w) => w.id === shownTree) && <option value={shownTree}>This folder (not listed)</option>}
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
      {read}
      {overview.truncated && <p className="muted">Some branches, tags or work trees are not listed: this repository has too many.</p>}
      {unborn && <p className="muted">{`This ${branches.length === 0 ? 'repository' : 'branch'} has no commit yet.`}</p>}
      {mode === 'branch' && !unborn && (
        <div className="picks">
          <RefSelect label="Base" choice={chosen.base ?? NO_BASE} branches={branches} commits={commits} onPick={pick('base')} />
          <p className="muted">Committed changes only, from where this branch left the base. Uncommitted work is under Uncommitted.</p>
        </div>
      )}
      {mode === 'commit' && !unborn && chosen.commit && (
        <div className="picks">
          <Picker label="Commit" choice={chosen.commit} listed={commits.some((c) => c.id === chosen.commit.value)} onPick={pick('commit')}>
            {commits.map((c) => (
              <option key={c.id} value={c.id}>
                {authored(c)}
              </option>
            ))}
          </Picker>
        </div>
      )}
      {mode === 'range' && !unborn && chosen.head && (
        <div className="picks">
          <RefSelect label="From" choice={chosen.from} branches={branches} commits={commits} onPick={pick('base')} />
          <RefSelect label="To" choice={chosen.head} branches={branches} commits={commits} onPick={pick('head')} />
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
