import { useState } from 'react'
import { getJson } from './api.js'
import { fmt } from './format.js'
import { useLoad } from './useLoad.js'

export const MAX_DRAWN = 300 // more nodes than this are counted, not drawn
const SIZE = 640
const CENTRE = SIZE / 2
const RADII = [0, 115, 200, 265] // ring 0 = MEMORY.md, 1 = cold files, 2 = records and Claude notes, 3 = unresolved links
const LABEL_LIMIT = 60 // above this many drawn nodes only the inner rings are labelled
const LABEL_CHARS = 14

const KIND_NAMES = { kit: 'kit memory', claude: 'Claude memory', dangling: 'unresolved link' }
const plural = (n, word) => `${fmt(n)} ${word}${n === 1 ? '' : 's'}`
const round = (n) => n.toFixed(2)
const before = (a, b) => (a < b ? -1 : a > b ? 1 : 0)

/** Nodes in a stable order (ring, then id) with x/y on their ring. The same nodes give the same numbers, whatever order they arrive in. */
export function layout(nodes) {
  const ordered = [...nodes].sort((a, b) => a.ring - b.ring || before(a.id, b.id))
  const perRing = ordered.reduce((count, n) => ({ ...count, [n.ring]: (count[n.ring] ?? 0) + 1 }), {})
  const index = {}
  return ordered.map((n) => {
    const i = (index[n.ring] = (index[n.ring] ?? -1) + 1)
    const radius = n.ring === 0 && perRing[0] === 1 ? 0 : (RADII[n.ring] ?? RADII.at(-1)) || 24
    const angle = -Math.PI / 2 + n.ring * 0.6 + (2 * Math.PI * i) / perRing[n.ring] // each ring turned a little, so spokes do not line up
    return { ...n, x: CENTRE + Math.round(Math.cos(angle) * radius * 100) / 100, y: CENTRE + Math.round(Math.sin(angle) * radius * 100) / 100 }
  })
}

const short = (name) => (name.length > LABEL_CHARS ? `${name.slice(0, LABEL_CHARS - 1)}…` : name)
const label = (n) => `${n.name}, ${KIND_NAMES[n.kind]}`

/** The rings drawing: hand-made SVG, no layout library. */
export function RingsGraph({ nodes, edges, selectedId, onSelect, title = 'Memory graph' }) {
  const placed = layout(nodes)
  const at = new Map(placed.map((n) => [n.id, n]))
  const labelAll = placed.length <= LABEL_LIMIT
  const touching = (e) => e.source === selectedId || e.target === selectedId
  const drawn = edges
    .filter((e) => at.has(e.source) && at.has(e.target))
    .sort((a, b) => before(a.source + a.target, b.source + b.target))
  return (
    <svg className="memory-svg" viewBox={`0 0 ${SIZE} ${SIZE}`} role="group" aria-label={title}>
      {RADII.slice(1).map((r) => (
        <circle key={r} className="memory-ring" cx={CENTRE} cy={CENTRE} r={r} />
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
          <circle cx={round(n.x)} cy={round(n.y)} r={n.ring === 0 ? 12 : 8} />
          {(labelAll || n.ring <= 1 || n.id === selectedId) && (
            <text x={round(n.x)} y={round(n.y + (n.ring === 0 ? 26 : 22))} textAnchor="middle">
              {short(n.name)}
            </text>
          )}
        </g>
      ))}
    </svg>
  )
}

function Panel({ node }) {
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
          {node.kind !== 'dangling' && !node.abstained && <p>{node.description || <span className="muted">No description</span>}</p>}
          {node.path && <code className="memory-path">{node.path}</code>}
          <p className="muted">
            links in {node.links_in} · out {node.links_out}
          </p>
        </>
      )}
    </aside>
  )
}

function ProjectGraph({ project }) {
  const [selectedId, setSelectedId] = useState(null)
  const state = useLoad(() => getJson(`/api/memory-graph?project=${encodeURIComponent(project)}`), [project])
  if (state.status === 'loading') return <p role="status">Loading</p>
  if (state.status === 'error') return <p role="alert" className="error">Could not load the memory graph.</p>
  const { nodes, edges, counts, name } = state.data
  const ordered = layout(nodes)
  const shown = ordered.slice(0, MAX_DRAWN)
  const notDrawn = ordered.length - shown.length
  const selected = nodes.find((n) => n.id === selectedId) ?? null
  const facts = [plural(counts.notes, 'note'), plural(counts.links, 'link'), `${fmt(counts.dangling)} unresolved`]
  if (counts.abstained > 0) facts.push(`${fmt(counts.abstained)} could not be read`)
  if (counts.duplicates > 0) facts.push(plural(counts.duplicates, 'duplicate name'))
  return (
    <>
      <div className="memory-body">
        <div className="memory-graph">
          <RingsGraph nodes={shown} edges={edges} selectedId={selectedId} onSelect={setSelectedId} title={`Memory graph of ${name}`} />
        </div>
        <Panel node={selected} />
      </div>
      <ul className="memory-legend" aria-label="Legend">
        {Object.entries(KIND_NAMES).map(([kind, text]) => (
          <li key={kind}>
            <svg width="14" height="14" viewBox="0 0 14 14" aria-hidden="true" focusable="false">
              <circle className={`memory-swatch memory-node-${kind}`} cx="7" cy="7" r="5" />
            </svg>
            {text}
          </li>
        ))}
        <li className="muted">{facts.join(' · ')}</li>
      </ul>
      {notDrawn > 0 && <p className="muted">{plural(notDrawn, 'more note')} not drawn</p>}
      {counts.capped && <p className="muted">This project has more memory files than were read.</p>}
    </>
  )
}

const noteCount = (p) => (p.kit_notes ?? 0) + (p.claude_notes ?? 0) + (p.easy_memory_notes ?? 0)

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
                  {p.name}
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
