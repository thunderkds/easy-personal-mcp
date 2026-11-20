import { useRef, useState } from 'react'
import { getAgentRun, getJson, startAppSession } from './api.js'
import { ago, fmt } from './format.js'
import { MAX_BYTES, byteLength, repoChoices } from './NewSession.jsx'
import { OFF } from './NewSessionSheet.jsx'
import { useNotify } from './Toast.jsx'
import { NO_SCOPE, scopedPath } from './Scope.jsx'
import { useLoad } from './useLoad.js'
import Loading from './Loading.jsx'

const load = (scope) => getJson(scopedPath('/api/board', scope))
// What the Run button needs, read once beside the board. Any failure means no Run button: the board itself still shows.
const loadRun = () => Promise.all([getAgentRun(), getJson('/api/workspaces')]).then(([agentRun, list]) => (agentRun ? { agentRun, workspaces: list.workspaces } : null), () => null)

// The kit's own status names, in board order (FR-038).
const COLUMNS = ['Todo', 'In Progress', 'Ready for Review', 'Done', 'Closed (will not do)', 'Blocked']
const COLLAPSED = 'Done'
const PREVIEW = 5
const TONES = 4 // project dot colours, see .project-dot in app.css

const plural = (n, word) => `${fmt(n)} ${word}${n === 1 ? '' : 's'}`
const newest = (times) => times.reduce((a, b) => (a > b ? a : b))
const ALL = 'all'
const CLOSED = ['Done', 'Closed (will not do)']
const RUNNABLE = ['Todo', 'In Progress', 'Ready for Review', 'Blocked']
export const NO_REPO = 'Add this repo to a workspace to run it'
export const AMBIGUOUS = 'Two workspace repos share this name'
const FAILED = 'Could not start the session. Try again.'

/** A title as plain text: markdown emphasis and code marks removed, never parsed as markup. */
const plain = (title) =>
  title
    .replace(/\*\*(.+?)\*\*/g, '$1')
    .replace(/__(.+?)__/g, '$1')
    .replace(/`([^`]+)`/g, '$1')
    .replace(/(^|[^\w*])\*(\S(?:[^*]*\S)?)\*(?![\w*])/g, '$1$2')

const baseName = (path) => path.split(/[\\/]/).filter(Boolean).at(-1)

/** The one workspace repo whose folder name is the board project's: `{workspace, repo}`, else `{reason}` (none, or more than one). */
export function repoFor(project, workspaces) {
  const found = repoChoices(workspaces).filter((r) => baseName(r.repo) === project)
  if (found.length === 0) return { reason: NO_REPO }
  if (found.length > 1) return { reason: AMBIGUOUS }
  return found[0]
}

/** The fixed first prompt; a title too long for the 100 000-byte limit is cut by whole characters. */
export function promptFor(id, title) {
  const head = `Work on task ${id}: `
  const tail = `. Follow this project's workflow; its guide is tasks/TASK_GUIDE_${id}.md if that file exists.`
  const room = MAX_BYTES - byteLength(head) - byteLength(tail)
  let chars = Array.from(title)
  while (byteLength(chars.join('')) > room) chars = chars.slice(0, Math.floor(chars.length * 0.9))
  return head + chars.join('') + tail
}

/** Every board task with the project it belongs to; the key is project position + id, so the same id in two projects stays two cards. */
function flatten(projects) {
  return projects.flatMap((p, pi) => p.tasks.map((t) => ({ ...t, project: p.project, board: p.board, pi, tone: pi % TONES })))
}

const openCount = (p) => p.tasks.filter((t) => !CLOSED.includes(t.status)).length

/** Projects that have a board file, projects that name a board kind first, then the newest pull first; the sort is stable. */
function listBoards(projects) {
  const rank = (p) => (p.board ? 0 : 1)
  return projects
    .filter((p) => p.status !== 'no data')
    .sort((a, b) => rank(a) - rank(b) || (b.last_pull || '').localeCompare(a.last_pull || ''))
}

/** The newest board pull; else the first project with tasks; else the first. */
function defaultChoice(listed) {
  const pulled = listed.findIndex((p) => p.last_pull)
  const tasked = listed.findIndex((p) => p.tasks.length > 0)
  const i = pulled >= 0 ? pulled : tasked >= 0 ? tasked : 0
  return String(i)
}

