import { useState } from 'react'
import { getJson } from './api.js'
import { ago, compact, fmt, percent } from './format.js'
import { useLoad } from './useLoad.js'

export const MAX_DRAWN = 300 // more nodes than this are counted, not drawn
const SIZE = 640
const SIZE_WIDE = 800 // room for ring 4 and its labels; used only when a ring 4 node exists, so other graphs keep their canvas
const RADII = [0, 115, 200, 265, 335] // ring 0 = MEMORY.md, 1 = cold files, 2 = records and Claude notes, 3 = unresolved links, 4 = easy-memory cited files
const EASY_RING = 4
const CITED_MIN = 7 // radius of an easy-memory node cited once
const CITED_MAX = 12
const LABEL_LIMIT = 60 // above this many drawn nodes only the inner rings are labelled
const LABEL_CHARS = 14
const EASY_LABEL_CHARS = 10 // ring 4 is crowded: shorter labels, and every second one above its node

const KIND_NAMES = { kit: 'kit memory', claude: 'Claude memory', dangling: 'unresolved link', easy_memory: 'easy-memory file' }
const STALE = {
  fresh: 'unchanged since the newest note',
  stale: 'changed since the newest note',
  gone: 'file gone',
  unchecked: 'not checked',
}
const MARKS = { stale: '!', gone: '×' } // a symbol as well as a colour: state is never colour alone
const isMarked = (n) => n.stale === 'stale' || n.stale === 'gone'
const plural = (n, word) => `${fmt(n)} ${word}${n === 1 ? '' : 's'}`
const round = (n) => n.toFixed(2)
const before = (a, b) => (a < b ? -1 : a > b ? 1 : 0)

/** Nodes in a stable order (ring, then id) with x/y on their ring. The same nodes give the same numbers, whatever order they arrive in. */
export function layout(nodes) {
  const centre = (nodes.some((n) => n.ring === EASY_RING) ? SIZE_WIDE : SIZE) / 2
  const ordered = [...nodes].sort((a, b) => a.ring - b.ring || before(a.id, b.id))
  const perRing = ordered.reduce((count, n) => ({ ...count, [n.ring]: (count[n.ring] ?? 0) + 1 }), {})
  const index = {}
  return ordered.map((n) => {
    const i = (index[n.ring] = (index[n.ring] ?? -1) + 1)
    const radius = n.ring === 0 && perRing[0] === 1 ? 0 : (RADII[n.ring] ?? RADII.at(-1)) || 24
    const angle = -Math.PI / 2 + n.ring * 0.6 + (2 * Math.PI * i) / perRing[n.ring] // each ring turned a little, so spokes do not line up
    return { ...n, x: centre + Math.round(Math.cos(angle) * radius * 100) / 100, y: centre + Math.round(Math.sin(angle) * radius * 100) / 100 }
  })
}

const short = (name, limit = LABEL_CHARS) => (name.length > limit ? `${name.slice(0, limit - 1)}…` : name)
const label = (n) => `${n.name}, ${KIND_NAMES[n.kind]}${isMarked(n) ? `, ${STALE[n.stale]}` : ''}`
const nodeRadius = (n) => (n.kind === 'easy_memory' ? Math.min(CITED_MAX, CITED_MIN + Math.floor(Math.sqrt(n.cited ?? 1)) - 1) : n.ring === 0 ? 12 : 8)

