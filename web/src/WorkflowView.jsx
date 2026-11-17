import { useContext, useEffect, useId, useLayoutEffect, useMemo, useRef, useState } from 'react'
import { getJson } from './api.js'
import { duration } from './format.js'
import { LiveContext } from './LiveView.jsx'
import Loading from './Loading.jsx'
import { useLoad } from './useLoad.js'

const WORKING_MS = 5 * 60_000 // an agent whose newest call is this recent is working (FR-118)
const TICK_MS = 15_000
const NOT_A_TYPE = new Set(['main', 'sub-agent']) // what the Live feed says when a call has no sub-agent type
const NOISE = new Set(['agent', 'developer', 'implementer', 'expert', 'automation'])
const KIT = 'easy-kit' // the built-in flow a session with a known stage is put on (FR-118)
const PLAIN = 'plain' // the built-in flow of a session with no kit

// The flows are the route's (FR-117); the page holds no copy of them.
const loadMap = () => getJson('/api/kit/workflow')

/** A role without its suffix and case: `Backend-Implementer`, `backend-developer` and `backend` are one role. */
function role(name) {
  const words = String(name).toLowerCase().split(/[^a-z0-9]+/).filter(Boolean)
  const kept = words.filter((w) => !NOISE.has(w))
  return (kept.length > 0 ? kept : words).join('-')
}
const isRole = (list, name) => typeof name === 'string' && list.some((a) => role(a) === role(name))

const list = (v) => (Array.isArray(v) ? v : [])
const stepsOf = (flow) => list(flow?.steps)
const signalsOf = (step) => step?.signals ?? {}

/** Does one feed event put its session on this step? A tool name, a Bash first word, a sub-agent type or a Skill label (FR-117). */
function matches(step, e) {
  const sig = signalsOf(step)
  return (
    list(sig.tools).includes(e.tool) ||
    (e.tool === 'Bash' && list(sig.commands).includes(e.label)) ||
    isRole(list(sig.agents), e.agent) ||
    (e.tool === 'Skill' && list(sig.skills).includes(e.label))
  )
}

const newestFirst = (a, b) => (Date.parse(b.ts) || 0) - (Date.parse(a.ts) || 0) || (b.id ?? 0) - (a.id ?? 0)

/**
 * The flow and the step of one live session (FR-118). Flow: the one its project is assigned to, else easy-kit when its stage is known, else plain.
 * Step: on easy-kit the one whose signals hold the stage (a stage no step holds gets a step of its own), elsewhere the one the session's newest matching
 * event points at. `step` is null while the step is not known yet. `events` carry session_id, tool, label, agent, ts (and an id to order equal times).
 */
export function placeSession(session, events, flows, projects) {
  const all = list(flows)
  const byId = (id) => all.find((f) => f.id === id)
  const hasStage = session.stage !== null && session.stage !== undefined
  const flow = byId(projects?.[session.project]) ?? (hasStage ? (byId(KIT) ?? all[0]) : (byId(PLAIN) ?? all[0])) ?? null
  if (!flow) return { flow: null, step: null }
  if (flow.id === KIT) {
    if (!hasStage) return { flow, step: null }
    const stage = String(session.stage)
    const step = stepsOf(flow).find((s) => list(signalsOf(s).stages).includes(stage))
    return { flow, step: step ?? { id: stage, name: `Stage ${stage}`, parts: [], agents: [], signals: {} } }
  }
  const mine = list(events).filter((e) => e.session_id === session.session_id).sort(newestFirst)
  for (const e of mine) {
    const step = stepsOf(flow).find((s) => matches(s, e))
    if (step) return { flow, step }
  }
  return { flow, step: null }
}

/** The feed rows the page already holds, as the events `placeSession` and `workingAgents` read. A row's `who` is the sub-agent type, "sub-agent" or "main" (LiveView). */
function eventsOf(sessions) {
  return sessions.flatMap((s) =>
    s.feed.map((item) => ({
      id: item.id,
      session_id: s.session_id,
      tool: item.tool,
      label: item.label,
      agent: NOT_A_TYPE.has(item.who) ? undefined : item.who,
      sidechain: item.who !== 'main',
      ts: item.ts,
    })),
  )
}

