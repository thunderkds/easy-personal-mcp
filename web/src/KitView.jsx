import { useState } from 'react'
import { getJson } from './api.js'
import FeedCard from './FeedCard.jsx'
import { fmt, localTime, percent } from './format.js'
import { NO_SCOPE, scopedPath } from './Scope.jsx'
import { useLoad } from './useLoad.js'
import Loading from './Loading.jsx'
import { Sparkline } from './Widgets.jsx'

const load = (scope) => getJson(scopedPath('/api/kit', scope))
const COLUMNS = [
  ['kernel', 'Kernel'],
  ['verifier', 'Verifier'],
  ['context', 'Context'],
  ['git', 'Git'],
]
const plural = (n, word) => `${fmt(n)} ${word}${n === 1 ? '' : 's'}`

const ERROR_RATE_LIMIT = 0.1
const DAYS = 30
const DAY_MS = 86400000
// Plain names for the card; `meaning` says what the part is in a few words.
const PART_NAMES = {
  kernel: { name: 'Agent kit', meaning: 'tool calls the kit traced', short: 'Agent kit' },
  context: { name: 'Code index', meaning: 'easy-context', short: 'Code index' },
  verifier: { name: 'Verifier score', meaning: null, short: 'Verifier' },
  git: { name: 'Git history', meaning: 'local commits', short: 'Git history' },
}
const PART_ORDER = ['kernel', 'context', 'verifier', 'git']

/** A part the kit does not use here (FR-080): shown nowhere and never counted. */
const notInUse = (part) => Boolean(part) && part.status === 'not in use'
const hasData = (part) => Boolean(part) && part.status !== 'no data' && part.status !== 'abstained' && !notInUse(part)
const pad = (n) => String(n).padStart(2, '0')
/** 'YYYY-MM-DD' of a Date in local time. */
const localDay = (date) => `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`
/** Whole local days from a 'YYYY-MM-DD' day to `now` (0 = today). */
const daysBefore = (day, now) => {
  const [y, m, d] = day.split('-').map(Number)
  return Math.round((Date.UTC(now.getFullYear(), now.getMonth(), now.getDate()) - Date.UTC(y, m - 1, d)) / DAY_MS)
}
const ago = (days) => (days <= 0 ? 'today' : days === 1 ? 'yesterday' : `${days} days ago`)

/** Commits today and in the last 30 local days (today included), plus one value per day, oldest first. */
export function commitStats(commitsByDay, now) {
  const series = Array.from({ length: DAYS }, (_, i) => {
    const day = new Date(now.getFullYear(), now.getMonth(), now.getDate() - (DAYS - 1 - i))
    return commitsByDay[localDay(day)] ?? 0
  })
  return { today: series[DAYS - 1], last30: series.reduce((a, b) => a + b, 0), peak: Math.max(...series), series }
}

/** One health verdict per project: attention (with the reasons), healthy, or none. */
export function projectHealth(project) {
  const { kernel, context } = project.parts
  const reasons = []
  if (hasData(kernel) && typeof kernel.error_rate === 'number' && kernel.error_rate > ERROR_RATE_LIMIT) {
    reasons.push(`${Math.round(kernel.error_rate * 100)}% of tool calls failed`)
  }
  if (hasData(context) && context.stale === true) reasons.push('code index is out of date')
  if (reasons.length > 0) return { level: 'attention', text: `Needs attention: ${reasons.join('; ')}` }
  if (Object.values(project.parts).some(hasData)) return { level: 'healthy', text: 'Healthy' }
  return { level: 'none', text: 'No kit data' }
}

/** Newest local day with kernel traces or commits, with the kernel time to break ties; null when neither. */
function lastActive(project) {
  const { kernel, git } = project.parts
  const found = []
  if (hasData(kernel) && kernel.last_utc) found.push({ day: localDay(new Date(kernel.last_utc)), at: Date.parse(kernel.last_utc) })
  if (hasData(git)) {
    const days = Object.keys(git.commits_by_day ?? {}).filter((d) => git.commits_by_day[d] > 0)
    if (days.length > 0) found.push({ day: days.sort().at(-1), at: 0 })
  }
  return found.sort((a, b) => b.day.localeCompare(a.day) || b.at - a.at)[0] ?? null
}

/** Cards order: newest activity first, then projects with data but no dates, then projects with no data. */
function byActivity(projects) {
  const info = projects.map((project) => ({ project, active: lastActive(project), health: projectHealth(project) }))
  const rank = (i) => (i.active ? 0 : i.health.level === 'none' ? 2 : 1)
  return info.sort(
    (a, b) =>
      rank(a) - rank(b) ||
      (a.active && b.active ? b.active.day.localeCompare(a.active.day) || b.active.at - a.active.at : 0) ||
      a.project.project.localeCompare(b.project.project),
  )
}