/** Picker labels stay distinct: a folder name that repeats gets its board kind, else its position. */
function labels(listed) {
  return listed.map((p, i) => {
    const same = listed.filter((q) => q.project === p.project).length > 1
    const name = same ? `${p.project} · ${p.board || `#${i + 1}`}` : p.project
    return `${name} (${fmt(openCount(p))} open)`
  })
}

/** "2 projects with a board · 1 synced (last pull 2 h ago)" */
function summarize(listed, now) {
  const synced = listed.filter((p) => p.board)
  const pulls = synced.map((p) => p.last_pull).filter(Boolean)
  const pull = pulls.length > 0 ? ` (last pull ${ago(newest(pulls), now)})` : ''
  return `${plural(listed.length, 'project')} with a board · ${fmt(synced.length)} synced${pull}`
}

function RunButton({ task, title, run, onStarted }) {
  const notify = useNotify()
  const [busy, setBusy] = useState(false)
  const open = useRef(false) // one request at a time: a second click while one is out is dropped
  const target = repoFor(task.project, run.workspaces)
  const off = !run.agentRun.enabled
  const start = async () => {
    if (open.current || off || target.reason) return
    open.current = true
    setBusy(true)
    try {
      const answer = await startAppSession({
        repo: target.repo,
        workspace: target.workspace,
        account: null,
        model: 'default',
        effort: 'default',
        mode: 'default', // a one-click start never skips the permission prompts
        prompt: promptFor(task.id, title),
        worktree: false,
      })
      onStarted?.(answer.id)
    } catch (e) {
      notify('error', e.status && e.message ? e.message : FAILED)
    } finally {
      open.current = false
      setBusy(false)
    }
  }
  return (
    <div className="board-run">
      <button type="button" className="primary" aria-label={`Run ${task.id}`} aria-describedby={off ? 'board-run-off' : undefined} disabled={busy || off || Boolean(target.reason)} onClick={start}>
        {busy ? 'Starting…' : 'Run'}
      </button>
      {target.reason && <span className="muted board-reason">{target.reason}</span>}
    </div>
  )
}

function Card({ task, showProject, run, onStarted }) {
  const id = `board-${task.pi}-${task.id}`
  const title = plain(task.title)
  return (
    <li>
      <article className="board-card" aria-labelledby={`${id} ${id}-title`}>
        <h3 id={id}>{task.id}</h3>
        <p className="board-title" id={`${id}-title`} title={title}>
          {title}
        </p>
        {showProject && (
          <p className="board-meta">
            <span className="project-dot" data-tone={task.tone} aria-hidden="true" />
            <span className="board-project">{task.project}</span>
            {task.board && <span className="chip board-kind">{task.board}</span>}
          </p>
        )}
        {run && RUNNABLE.includes(task.status) && <RunButton task={task} title={title} run={run} onStarted={onStarted} />}
      </article>
    </li>
  )
}

function Column({ status, tasks, showProject, run, onStarted }) {
  const [open, setOpen] = useState(false)
  const [all, setAll] = useState(false)
  const collapsible = status === COLLAPSED
  const shown = !collapsible ? tasks : !open ? [] : all ? tasks : tasks.slice(-PREVIEW)
  const title = `${status} (${fmt(tasks.length)})`
  const headingId = `board-col-${status.replace(/\W+/g, '-')}`
  return (
    <section className="board-column" aria-labelledby={headingId}>
      <h2 id={headingId}>
        {collapsible ? (
          <button type="button" className="board-toggle" aria-expanded={open} onClick={() => setOpen(!open)}>
            <span className="board-chevron" aria-hidden="true" />
            {title}
          </button>
        ) : (
          title
        )}
      </h2>
      <ul className="board-cards">
        {shown.map((t) => (
          <Card key={`${t.pi}/${t.id}`} task={t} showProject={showProject} run={run} onStarted={onStarted} />
        ))}
      </ul>
      {collapsible && open && !all && tasks.length > PREVIEW && (
        <button type="button" className="board-all" onClick={() => setAll(true)}>
          {`Show all ${fmt(tasks.length)}`}
        </button>
      )}
    </section>
  )
}