const agentName = (e) => e.agent ?? (e.sidechain ? 'sub-agent' : 'main session')
const asTime = (e) => Date.parse(e.ts) || 0

/**
 * Who is working where (FR-118). One row for each (session, agent) whose newest call is at most 5 minutes old; the agent is the sub-agent type, "sub-agent" when a
 * sidechain call names none, or "main session". On easy-kit the agent sits on its session's step (the stage). On any other flow it sits on the step its own newest
 * matching call points at, else on its session's step; `step` is null while neither is known. `first` is the agent's first call at that step, `last` its newest call.
 */
export function workingAgents(sessions, events, flows, projects, now) {
  const rows = []
  for (const session of list(sessions)) {
    const mine = list(events).filter((e) => e.session_id === session.session_id)
    const home = placeSession(session, mine, flows, projects)
    if (!home.flow) continue
    const groups = new Map()
    for (const e of mine) groups.set(agentName(e), [...(groups.get(agentName(e)) ?? []), e])
    for (const [agent, calls] of groups) {
      const newest = [...calls].sort(newestFirst)[0]
      if (!(now - Date.parse(newest.ts) <= WORKING_MS)) continue
      let step = home.step
      let here = calls
      if (home.flow.id !== KIT) {
        const own = [...calls].sort(newestFirst).find((e) => stepsOf(home.flow).some((s) => matches(s, e)))
        if (own) {
          step = stepsOf(home.flow).find((s) => matches(s, own))
          here = calls.filter((e) => matches(step, e))
        }
      }
      const first = here.reduce((a, b) => (asTime(b) < asTime(a) ? b : a))
      rows.push({ session_id: session.session_id, project: session.project, agent, flow: home.flow, step, first: first.ts, last: newest.ts })
    }
  }
  return rows
}

/** Time taken, as the page's other durations read: "45s" under a minute, then "12m", "1h 5m". */
export function takenText(first, last) {
  const ms = Date.parse(last) - Date.parse(first)
  return ms < 60_000 ? `${Math.max(0, Math.floor(ms / 1000))}s` : duration(first, last)
}

const byWho = (a, b) => a.project.localeCompare(b.project) || a.agent.localeCompare(b.agent)

const LINK = {
  connecting: 'Connecting to the live stream.',
  reconnecting: 'The live stream dropped: showing what we had.',
  closed: 'The live stream closed: showing what we had.',
}

/** The stage ids the stream sends that no step of easy-kit holds: one extra node each, at the end, never dropped. */
function extraSteps(flow, placed) {
  const known = new Set(stepsOf(flow).map((s) => s.id))
  const extra = new Map()
  for (const p of placed) if (p.flow.id === flow.id && p.step && !known.has(p.step.id)) extra.set(p.step.id, p.step)
  return [...extra.values()].sort((a, b) => a.id.localeCompare(b.id))
}

/** What is open: hovered, focused or pinned by a tap or click. Escape, a blur and a second tap close it. One panel at a time; `key` is `node:<id>` or `gate:<n>`. */
function usePanel() {
  const [state, setState] = useState({ hover: null, focus: null, pinned: null })
  const open = state.pinned ?? state.hover ?? state.focus
  const bind = (key) => ({
    'data-key': key,
    onMouseEnter: () => setState((s) => ({ ...s, hover: key })),
    onMouseLeave: () => setState((s) => ({ ...s, hover: s.hover === key ? null : s.hover })),
    onFocus: () => setState((s) => ({ ...s, focus: key })),
    onBlur: () => setState((s) => ({ hover: s.hover, focus: s.focus === key ? null : s.focus, pinned: s.pinned === key ? null : s.pinned })),
    onClick: () => setState((s) => (s.pinned === key ? { hover: null, focus: null, pinned: null } : { ...s, pinned: key })),
  })
  const close = () => setState({ hover: null, focus: null, pinned: null })
  return { open, bind, close }
}

