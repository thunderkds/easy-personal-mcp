import { useState } from 'react'
import { getJson } from './api.js'
import { fmt, localTime, percent } from './format.js'
import { useLoad } from './useLoad.js'

const load = () => getJson('/api/kit')
const COLUMNS = [
  ['kernel', 'Kernel'],
  ['verifier', 'Verifier'],
  ['context', 'Context'],
  ['git', 'Git'],
]
const plural = (n, word) => `${fmt(n)} ${word}${n === 1 ? '' : 's'}`

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
  if (!part || part.status === 'no data') return 'no data'
  if (part.status === 'abstained') return 'abstained'
  if (name === 'kernel') return <Kernel part={part} />
  if (name === 'context') return <Context part={part} />
  if (name === 'git') return plural(part.commits, 'commit')
  return <Generic part={part} />
}

function Projects({ projects }) {
  const [descending, setDescending] = useState(false)
  if (projects.length === 0) return <p className="muted">No projects with usage on this machine.</p>
  const known = new Set(COLUMNS.map(([k]) => k))
  const extra = [...new Set(projects.flatMap((p) => Object.keys(p.parts)))].filter((k) => !known.has(k)).sort()
  const columns = [...COLUMNS, ...extra.map((k) => [k, k])]
  const sorted = [...projects].sort((a, b) => a.project.localeCompare(b.project) * (descending ? -1 : 1))
  return (
    <div className="table-wrap">
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

export default function KitView() {
  const state = useLoad(load)
  return (
    <main>
      <h1>Easy Kit</h1>
      {state.status === 'loading' && <p role="status">Loading kit</p>}
      {state.status === 'error' && (
        <p role="alert" className="error">
          Could not load kit: {state.message}
        </p>
      )}
      {state.status === 'ready' && (
        <>
          <section aria-labelledby="h-kit">
            <h2 id="h-kit">Kit by project</h2>
            <Projects projects={state.data.projects} />
            <p className="muted">"no data" means the kit part left nothing to read for that project. It is not zero.</p>
          </section>
          <Ui ui={state.data.ui} />
        </>
      )}
    </main>
  )
}
