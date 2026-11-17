import { useContext, useEffect, useMemo, useState } from 'react'
import { getJson } from './api.js'
import FeedCard from './FeedCard.jsx'
import { LiveContext } from './LiveView.jsx'
import Loading from './Loading.jsx'
import { NO_SCOPE, scopedPath } from './Scope.jsx'
import { useLoad } from './useLoad.js'

const AGENT_WINDOW_MS = 5 * 60_000 // a sub-agent seen this recently counts as working (FR-118)
const TICK_MS = 15_000
const MAX_CHIPS = 5 // session chips per step; the rest is "+N more"
const NOT_A_TYPE = new Set(['main', 'sub-agent']) // what the Live feed says when a call has no sub-agent type
const NOISE = new Set(['agent', 'developer', 'implementer', 'expert', 'automation'])
const KIT = 'easy-kit' // the built-in flow a session with a known stage is put on (FR-118)
const PLAIN = 'plain' // the built-in flow of a session with no kit
const STAGE_3 = '3' // the step where Todo tasks wait and In Progress tasks sit
const STAGE_4 = '4' // the step for Ready for Review: reviews (4) and verify (5)

// The flows are the route's (FR-117); only where a board status lands is the page's own rule.
const loadMap = () => getJson('/api/kit/workflow')
const loadBoard = (scope) => getJson(scopedPath('/api/board', scope))

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

/** The feed rows the page already holds, as the events `placeSession` reads. A row's `who` is the sub-agent type, "sub-agent" or "main" (LiveView). */
function eventsOf(sessions) {
  return sessions.flatMap((s) =>
    s.feed.map((item) => ({ id: item.id, session_id: s.session_id, tool: item.tool, label: item.label, agent: NOT_A_TYPE.has(item.who) ? undefined : item.who, ts: item.ts })),
  )
}

/** Sub-agent types the feeds show in the last 5 minutes. */
function seenAgents(events, now) {
  const types = new Set()
  for (const e of events) {
    if (typeof e.agent === 'string' && now - Date.parse(e.ts) <= AGENT_WINDOW_MS) types.add(e.agent)
  }
  return [...types].sort()
}

/** Board tasks of the answer by the four statuses the steps use; `total` counts every task, whatever its status. */
function countBoard(data) {
  const count = { total: 0, todo: 0, progress: 0, review: 0, done: 0 }
  for (const p of data.projects ?? []) {
    for (const t of p.tasks ?? []) {
      count.total += 1
      if (t.status === 'Todo') count.todo += 1
      else if (t.status === 'In Progress') count.progress += 1
      else if (t.status === 'Ready for Review') count.review += 1
      else if (t.status === 'Done') count.done += 1
    }
  }
  return count
}

const LINK = {
  connecting: 'Connecting to the live stream.',
  reconnecting: 'The live stream dropped: showing what we had.',
  closed: 'The live stream closed: showing what we had.',
}

const stepKey = (flow, id) => `workflow-step-${flow}-${String(id).replace(/\W+/g, '-')}`
const bySession = (a, b) => a.project.localeCompare(b.project) || a.session_id.localeCompare(b.session_id)

function Chips({ label, items, className = 'chip', more = 0 }) {
  return (
    <div>
      <dt>{label}</dt>
      <dd>
        <ul className="workflow-chips" aria-label={label}>
          {items.map((c) => (
            <li key={c.key} className={className}>
              {c.text}
            </li>
          ))}
          {more > 0 && <li className="muted">{`+${more} more`}</li>}
        </ul>
      </dd>
    </div>
  )
}

const sessionChips = (here) => ({ items: here.slice(0, MAX_CHIPS).map((s) => ({ key: s.session_id, text: s.project })), more: Math.max(0, here.length - MAX_CHIPS) })

function Step({ flowId, step, sessions, agents, boardText }) {
  const names = list(step.agents).map((text) => ({ key: text, text }))
  const roles = list(signalsOf(step).agents).concat(list(step.agents))
  const seen = agents.filter((type) => isRole(roles, type)).map((type) => ({ key: type, text: `agent · ${type}` }))
  const chips = sessionChips(sessions)
  const parts = list(step.parts)
  const hasRows = names.length > 0 || chips.items.length > 0 || seen.length > 0 || boardText
  return (
    <FeedCard id={stepKey(flowId, step.id)} icon={step.id} title={step.name} className="workflow-step">
      {parts.length > 0 && <p className="muted workflow-parts">{parts.join(' · ')}</p>}
      {hasRows && (
        <dl className="workflow-rows">
          {names.length > 0 && <Chips label="Agents" items={names} />}
          {chips.items.length > 0 && <Chips label="Sessions" className="chip chip-ok" {...chips} />}
          {seen.length > 0 && <Chips label="Seen now" items={seen} className="chip chip-ok" />}
          {boardText && (
            <div>
              <dt>Board</dt>
              <dd className="count">{boardText}</dd>
            </div>
          )}
        </dl>
      )}
    </FeedCard>
  )
}