/** The rings drawing: hand-made SVG, no layout library. */
export function RingsGraph({ nodes, edges, selectedId, onSelect, title = 'Memory graph' }) {
  const placed = layout(nodes)
  const wide = placed.some((n) => n.ring === EASY_RING)
  const size = wide ? SIZE_WIDE : SIZE
  const centre = size / 2
  const at = new Map(placed.map((n) => [n.id, n]))
  const outer = new Map(placed.filter((n) => n.ring === EASY_RING).map((n, i) => [n.id, i])) // position along ring 4
  const labelAll = placed.length <= LABEL_LIMIT
  const touching = (e) => e.source === selectedId || e.target === selectedId
  const drawn = edges
    .filter((e) => at.has(e.source) && at.has(e.target))
    .sort((a, b) => before(a.source + a.target, b.source + b.target))
  return (
    <svg className="memory-svg" viewBox={`0 0 ${size} ${size}`} data-wide={wide || undefined} role="group" aria-label={title}>
      {RADII.slice(1, wide ? RADII.length : EASY_RING).map((r) => (
        <circle key={r} className="memory-ring" cx={centre} cy={centre} r={r} />
      ))}
      {drawn.map((e) => {
        const a = at.get(e.source)
        const b = at.get(e.target)
        return <line key={`${e.source}>${e.target}`} className="memory-edge" data-active={touching(e) || undefined} x1={round(a.x)} y1={round(a.y)} x2={round(b.x)} y2={round(b.y)} />
      })}
      {placed.map((n) => (
        <g
          key={n.id}
          className={`memory-node memory-node-${n.kind}`}
          data-selected={n.id === selectedId || undefined}
          role="button"
          tabIndex={0}
          aria-label={label(n)}
          aria-pressed={n.id === selectedId}
          onClick={() => onSelect(n.id)}
          onKeyDown={(event) => {
            if (event.key === 'Enter' || event.key === ' ') {
              event.preventDefault()
              onSelect(n.id)
            }
          }}
        >
          <circle cx={round(n.x)} cy={round(n.y)} r={nodeRadius(n)} />
          {isMarked(n) && (
            <g className={`memory-mark memory-mark-${n.stale}`} role="img" aria-label={STALE[n.stale]}>
              <circle cx={round(n.x + nodeRadius(n))} cy={round(n.y - nodeRadius(n))} r="6" />
              <text x={round(n.x + nodeRadius(n))} y={round(n.y - nodeRadius(n) + 4)} textAnchor="middle">
                {MARKS[n.stale]}
              </text>
            </g>
          )}
          {(labelAll || n.ring <= 1 || n.id === selectedId) && (
            <text x={round(n.x)} y={round(n.y + (n.ring === 0 ? 26 : outer.get(n.id) % 2 === 1 ? -nodeRadius(n) - 6 : 22))} textAnchor="middle">
              {n.ring === EASY_RING ? short(n.name, EASY_LABEL_CHARS) : short(n.name)}
            </text>
          )}
        </g>
      ))}
    </svg>
  )
}

const hasCited = (node) => node.cited !== undefined

function Panel({ node }) {
  const easy = node?.kind === 'easy_memory'
  return (
    <aside className="memory-panel card" aria-label="Selected note">
      {!node ? (
        <p className="muted">Select a note to see its name, description and path.</p>
      ) : (
        <>
          <h2>{node.name}</h2>
          <p className="muted">
            {KIND_NAMES[node.kind]}
            {node.account && ` · ${node.account}`}
          </p>
          {node.kind === 'dangling' && <p>No note has this name.</p>}
          {node.abstained && <p>This file could not be read (too big or not UTF-8).</p>}
          {node.kind !== 'dangling' && !easy && !node.abstained && <p>{node.description || <span className="muted">No description</span>}</p>}
          {node.path && <code className="memory-path">{node.path}</code>}
          {!easy && (
            <p className="muted">
              links in {node.links_in} · out {node.links_out}
            </p>
          )}
          {hasCited(node) && (
            <ul className="memory-cited">
              <li>cited by {plural(node.cited, 'note')}</li>
              <li>newest note {node.newest}</li>
              <li>{STALE[node.stale] ?? 'not checked'}</li>
            </ul>
          )}
        </>
      )}
    </aside>
  )
}

/** 30 bars, oldest first; a day with no note is a 1-unit tick, so an empty window is a flat line. */
function Sparkline({ days, total }) {
  const window = days.reduce((sum, d) => sum + d.count, 0)
  const top = Math.max(1, ...days.map((d) => d.count))
  return (
    <svg className="memory-spark" viewBox="0 0 120 28" role="img" aria-label={`Notes saved per day, last 30 days: ${fmt(window)} in the window, ${fmt(total)} notes in total`}>
      {days.map((d, i) => {
        const height = d.count === 0 ? 1 : Math.max(2, Math.round((d.count / top) * 26))
        return <rect key={d.date} x={i * 4} y={27 - height} width="3" height={height} />
      })}
    </svg>
  )
}

