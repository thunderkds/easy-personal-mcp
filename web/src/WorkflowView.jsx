import { Fragment, useContext, useEffect, useMemo, useState } from 'react'
import { getJson } from './api.js'
import FeedCard from './FeedCard.jsx'
import { LiveContext } from './LiveView.jsx'
import Loading from './Loading.jsx'
import { NO_SCOPE, scopedPath } from './Scope.jsx'
import { useLoad } from './useLoad.js'

const AGENT_WINDOW_MS = 5 * 60_000 // a sub-agent seen this recently counts as working (FR-118)
const TICK_MS = 15_000
const MAX_CHIPS = 5 // live chips per lane; the rest is "+N more"
const NOT_A_TYPE = new Set(['main', 'sub-agent']) // what the Live feed says when a call has no sub-agent type
const NOISE = new Set(['agent', 'developer', 'implementer', 'expert', 'automation'])
const STAGE_3 = '3' // the lane where Todo tasks wait and In Progress tasks sit (FR-118)
const STAGE_4 = '4' // the lane for Ready for Review: reviews (4) and verify (5)

// The map is the route's (FR-117); only where a board status lands is the page's own rule.
const loadMap = () => getJson('/api/kit/workflow')
const loadBoard = (scope) => getJson(scopedPath('/api/board', scope))

/** A role without its suffix and case: `Backend-Implementer`, `backend-developer` and `backend` are one role. */
function role(name) {
  const words = String(name).toLowerCase().split(/[^a-z0-9]+/).filter(Boolean)
  const kept = words.filter((w) => !NOISE.has(w))
  return (kept.length > 0 ? kept : words).join('-')
}

const laneKey = (id) => `workflow-lane-${String(id).replace(/\W+/g, '-')}`

/** Sub-agent types the feeds show in the last 5 minutes. A feed row carries `who`: the type, "sub-agent" or "main" (LiveView). */
function seenAgents(sessions, now) {
  const types = new Set()
  for (const s of sessions) {
    for (const item of s.feed) {
      if (typeof item.who === 'string' && !NOT_A_TYPE.has(item.who) && now - Date.parse(item.ts) <= AGENT_WINDOW_MS) types.add(item.who)
    }
  }
  return [...types].sort()
}

/** The route's lanes, then one lane for each stage id the stream sends that the route lacks. Each lane carries its live chips. */
function buildLanes(stages, sessions, agents) {
  const known = new Set(stages.map((s) => s.id))
  const extra = [...new Set(sessions.filter((s) => s.stage !== null && s.stage !== undefined).map((s) => String(s.stage)))].filter((id) => !known.has(id)).sort()
  const all = [...stages, ...extra.map((id) => ({ id, name: `Stage ${id}`, skills: [], agents: [] }))]
  const bySession = (a, b) => a.project.localeCompare(b.project) || a.session_id.localeCompare(b.session_id)
  return all.map((lane) => {
    const here = sessions.filter((s) => String(s.stage) === lane.id).sort(bySession)
    const roles = new Set((lane.agents ?? []).map(role))
    const live = [
      ...here.map((s) => ({ key: `s-${s.session_id}`, text: s.stage_skill ? `${s.project} · ${s.stage_skill}` : s.project })),
      ...agents.filter((type) => roles.has(role(type))).map((type) => ({ key: `a-${type}`, text: `agent · ${type}` })),
    ]
    return { ...lane, skills: lane.skills ?? [], agents: lane.agents ?? [], live }
  })
}

/** Board tasks of the answer by the four statuses the lanes use; `total` counts every task, whatever its status. */
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

const names = (list) => list.map((text) => ({ key: text, text }))

function Lane({ lane, board }) {
  const shown = lane.live.slice(0, MAX_CHIPS)
  const boardText =
    board && lane.id === STAGE_3 ? `In Progress ${board.progress}` : board && lane.id === STAGE_4 ? `Ready for Review ${board.review} · Stage 4–5` : null
  const hasRows = lane.skills.length > 0 || lane.agents.length > 0 || lane.live.length > 0 || boardText
  return (
    <FeedCard id={laneKey(lane.id)} icon={lane.id} title={lane.name} className="workflow-lane">
      {hasRows && (
        <dl className="workflow-rows">
          {lane.skills.length > 0 && <Chips label="Skills" items={names(lane.skills)} />}
          {lane.agents.length > 0 && <Chips label="Agents" items={names(lane.agents)} />}
          {lane.live.length > 0 && <Chips label="Live now" items={shown} className="chip chip-ok" more={lane.live.length - shown.length} />}
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

function Workflow({ map, board, live, now }) {
  const sessions = useMemo(() => (live ? Object.values(live.state.sessions) : []), [live])
  const agents = seenAgents(sessions, now)
  const lanes = buildLanes(map.stages ?? [], sessions, agents)
  const gates = map.gates ?? []
  const unplaced = gates.filter((g) => !lanes.some((l) => l.id === g.after))
  const unknown = sessions.filter((s) => s.stage === null || s.stage === undefined)
  const waitBefore = lanes.some((l) => l.id === STAGE_3) ? STAGE_3 : lanes[0]?.id
  return (
    <>
      <h2>Pipeline</h2>
      <ol className="workflow-lanes" aria-label="Pipeline">
        {lanes.map((lane) => (
          <Fragment key={lane.id}>
            {board && lane.id === waitBefore && <li className="workflow-board count">{`${board.todo} waiting for Stage 3`}</li>}
            <li>
              <Lane lane={lane} board={board} />
            </li>
            {gates
              .filter((g) => g.after === lane.id)
              .map((g) => (
                <li key={`${g.after}-${g.text}`} className="workflow-gate">{`Gate · ${g.text}`}</li>
              ))}
          </Fragment>
        ))}
        {unplaced.map((g) => (
          <li key={`${g.after}-${g.text}`} className="workflow-gate">{`Gate · ${g.text}`}</li>
        ))}
        {board && <li className="workflow-board count">{`${board.done} done`}</li>}
        {unknown.length > 0 && (
          <li>
            <FeedCard id="workflow-lane-unknown" icon="?" title="Stage not known yet" className="workflow-lane">
              <dl className="workflow-rows">
                <Chips label="Live now" items={unknown.slice(0, MAX_CHIPS).map((s) => ({ key: s.session_id, text: s.project }))} className="chip chip-ok" more={Math.max(0, unknown.length - MAX_CHIPS)} />
              </dl>
            </FeedCard>
          </li>
        )}
      </ol>
    </>
  )
}

/** `// kit` › Workflow: the kit's fixed pipeline (the route's map), with live sessions and sub-agents on their lanes and the board counts. Metadata only (NFR-002). */
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
      <p className="muted">The kit&apos;s fixed pipeline, as the kit&apos;s own table sends it.</p>
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