// The drawing (owner reference 2026-11-17): fixed node sizes, positions computed here so the edges meet the ports. The row wraps to the canvas width so the whole flow shows.
const PAD = 16
const GAP = 76 // between two cards of a row; a gate label sits on the edge in it
const ROW = 112 // the height of one row
const CARD = { w: 168, h: 52 }
const START = { w: CARD.w, h: 64 }
const END = { w: 8, h: 8 } // the end of the trailing edge a last gate sits on
const PILL = { w: 60, h: 22 }
const PILL_CHARS = 6
const FALLBACK_WIDTH = 700 // the canvas width when it cannot be measured (tests, no layout)

/** How many nodes fit in one row of a canvas `width` px wide (at least one). */
export const columnsFor = (width) => Math.max(1, Math.floor((width - 2 * PAD + GAP) / (CARD.w + GAP)))

const shortText = (text) => (text.length > PILL_CHARS ? `${text.slice(0, PILL_CHARS).trimEnd()}…` : text)
/** The icon tile of a step card: its id when short ("3", "1.5"), else the first letter of its name. */
const monogram = (step) => (step.id.length <= 3 ? step.id : step.name.charAt(0).toUpperCase())

/** One cubic bezier from the right port of one node to the left port of the next; going down to the next row it is a smooth S. */
function curve(a, b) {
  const x1 = a.x + a.w
  const down = b.row !== a.row
  const dx = down ? 34 : (b.x - x1) / 2
  return `M ${x1} ${a.cy} C ${x1 + dx} ${a.cy}, ${b.x - dx} ${b.cy}, ${b.x} ${b.cy}`
}

/** The canvas width, kept up to date; `FALLBACK_WIDTH` when it has none. */
function useWidth(ref) {
  const [width, setWidth] = useState(0)
  useLayoutEffect(() => {
    const el = ref.current
    if (!el) return undefined
    const read = () => setWidth(el.clientWidth)
    read()
    if (typeof ResizeObserver === 'undefined') return undefined
    const watch = new ResizeObserver(read)
    watch.observe(el)
    return () => watch.disconnect()
  }, [ref])
  return width || FALLBACK_WIDTH
}