/** The age of the report, or '' when `generated` does not parse or lies in the future. */
function updated(generated) {
  const time = Date.parse(generated)
  return Number.isNaN(time) || time > Date.now() ? '' : `updated ${ago(generated)}`
}

const groupText = (g) => (g.sessions === 0 || g.tokens_all_median === null ? 'no sessions' : `${compact(g.tokens_all_median)} tokens / session`)

function SavingLine({ report }) {
  const { on, off, min_sessions: need, too_few: tooFew, on_vs_off_pct: pct } = report
  if (tooFew) return <p className="muted">{`too few sessions to compare (${fmt(on.sessions)} on / ${fmt(off.sessions)} off, need ${fmt(need)} each)`}</p>
  if (pct === null) return null
  const whole = Math.round(Math.abs(pct))
  return <p>{pct < 0 && whole > 0 ? `Saves ${whole}%` : pct > 0 && whole > 0 ? `Uses ${whole}% more` : 'No difference'}</p>
}

/** FR-065: sessions and median tokens with easy-memory on and off, read from its report.json. Numbers only. */
function MemoryOnOff({ report }) {
  const age = report.state === 'ok' ? updated(report.generated) : ''
  return (
    <div className="memory-report" role="group" aria-label="memory on vs off">
      <p>
        <strong>Memory on vs off</strong>
        {age && <span className="muted"> {age}</span>}
      </p>
      {report.state === 'unsupported' && <p className="muted">report format not supported</p>}
      {report.state === 'none' && <p className="muted">no report yet</p>}
      {report.state === 'ok' && (
        <>
          {[['On', report.on], ['Off', report.off]].map(([label, g]) => (
            <p key={label} className="memory-report-row">
              <span>{label}</span> <span>{plural(g.sessions, 'session')}</span> <span>{groupText(g)}</span>
            </p>
          ))}
          <SavingLine report={report} />
        </>
      )}
    </div>
  )
}

/** FR-053: notes per day, injection hit rate, files not drawn, abstained notes. Counts only; no note text. */
function EasyMemoryPanel({ data }) {
  const rate = data.prompts_on > 0 ? `${percent(data.injected / data.prompts_on)} of ${compact(data.prompts_on)} prompt${data.prompts_on === 1 ? '' : 's'}` : 'no prompts'
  return (
    <section className="memory-easy card" aria-label="easy-memory">
      <h2>easy-memory</h2>
      <p className="memory-easy-total">{plural(data.notes, 'note')}</p>
      <Sparkline days={data.days} total={data.notes} />
      <p className="muted memory-easy-caption">saved per day, last 30 days</p>
      <p>
        <span className="muted">Hit rate </span>
        {rate}
      </p>
      {data.report && <MemoryOnOff report={data.report} />}
      {data.files_not_drawn > 0 && <p className="muted">{plural(data.files_not_drawn, 'file')} not drawn</p>}
      {data.abstained > 0 && <p className="muted">{plural(data.abstained, 'note')} could not be read</p>}
    </section>
  )
}