function Line({ flow, placed, agents, board }) {
  const steps = [...stepsOf(flow), ...(flow.id === KIT ? extraSteps(flow, placed) : [])]
  const gates = list(flow.gates)
  const unplaced = gates.filter((g) => !steps.some((s) => s.id === g.after))
  const showBoard = board && flow.id === KIT
  // The board's "waiting" count sits just before Stage 3: at the foot of the column before it (the first column when there is none).
  const waitAt = Math.max(0, steps.findIndex((s) => s.id === STAGE_3) - 1)
  const marker = (g) => (
    <p key={`${g.after}-${g.text}`} className="workflow-gate">{`Gate · ${g.text}`}</p>
  )
  return (
    <ol className="workflow-line" aria-label="Steps">
      {steps.map((step, i) => {
        const mine = placed.filter((p) => p.flow.id === flow.id && p.step?.id === step.id).map((p) => p.session).sort(bySession)
        const boardText = !showBoard ? null : step.id === STAGE_3 ? `In Progress ${board.progress}` : step.id === STAGE_4 ? `Ready for Review ${board.review} · Stage 4–5` : null
        const last = i === steps.length - 1
        return (
          <li key={step.id} className="workflow-step-item">
            <Step flowId={flow.id} step={step} sessions={mine} agents={agents} boardText={boardText} />
            {gates.filter((g) => g.after === step.id).map(marker)}
            {showBoard && i === waitAt && <p className="workflow-board count">{`${board.todo} waiting for Stage 3`}</p>}
            {last && unplaced.map(marker)}
            {showBoard && last && <p className="workflow-board count">{`${board.done} done`}</p>}
          </li>
        )
      })}
    </ol>
  )
}

/** The stage ids the stream sends that no step of easy-kit holds: one extra step each, at the end, never dropped. */
function extraSteps(flow, placed) {
  const known = new Set(stepsOf(flow).map((s) => s.id))
  const extra = new Map()
  for (const p of placed) if (p.flow.id === flow.id && p.step && !known.has(p.step.id)) extra.set(p.step.id, p.step)
  return [...extra.values()].sort((a, b) => a.id.localeCompare(b.id))
}

function Workflow({ map, board, live, now }) {
  const flows = useMemo(() => [...list(map.flows)].sort((a, b) => (b.id === KIT) - (a.id === KIT)), [map])
  const [chosen, setChosen] = useState(null)
  const flow = flows.find((f) => f.id === chosen) ?? flows[0]
  const sessions = useMemo(() => (live ? Object.values(live.state.sessions) : []), [live])
  const events = useMemo(() => eventsOf(sessions), [sessions])
  const agents = seenAgents(events, now)
  const placed = useMemo(
    () => sessions.map((session) => ({ session, ...placeSession(session, events, flows, map.projects) })).filter((p) => p.flow),
    [sessions, events, flows, map.projects],
  )
  if (!flow) return <p className="muted">The route sent no flow.</p>
  const unknown = placed.filter((p) => p.flow.id === flow.id && !p.step).map((p) => p.session).sort(bySession)
  const lost = sessionChips(unknown)
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
      <Line flow={flow} placed={placed} agents={agents} board={board} />
      {unknown.length > 0 && (
        <FeedCard id="workflow-step-unknown" icon="?" title="Step not known yet" className="workflow-step workflow-unknown">
          <dl className="workflow-rows">
            <Chips label="Sessions" className="chip chip-ok" {...lost} />
          </dl>
        </FeedCard>
      )}
    </>
  )
}

/** `// kit` › Workflow: a flow picker and the chosen flow (the kit's pipeline, plain prompts, or the owner's) as one horizontal line, with live sessions and sub-agents on their step and, on easy-kit, the board counts. Metadata only (NFR-002). */
export default function WorkflowView({ scope = NO_SCOPE }) {
  const map = useLoad(loadMap, [])
  const board = useLoad(() => loadBoard(scope), [scope.workspace, scope.account])
  const live = useContext(LiveContext) // null outside a LiveProvider: no live layer
  const [now, setNow] = useState(Date.now())
  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), TICK_MS)
    return () => clearInterval(timer)
  }, [])
  const counts = board.status === 'ready' ? countBoard(board.data) : null
  const boardLine =
    board.status === 'error'
      ? `Board counts are not available: ${board.message}`
      : counts && counts.total === 0
        ? 'No board tasks in this scope.'
        : null
  const liveLine = !live ? 'No live stream on this page: sessions and sub-agents are not placed.' : LINK[live.link]
  return (
    <main className="workflow">
      <h1>Workflow</h1>
      <p className="muted">Pick a flow: the kit&apos;s pipeline, plain prompts with no kit, or your own.</p>
      {liveLine && <p className="muted">{liveLine}</p>}
      {boardLine && <p className="muted">{boardLine}</p>}
      {map.status === 'loading' && <Loading label="Loading workflow" />}
      {map.status === 'error' && (
        <p role="alert" className="error">
          Could not load workflow: {map.message}
        </p>
      )}
      {map.status === 'ready' && <Workflow map={map.data} board={counts && counts.total > 0 ? counts : null} live={live} now={now} />}
    </main>
  )
}