function Unlisted({ projects }) {
  const [shown, setShown] = useState(false)
  if (projects.length === 0) return null
  return (
    <div className="board-unlisted">
      <button type="button" className="board-toggle" aria-expanded={shown} onClick={() => setShown(!shown)}>
        {`${fmt(projects.length)} without a board`}
      </button>
      {shown && (
        <ul className="board-projects" aria-label="Projects without a board">
          {projects.map((p, i) => (
            <li key={p.project + i}>
              <span className="board-project">{p.project}</span>
            </li>
          ))}
        </ul>
      )}
    </div>
  )
}

function Board({ projects, gone, run, onStarted, onOpenSettings }) {
  const [picked, setPicked] = useState(null)
  const now = Date.now()
  const listed = listBoards([...projects])
  const unlisted = projects.filter((p) => p.status === 'no data')
  const choice = picked !== null && (picked === ALL || listed[Number(picked)]) ? picked : defaultChoice(listed)
  const everyone = choice === ALL
  const current = everyone ? null : listed[Number(choice)]
  const tasks = flatten(listed).filter((t) => everyone || t.pi === Number(choice))
  const abstained = (everyone ? listed : current ? [current] : []).reduce((n, p) => n + p.abstained, 0)
  const names = labels(listed)
  return (
    <>
      {listed.length === 0 ? (
        <p className="muted">No project on this machine has a board file.</p>
      ) : (
        <>
          <p className="muted" data-testid="board-summary">
            {summarize(listed, now)}
          </p>
          <label className="board-filter">
            project
            <select value={choice} onChange={(e) => setPicked(e.target.value)}>
              {listed.map((p, i) => (
                <option key={p.project + i} value={String(i)}>
                  {names[i]}
                </option>
              ))}
              <option value={ALL}>All projects</option>
            </select>
          </label>
          {current && (
            <p className="board-current">
              {current.board && <span className="chip board-kind">{current.board}</span>}
              <span className="muted">{current.board ? `pulled ${ago(current.last_pull, now)}` : 'not synced'}</span>
            </p>
          )}
        </>
      )}
      <Unlisted projects={unlisted} />
      {gone > 0 && <p className="muted">{`${plural(gone, 'gone folder')} hidden`}</p>}
      {abstained > 0 && <p className="muted">{`${plural(abstained, 'row')} could not be read (unknown section or no task id).`}</p>}
      {listed.length > 0 && run && !run.agentRun.enabled && (
        <p className="muted board-off" id="board-run-off">
          {OFF}{' '}
          {onOpenSettings && (
            <button type="button" className="link-button" onClick={onOpenSettings}>
              Open Settings
            </button>
          )}
        </p>
      )}
      {listed.length > 0 && (
        <div className="board-scroll">
          <div className="board-columns">
            {COLUMNS.map((status) => (
              <Column key={status} status={status} tasks={tasks.filter((t) => t.status === status)} showProject={everyone} run={run} onStarted={onStarted} />
            ))}
          </div>
        </div>
      )}
    </>
  )
}

/** `onOpenAppSession(id)` opens Live on a session just started; `onOpenSettings()` is the link of the "app sessions are off" line. */
export default function BoardView({ scope = NO_SCOPE, onOpenAppSession, onOpenSettings }) {
  const runState = useLoad(loadRun, [])
  const run = runState.status === 'ready' ? runState.data : null
  const state = useLoad(() => load(scope), [scope.workspace, scope.account])
  return (
    <main className="board">
      <h1>Board</h1>
      {state.status === 'loading' && <Loading label="Loading board" />}
      {state.status === 'error' && (
        <p role="alert" className="error">
          Could not load board: {state.message}
        </p>
      )}
      {state.status === 'ready' &&
        (state.data.projects.length === 0 ? <p className="muted">No projects with usage on this machine.</p> : <Board key={`${scope.workspace}/${scope.account}`} projects={state.data.projects} gone={Number(state.data.gone) || 0} run={run} onStarted={onOpenAppSession} onOpenSettings={onOpenSettings} />)}
    </main>
  )
}