/** "Last active today. 13,976 agent tool calls, none failed. 39 commits today, 342 in the last 30 days." */
function summarize(project, active, stats, now) {
  const { kernel } = project.parts
  const sentences = []
  if (active) sentences.push(`Last active ${ago(daysBefore(active.day, now))}.`)
  if (hasData(kernel)) {
    const failed = typeof kernel.error_rate !== 'number' ? '' : kernel.tool_errors === 0 ? ', none failed' : `, ${fmt(kernel.tool_errors)} failed`
    sentences.push(`${plural(kernel.tool_calls, 'agent tool call')}${failed}.`)
  }
  if (stats) sentences.push(`${plural(stats.today, 'commit')} today, ${fmt(stats.last30)} in the last ${DAYS} days.`)
  return sentences.length > 0 ? sentences.join(' ') : 'No kit data yet.'
}

function Kernel({ part }) {
  const rate = part.error_rate === null ? '' : ` (${percent(part.error_rate)})`
  return (
    <>
      {plural(part.tool_calls, 'tool call')}, {plural(part.tool_errors, 'error')}
      {rate}; {plural(part.kit_tasks, 'kit task')}
      {part.abstained > 0 && `; ${fmt(part.abstained)} abstained`}
    </>
  )
}

function Context({ part }) {
  const state = part.stale === true ? 'stale' : part.stale === false ? 'fresh' : 'unknown'
  return (
    <>
      <span className={state === 'stale' ? 'stale' : undefined}>{state}</span>
      {part.last_index_at && ` · indexed ${localTime(part.last_index_at)}`}
    </>
  )
}

/** Any part we have no special text for (e.g. a later `verifier`): its scalar keys, as "key: value". */
function Generic({ part }) {
  const { status: _status, ...rest } = part
  return Object.entries(rest)
    .filter(([, v]) => v === null || ['string', 'number', 'boolean'].includes(typeof v))
    .map(([k, v]) => `${k}: ${v === null ? 'unknown' : v}`)
    .join('; ')
}

function Part({ name, part }) {
  if (notInUse(part)) return null
  if (!part || part.status === 'no data') return 'no data'
  if (part.status === 'abstained') return 'abstained'
  if (name === 'kernel') return <Kernel part={part} />
  if (name === 'context') return <Context part={part} />
  if (name === 'git') return plural(part.commits, 'commit')
  return <Generic part={part} />
}

const statusClass = { healthy: 'chip-ok', attention: 'chip-attention', none: 'chip-none' }

function PartDetail({ name, part, now }) {
  if (part.status === 'abstained') return 'could not be read'
  if (name === 'kernel') {
    return (
      <>
        {plural(part.kit_tasks, 'kit task')}
        {part.abstained > 0 && `, ${plural(part.abstained, 'record')} could not be read`}
      </>
    )
  }
  if (name === 'context') {
    const state = part.stale === true ? 'stale' : part.stale === false ? 'fresh' : 'freshness unknown'
    const when = part.last_index_at ? `indexed ${ago(daysBefore(localDay(new Date(part.last_index_at)), now))}` : 'index date unknown'
    return (
      <>
        {when}, <span className={part.stale === true ? 'stale' : undefined}>{state}</span>
      </>
    )
  }
  if (name === 'verifier') {
    const overall = part.overall
    if (overall && typeof overall.value === 'number') return `${overall.value}/100`
    if (overall && overall.abstained) return 'could not be scored'
  }
  if (name === 'git') return `${plural(commitStats(part.commits_by_day ?? {}, now).last30, 'commit')} in the last ${DAYS} days`
  return <Generic part={part} />
}

function PartLine({ name, part, now }) {
  const known = PART_NAMES[name]
  const meaning = known?.meaning && ` (${known.meaning})`
  return (
    <li>
      <strong>{known ? known.name : name}</strong>
      {meaning} — <PartDetail name={name} part={part} now={now} />
    </li>
  )
}

function ProjectCard({ project, active, health, now }) {
  const id = `card-${project.project}`
  const { git } = project.parts
  const stats = hasData(git) ? commitStats(git.commits_by_day ?? {}, now) : null
  const keys = [...PART_ORDER, ...Object.keys(project.parts).filter((k) => !PART_ORDER.includes(k)).sort()]
  const shownKeys = keys.filter((k) => !notInUse(project.parts[k]))
  const present = shownKeys.filter((k) => project.parts[k] && project.parts[k].status !== 'no data')
  const missing = shownKeys.filter((k) => !present.includes(k)).map((k) => PART_NAMES[k]?.short ?? k)
  const label = stats && `Commits per day, last ${DAYS} days: ${fmt(stats.last30)} in total, peak ${fmt(stats.peak)} in one day`
  return (
    <li>
      <FeedCard
        id={id}
        className="project-card"
        icon={project.project}
        title={project.project}
        summary={
          <>
            {project.missing_on_disk && <span className="muted">(folder missing) </span>}
            <span className={`chip ${statusClass[health.level]}`}>{health.text}</span>
          </>
        }
        time={active ? ago(daysBefore(active.day, now)) : undefined}
        footer={missing.length > 0 && <span>Not set up here: {missing.join(', ')}</span>}
      >
        <p className="card-summary">{summarize(project, active, stats, now)}</p>
        {stats && <Sparkline values={stats.series} label={label} />}
        {present.length > 0 && (
          <ul className="part-lines">
            {present.map((k) => (
              <PartLine key={k} name={k} part={project.parts[k]} now={now} />
            ))}
          </ul>
        )}
      </FeedCard>
    </li>
  )
}