function ProjectGraph({ project }) {
  const [selectedId, setSelectedId] = useState(null)
  const state = useLoad(() => getJson(`/api/memory-graph?project=${encodeURIComponent(project)}`), [project])
  if (state.status === 'loading') return <p role="status">Loading</p>
  if (state.status === 'error') return <p role="alert" className="error">Could not load the memory graph.</p>
  const { nodes, edges, counts, name } = state.data
  const easy = state.data.easy_memory ?? null
  const ordered = layout(nodes)
  const inner = ordered.filter((n) => n.ring !== EASY_RING)
  const shown = [...inner.slice(0, MAX_DRAWN), ...ordered.filter((n) => n.ring === EASY_RING)] // the API sends at most 30 easy-memory nodes: never cut by the cap
  const notDrawn = Math.max(0, inner.length - MAX_DRAWN)
  const selected = nodes.find((n) => n.id === selectedId) ?? null
  const facts = [plural(counts.notes, 'note'), plural(counts.links, 'link'), `${fmt(counts.dangling)} unresolved`]
  if (counts.abstained > 0) facts.push(`${fmt(counts.abstained)} could not be read`)
  if (counts.duplicates > 0) facts.push(plural(counts.duplicates, 'duplicate name'))
  const kinds = Object.entries(KIND_NAMES).filter(([kind]) => kind !== 'easy_memory' || easy)
  return (
    <>
      <div className="memory-body" data-wide={ordered.some((n) => n.ring === EASY_RING) || undefined}>
        <div className="memory-graph">
          <RingsGraph nodes={shown} edges={edges} selectedId={selectedId} onSelect={setSelectedId} title={`Memory graph of ${name}`} />
        </div>
        <div className="memory-side">
          <Panel node={selected} />
          {easy && <EasyMemoryPanel data={easy} />}
        </div>
      </div>
      <ul className="memory-legend" aria-label="Legend">
        {kinds.map(([kind, text]) => (
          <li key={kind}>
            <svg width="14" height="14" viewBox="0 0 14 14" aria-hidden="true" focusable="false">
              <circle className={`memory-swatch memory-node-${kind}`} cx="7" cy="7" r="5" />
            </svg>
            {text}
          </li>
        ))}
        {nodes.some(isMarked) && <li className="muted">! changed since the newest note · × file gone</li>}
        <li className="muted">{facts.join(' · ')}</li>
      </ul>
      {notDrawn > 0 && <p className="muted">{plural(notDrawn, 'more note')} not drawn</p>}
      {counts.capped && <p className="muted">This project has more memory files than were read.</p>}
    </>
  )
}

const noteCount = (p) => (p.kit_notes ?? 0) + (p.claude_notes ?? 0) + (p.easy_memory_notes ?? 0)

/** "alpha · kit 4 · Claude 1 · easy-memory 6": the picker text; a count of 0 is left out. */
const optionText = (p) =>
  [p.name, ...[['kit', p.kit_notes], ['Claude', p.claude_notes], ['easy-memory', p.easy_memory_notes]].filter(([, n]) => n > 0).map(([word, n]) => `${word} ${compact(n)}`)].join(' · ')

const isRealPath = (p) => p.project.startsWith('/') // a Claude-folder-only key is not an absolute path

/** Real project paths first, then the most notes (kit + Claude + easy-memory), then the lower name. */
export function busiest(projects) {
  return [...projects].sort((a, b) => Number(isRealPath(b)) - Number(isRealPath(a)) || noteCount(b) - noteCount(a) || before(a.name, b.name))[0]?.project
}

/** `// work` › Memory: one project's memory notes as rings. Names and links only; no note body is ever sent. */
export default function MemoryView() {
  const [chosen, setChosen] = useState(null)
  const list = useLoad(() => getJson('/api/memory-graph'), [])
  const projects = list.status === 'ready' ? list.data.projects : []
  const project = projects.some((p) => p.project === chosen) ? chosen : busiest(projects)
  return (
    <main className="memory">
      <h1>Memory</h1>
      {list.status === 'loading' && <p role="status">Loading</p>}
      {list.status === 'error' && (
        <p role="alert" className="error">
          Could not load the memory graph.
        </p>
      )}
      {list.status === 'ready' && projects.length === 0 && <p>No memory notes found.</p>}
      {list.status === 'ready' && list.data.gone > 0 && <p className="muted gone-note">{plural(list.data.gone, 'gone folder')} hidden</p>}
      {project && (
        <>
          <div className="field memory-picker">
            <label htmlFor="memory-project">Project</label>
            <select id="memory-project" value={project} onChange={(event) => setChosen(event.target.value)}>
              {projects.map((p) => (
                <option key={p.project} value={p.project}>
                  {optionText(p)}
                </option>
              ))}
            </select>
          </div>
          <ProjectGraph key={project} project={project} />
        </>
      )}
    </main>
  )
}
