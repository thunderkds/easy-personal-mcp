import { useState } from 'react'
import { getJson } from './api.js'
import { ago, fmt } from './format.js'
import { NO_SCOPE, scopedPath } from './Scope.jsx'
import { useLoad } from './useLoad.js'

const load = (scope) => getJson(scopedPath('/api/board', scope))

// The kit's own status names, in board order (FR-038).
const COLUMNS = ['Todo', 'In Progress', 'Ready for Review', 'Done', 'Closed (will not do)', 'Blocked']
const COLLAPSED = 'Done'
const PREVIEW = 5
const TONES = 4 // project dot colours, see .project-dot in app.css

const plural = (n, word) => `${fmt(n)} ${word}${n === 1 ? '' : 's'}`
const newest = (times) => times.reduce((a, b) => (a > b ? a : b))

/** Every board task with the project it belongs to; the key is project + id, so the same id in two projects stays two cards. */
function flatten(projects) {
  return projects.flatMap((p, tone) => p.tasks.map((t) => ({ ...t, project: p.project, board: p.board, tone: tone % TONES })))
}

/** "3 projects · 2 synced (last pull 2 h ago) · 1 no data" */
function summarize(projects, now) {
  const synced = projects.filter((p) => p.board)
  const pulls = synced.map((p) => p.last_pull).filter(Boolean)
  const pull = pulls.length > 0 ? ` (last pull ${ago(newest(pulls), now)})` : ''
  const noData = projects.filter((p) => p.status === 'no data').length
  return `${plural(projects.length, 'project')} · ${fmt(synced.length)} synced${pull} · ${fmt(noData)} no data`
}

function Card({ task }) {
  const id = `board-${task.project}-${task.id}`
  return (
    <li>
      <article className="board-card" aria-labelledby={id}>
        <h3 id={id}>{task.id}</h3>
        <p className="board-title">{task.title}</p>
        <p className="board-meta">
          <span className="project-dot" data-tone={task.tone} aria-hidden="true" />
          <span className="board-project">{task.project}</span>
          {task.board && <span className="chip board-kind">{task.board}</span>}
        </p>
      </article>
    </li>
  )
}

function Column({ status, tasks }) {
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
            {title}
          </button>
        ) : (
          title
        )}
      </h2>
      <ul className="board-cards">
        {shown.map((t) => (
          <Card key={`${t.project}/${t.id}`} task={t} />
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

function Projects({ projects, now }) {
  return (
    <ul className="board-projects" aria-label="Projects">
      {projects.map((p, tone) => (
        <li key={p.project + tone}>
          <span className="project-dot" data-tone={tone % TONES} aria-hidden="true" />
          <span className="board-project">{p.project}</span>
          {p.board && <span className="chip board-kind">{p.board}</span>}
          <span className="muted">{p.status === 'no data' ? 'no data' : p.board ? `pulled ${ago(p.last_pull, now)}` : 'not synced'}</span>
        </li>
      ))}
    </ul>
  )
}

function Board({ projects }) {
  const [project, setProject] = useState('')
  const now = Date.now()
  const tasks = flatten(projects).filter((t) => !project || t.project === project)
  const abstained = projects.reduce((n, p) => n + p.abstained, 0)
  return (
    <>
      <p className="muted" data-testid="board-summary">
        {summarize(projects, now)}
      </p>
      <Projects projects={projects} now={now} />
      <label className="board-filter">
        project
        <select value={project} onChange={(e) => setProject(e.target.value)}>
          <option value="">All projects</option>
          {projects.map((p, i) => (
            <option key={p.project + i} value={p.project}>
              {p.project}
            </option>
          ))}
        </select>
      </label>
      {abstained > 0 && <p className="muted">{`${plural(abstained, 'row')} could not be read (unknown section or no task id).`}</p>}
      <div className="board-scroll">
        <div className="board-columns">
          {COLUMNS.map((status) => (
            <Column key={status} status={status} tasks={tasks.filter((t) => t.status === status)} />
          ))}
        </div>
      </div>
    </>
  )
}

export default function BoardView({ scope = NO_SCOPE }) {
  const state = useLoad(() => load(scope), [scope.workspace, scope.account])
  return (
    <main className="board">
      <h1>Board</h1>
      {state.status === 'loading' && <p role="status">Loading board</p>}
      {state.status === 'error' && (
        <p role="alert" className="error">
          Could not load board: {state.message}
        </p>
      )}
      {state.status === 'ready' &&
        (state.data.projects.length === 0 ? <p className="muted">No projects with usage on this machine.</p> : <Board projects={state.data.projects} />)}
    </main>
  )
}