const RECENT_FALLBACK = 10 // when no project was active in the last 30 days, still show this many

/** Ordered cards split in two: active in the last 30 days (else the 10 newest), and the rest. */
export function splitRecent(ordered, now) {
  const recent = ordered.filter((i) => i.active && daysBefore(i.active.day, now) <= DAYS)
  const head = recent.length > 0 ? recent.length : Math.min(RECENT_FALLBACK, ordered.length)
  return { shown: ordered.slice(0, head), folded: ordered.slice(head) }
}

function Cards({ projects }) {
  const now = new Date()
  const [open, setOpen] = useState(false)
  const { shown, folded } = splitRecent(byActivity(projects), now)
  const card = ({ project, active, health }) => <ProjectCard key={project.project} project={project} active={active} health={health} now={now} />
  return (
    <>
      <ul className="project-cards">
        {shown.map(card)}
        {open && folded.map(card)}
      </ul>
      {folded.length > 0 && (
        <button type="button" className="details-toggle" aria-expanded={open} onClick={() => setOpen(!open)}>
          {open ? 'Show fewer' : `Show ${fmt(folded.length)} more`}
        </button>
      )}
    </>
  )
}

function DetailsTable({ projects }) {
  const [descending, setDescending] = useState(false)
  const known = new Set(COLUMNS.map(([k]) => k))
  const extra = [...new Set(projects.flatMap((p) => Object.keys(p.parts)))].filter((k) => !known.has(k)).sort()
  const columns = [...COLUMNS, ...extra.map((k) => [k, k])].filter(([k]) => !projects.every((p) => notInUse(p.parts[k])))
  const sorted = [...projects].sort((a, b) => a.project.localeCompare(b.project) * (descending ? -1 : 1))
  return (
    <div className="table-wrap table-frame">
      <table>
        <thead>
          <tr>
            <th scope="col" aria-sort={descending ? 'descending' : 'ascending'}>
              <button type="button" className="sort" onClick={() => setDescending(!descending)}>
                Project {descending ? '▼' : '▲'}
              </button>
            </th>
            {columns.map(([key, title]) => (
              <th scope="col" key={key}>
                {title}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {sorted.map((p) => (
            <tr key={p.project}>
              <th scope="row">
                {p.project}
                {p.missing_on_disk && <span className="muted"> (missing on disk)</span>}
              </th>
              {columns.map(([key]) => (
                <td key={key}>
                  <Part name={key} part={p.parts[key]} />
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}

function Projects({ projects }) {
  const [open, setOpen] = useState(false)
  if (projects.length === 0) return <p className="muted">No projects with usage on this machine.</p>
  return (
    <>
      <Cards projects={projects} />
      <button type="button" className="details-toggle" aria-expanded={open} aria-controls="kit-details" onClick={() => setOpen(!open)}>
        {open ? 'Hide details' : 'Show details'}
      </button>
      <div id="kit-details">
        {open && (
          <>
            <DetailsTable projects={projects} />
            <p className="muted">"no data" means the kit part left nothing to read for that project. It is not zero.</p>
          </>
        )}
      </div>
    </>
  )
}

function Ui({ ui }) {
  const { status: _status, first_utc, last_utc, ...counts } = ui
  return (
    <section aria-labelledby="h-ui">
      <h2 id="h-ui">easy-ui checks (overall)</h2>
      {ui.status !== 'ok' ? (
        <p>no data</p>
      ) : (
        <>
          <ul className="entries">
            {Object.entries(counts).map(([k, v]) => (
              <li key={k}>{`${k}: ${fmt(v)}`}</li>
            ))}
          </ul>
          <p className="muted">
            {localTime(first_utc)} to {localTime(last_utc)}. easy-ui checks are not split by project.
          </p>
        </>
      )}
    </section>
  )
}

export default function KitView({ scope = NO_SCOPE }) {
  const state = useLoad(() => load(scope), [scope.workspace, scope.account])
  return (
    <main>
      <h1>Easy Kit</h1>
      {state.status === 'loading' && <Loading label="Loading kit" />}
      {state.status === 'error' && (
        <p role="alert" className="error">
          Could not load kit: {state.message}
        </p>
      )}
      {state.status === 'ready' && (
        <>
          <section aria-labelledby="h-kit">
            <h2 id="h-kit">
              Kit by project <span className="count">{`(${state.data.projects.length})`}</span>
            </h2>
            <p className="muted">
              {state.data.projects.length > 0 && state.data.projects.every((p) => notInUse(p.parts.context))
                ? 'How the agent kit and verifier are doing in each project.'
                : 'How the agent kit, code index and verifier are doing in each project.'}
            </p>
            {state.data.gone > 0 && <p className="muted gone-note">{plural(state.data.gone, 'gone folder')} hidden</p>}
            <Projects projects={state.data.projects} />
          </section>
          <Ui ui={state.data.ui} />
        </>
      )}
    </main>
  )
}
