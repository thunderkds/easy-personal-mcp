import { Fragment, useContext, useEffect, useId, useLayoutEffect, useMemo, useRef, useState } from 'react'
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

function Line({ flow, placed, working, panel, panelId }) {
  const steps = [...stepsOf(flow), ...(flow.id === KIT ? extraSteps(flow, placed) : [])]
  const gates = list(flow.gates).map((g, n) => ({ ...g, key: `gate:${n}` }))
  const unplaced = gates.filter((g) => !steps.some((s) => s.id === g.after))
  const marker = (g) => (
    <button key={g.key} type="button" className="workflow-gate" aria-label={`Gate · ${g.text}`} aria-expanded={panel.open === g.key} aria-controls={panel.open === g.key ? panelId : undefined} {...panel.bind(g.key)}>
      <span aria-hidden="true">◆</span>
    </button>
  )
  return (
    <ol className="workflow-line" aria-label="Steps">
      {steps.map((step, i) => {
        const key = `node:${step.id}`
        const n = working.filter((r) => r.flow.id === flow.id && r.step?.id === step.id).length
        const last = i === steps.length - 1
        const after = gates.filter((g) => g.after === step.id).concat(last ? unplaced : [])
        return (
          <Fragment key={step.id}>
            <li className="workflow-node-item">
              <button
                type="button"
                className={`workflow-node${n > 0 ? ' working' : ''}`}
                aria-label={n > 0 ? `${step.name} · ${n} working` : undefined}
                aria-expanded={panel.open === key}
                aria-controls={panel.open === key ? panelId : undefined}
                {...panel.bind(key)}
              >
                {step.name}
              </button>
            </li>
            {(!last || after.length > 0) && (
              <li className="workflow-arrow">
                {!last && <span aria-hidden="true">→</span>}
                {after.map(marker)}
              </li>
            )}
          </Fragment>
        )
      })}
    </ol>
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
        <Line flow={flow} placed={placed} working={working} panel={panel} panelId={panelId} />
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