function Line({ flow, placed, working, live, panel, panelId }) {
  const canvas = useRef(null)
  const width = useWidth(canvas)
  const steps = [...stepsOf(flow), ...(flow.id === KIT ? extraSteps(flow, placed) : [])]
  const gates = list(flow.gates).map((g, n) => ({ ...g, key: `gate:${n}` }))
  const unplaced = gates.filter((g) => !steps.some((s) => s.id === g.after))
  const here = working.filter((r) => r.flow.id === flow.id)
  const count = (step) => here.filter((r) => r.step?.id === step.id).length
  const sessions = placed.filter((p) => p.flow.id === flow.id).length
  // The nodes: the start card, then the step cards. Gates are labels on the edge after their step; the gates after the last step sit on a short edge to an end mark.
  const after = (step, i) => gates.filter((g) => g.after === step.id).concat(i === steps.length - 1 ? unplaced : [])
  const nodes = [{ kind: 'start', key: 'start', ...START, active: false, gates: [] }]
  steps.forEach((step, i) => {
    const n = count(step)
    nodes.push({ kind: 'step', key: `node:${step.id}`, step, n, ...CARD, active: n > 0, gates: [] })
    const g = after(step, i)
    if (i < steps.length - 1) nodes.at(-1).gatesAfter = g
    else if (g.length > 0) nodes.push({ kind: 'end', key: 'end', ...END, active: false, gates: g })
  })
  for (let i = 0; i < nodes.length - 1; i++) nodes[i + 1].gates = nodes[i].gatesAfter ?? nodes[i + 1].gates
  const cols = Math.min(columnsFor(width), nodes.length)
  nodes.forEach((node, i) => {
    node.row = Math.floor(i / cols)
    node.x = PAD + (i % cols) * (CARD.w + GAP)
    node.cy = node.row * ROW + ROW / 2
    node.y = node.cy - node.h / 2
  })
  const graphW = 2 * PAD + cols * CARD.w + (cols - 1) * GAP
  const graphH = (nodes.at(-1).row + 1) * ROW
  const edges = nodes.slice(1).map((node, i) => ({ from: nodes[i], to: node, active: i === 0 ? here.length > 0 : node.active }))
  return (
    <div className="workflow-canvas" ref={canvas}>
      <div className="workflow-graph" style={{ width: graphW, height: graphH }}>
        <svg className="workflow-edges" width={graphW} height={graphH} aria-hidden="true" focusable="false">
          {edges.map((e) => (
            <path key={e.to.key} className={`workflow-edge ${e.active ? 'active' : 'idle'}`} data-to={e.to.key} d={curve(e.from, e.to)} />
          ))}
        </svg>
        <ol className="workflow-line" aria-label="Steps">
          {nodes.map((node) => {
            const box = { left: node.x, top: node.y, width: node.w, height: node.h }
            if (node.kind === 'start') {
              return (
                <li key={node.key} className="workflow-start" style={box}>
                  <span className="workflow-start-title">Live sessions</span>
                  <span className="workflow-sub">{live ? `${sessions} live` : 'no live stream'}</span>
                  <span className="workflow-port right" aria-hidden="true" />
                </li>
              )
            }
            if (node.kind === 'end') return <li key={node.key} className="workflow-end" style={box} aria-hidden="true" />
            const { step, n } = node
            return (
              <li key={node.key} className="workflow-node-item" style={box}>
                <button
                  type="button"
                  className={`workflow-node${n > 0 ? ' working' : ''}`}
                  aria-label={n > 0 ? `${step.name} · ${n} working` : step.name}
                  aria-expanded={panel.open === node.key}
                  aria-controls={panel.open === node.key ? panelId : undefined}
                  {...panel.bind(node.key)}
                >
                  <span className="workflow-port left" aria-hidden="true" />
                  <span className="workflow-tile" aria-hidden="true">{monogram(step)}</span>
                  <span className="workflow-text" aria-hidden="true">
                    <span className="workflow-title">{step.name}</span>
                    <span className="workflow-sub">{n > 0 ? `${n} working` : 'idle'}</span>
                  </span>
                  <span className="workflow-port right" aria-hidden="true" />
                </button>
              </li>
            )
          })}
        </ol>
        {edges.flatMap((e) =>
          e.to.gates.map((g, i) => {
            const mx = (e.from.x + e.from.w + e.to.x) / 2
            const my = (e.from.cy + e.to.cy) / 2 + (i - (e.to.gates.length - 1) / 2) * (PILL.h + 4)
            return (
              <button
                key={g.key}
                type="button"
                className={`workflow-gate ${e.active ? 'active' : 'idle'}`}
                style={{ left: mx - PILL.w / 2, top: my - PILL.h / 2, width: PILL.w, height: PILL.h }}
                aria-label={`Gate · ${g.text}`}
                aria-expanded={panel.open === g.key}
                aria-controls={panel.open === g.key ? panelId : undefined}
                {...panel.bind(g.key)}
              >
                <span aria-hidden="true">{shortText(g.text)}</span>
              </button>
            )
          }),
        )}
      </div>
    </div>
  )
}

/** The panel of the open node or gate: its working agents (project · agent · time), or the gate's text. It sits under the strip, clamped inside the column. */
function Panel({ id, open, flow, steps, gates, working, wrap }) {
  const ref = useRef(null)
  const [left, setLeft] = useState(0)
  useLayoutEffect(() => {
    const anchor = open && wrap.current ? wrap.current.querySelector(`[data-key="${open}"]`) : null
    if (!anchor || !ref.current) return
    const box = wrap.current.getBoundingClientRect()
    const at = anchor.getBoundingClientRect().left - box.left
    setLeft(Math.max(0, Math.min(at, box.width - ref.current.offsetWidth)))
  }, [open, wrap])
  if (!open) return null
  const [kind, which] = open.split(':')
  const step = kind === 'node' ? steps.find((s) => s.id === which) : null
  const gate = kind === 'gate' ? gates[Number(which)] : null
  if (!step && !gate) return null
  const rows = step ? working.filter((r) => r.flow.id === flow.id && r.step?.id === step.id).sort(byWho) : []
  return (
    <div id={id} ref={ref} role="region" aria-label={step ? `${step.name}: agents working` : 'Gate'} className="workflow-panel" style={{ left }}>
      {gate && <p>{gate.text}</p>}
      {step && rows.length === 0 && <p className="muted">No agent working now.</p>}
      {step && rows.length > 0 && (
        <ul>
          {rows.map((r) => (
            <li key={`${r.session_id}-${r.agent}`}>{`${r.project} · ${r.agent} · ${takenText(r.first, r.last)}`}</li>
          ))}
        </ul>
      )}
    </div>
  )
}

function Workflow({ map, live, now }) {
  const flows = useMemo(() => [...list(map.flows)].sort((a, b) => (b.id === KIT) - (a.id === KIT)), [map])
  const [chosen, setChosen] = useState(null)
  const panel = usePanel()
  const panelId = useId()
  const wrap = useRef(null)
  const flow = flows.find((f) => f.id === chosen) ?? flows[0]
  const sessions = useMemo(() => (live ? Object.values(live.state.sessions) : []), [live])
  const events = useMemo(() => eventsOf(sessions), [sessions])
  const placed = useMemo(
    () => sessions.map((session) => ({ session, ...placeSession(session, events, flows, map.projects) })).filter((p) => p.flow),
    [sessions, events, flows, map.projects],
  )
  const working = useMemo(() => workingAgents(sessions, events, flows, map.projects, now), [sessions, events, flows, map.projects, now])
  if (!flow) return <p className="muted">The route sent no flow.</p>
  const steps = [...stepsOf(flow), ...(flow.id === KIT ? extraSteps(flow, placed) : [])]
  const lost = [...new Set(placed.filter((p) => p.flow.id === flow.id && !p.step).map((p) => p.session.project))].sort()
  return (
    <>
      <div className="periods" role="group" aria-label="Flow">
        {flows.map((f) => (
          <button key={f.id} type="button" aria-pressed={f.id === flow.id} onClick={() => setChosen(f.id)}>
            {f.name}
          </button>
        ))}
      </div>
      <h2>{flow.name}</h2>
      <div className="workflow-stage" ref={wrap} onKeyDown={(e) => e.key === 'Escape' && panel.close()}>
        <Line flow={flow} placed={placed} working={working} live={live} panel={panel} panelId={panelId} />
        <Panel id={panelId} open={panel.open} flow={flow} steps={steps} gates={list(flow.gates)} working={working} wrap={wrap} />
      </div>
      {lost.length > 0 && <p className="muted workflow-unknown">{`step not known yet: ${lost.join(', ')}`}</p>}
    </>
  )
}

/** `// kit` › Workflow: a flow picker and the chosen flow (the kit's pipeline, plain prompts, or the owner's) as one line of nodes. A node with an agent working now is highlighted; hover, focus or a tap opens who works there: project, agent, time taken. Metadata only (NFR-002). */
export default function WorkflowView() {
  const map = useLoad(loadMap, [])
  const live = useContext(LiveContext) // null outside a LiveProvider: no live layer
  const [now, setNow] = useState(Date.now())
  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), TICK_MS)
    return () => clearInterval(timer)
  }, [])
  const liveLine = !live ? 'No live stream on this page: sessions and agents are not placed.' : LINK[live.link]
  return (
    <main className="workflow">
      <h1>Workflow</h1>
      <p className="muted">Pick a flow. A highlighted step has an agent working now; hover it to see who.</p>
      {liveLine && <p className="muted">{liveLine}</p>}
      {map.status === 'loading' && <Loading label="Loading workflow" />}
      {map.status === 'error' && (
        <p role="alert" className="error">
          Could not load workflow: {map.message}
        </p>
      )}
      {map.status === 'ready' && <Workflow map={map.data} live={live} now={now} />}
    </main>
  )
}
