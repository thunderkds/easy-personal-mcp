import { useEffect, useMemo, useRef, useState } from 'react'
import { getAgents, getTranscript } from './api.js'
import Transcript from './Transcript.jsx'

// The conversation of a session with its sub-agents' logs (T125, FR-130): the sub-agents list, the breadcrumb, and one Transcript for the log that is open
// (the main thread or one sub-agent). The text is React text, never HTML, held in React state only (DDR-0007, DDR-0008).
export const AGENTS_POLL_MS = 5000 // the list is read this often while the panel is on the page, never faster (FR-130)
const SPAWNS = ['Agent', 'Task']
const SHORT = 8

const isName = (v) => typeof v === 'string' && v !== ''
const known = (list) => (Array.isArray(list) ? list.filter((a) => a && typeof a === 'object' && isName(a.id)) : [])
const typeOf = (a) => (isName(a.type) ? a.type : 'sub-agent')

/**
 * What the list can say about one sub-agent. A stream whose meta could not be read comes back as type "sub-agent", no call and no depth, with `finished`
 * false: that says nothing about it running, so it is "state unknown", never "running".
 */
export function stateOf(a) {
  if (a.finished === true) return 'finished'
  const metaMissing = !isName(a.parent_call) && !Number.isInteger(a.depth) && typeOf(a) === 'sub-agent'
  return metaMissing ? 'state unknown' : 'running'
}

/** The newest 200 sub-agents of the session, read now and every 5 s while this is on the page. `status`: loading | ready | absent (no such route or session) | error. */
function useAgents(sessionId) {
  const [state, setState] = useState({ status: 'loading', agents: [], total: 0 })
  useEffect(() => {
    let current = true
    let out = false
    let timer = null
    const tick = async () => {
      if (out) return
      out = true
      try {
        const body = await getAgents(sessionId)
        if (!current) return
        const agents = known(body?.agents)
        setState({ status: 'ready', agents, total: Number.isInteger(body?.total) && body.total >= agents.length ? body.total : agents.length })
      } catch (e) {
        if (!current) return
        if (e?.status === 404) {
          clearInterval(timer) // an older server or a session that is gone: asking again changes nothing
          setState({ status: 'absent', agents: [], total: 0 })
        } else setState((s) => (s.status === 'ready' ? s : { status: 'error', agents: [], total: 0 })) // the list shown stays when a refresh fails
      } finally {
        out = false
      }
    }
    tick()
    timer = setInterval(tick, AGENTS_POLL_MS)
    return () => {
      current = false
      clearInterval(timer)
    }
  }, [sessionId])
  return state
}

/** The ids of the `Agent` / `Task` calls in the newest page of one sub-agent's log; `null` when the log cannot be read now (not cached: asked again next time). */
async function callsIn(sessionId, agent) {
  try {
    const page = await getTranscript(sessionId, { agent })
    const entries = Array.isArray(page?.entries) ? page.entries : []
    return new Set(entries.filter((e) => e && e.kind === 'tool' && SPAWNS.includes(e.name) && isName(e.id)).map((e) => e.id))
  } catch {
    return null
  }
}

/**
 * Which sub-agent started which: `parent_call` is the id of the `Agent` call that started a stream, and that call is in the log of its parent (the main thread or
 * another sub-agent), which the list does not say. A stream of depth 1 is the main thread's. For a deeper one the logs of the sub-agents one level up are read
 * (newest page, each again only when it changed) until one holds the call. Not found: no owner, the stream is shown at the top level. Returns Map(child id -> parent id).
 */
function useOwners(sessionId, agents) {
  const [owners, setOwners] = useState(() => new Map())
  const found = useRef(new Map()) // child id -> owner id: a call never moves to another log
  const scanned = useRef(new Map()) // parent id -> {modified, calls}: the call ids in that log as last read; one entry per log, replaced when it changed
  useEffect(() => {
    let current = true
    // what was found is shown whether or not this run lives to its end: a later run skips the children already found
    const publish = () => setOwners((shown) => (shown.size === found.current.size ? shown : new Map(found.current)))
    const deep = agents.filter((a) => isName(a.parent_call) && Number.isInteger(a.depth) && a.depth >= 2 && !found.current.has(a.id))
    publish()
    if (deep.length === 0) return undefined
    ;(async () => {
      for (const child of deep) {
        for (const parent of agents) {
          if (parent.id === child.id || parent.depth !== child.depth - 1) continue
          let seen = scanned.current.get(parent.id)
          if (!seen || seen.modified !== parent.modified) {
            const calls = await callsIn(sessionId, parent.id)
            if (calls === null) continue // cannot be read now: not cached, tried again on the next read of the list
            seen = { modified: parent.modified, calls }
            scanned.current.set(parent.id, seen)
          }
          if (!current) return
          if (seen.calls.has(child.parent_call)) {
            found.current.set(child.id, parent.id)
            publish()
            break
          }
        }
      }
    })()
    return () => {
      current = false
    }
    // `agents` is a new array on every read of the list (every 5 s): a log that could not be read is tried again then; one already read is not
  }, [sessionId, agents])
  return owners
}

/** The list as a tree: a sub-agent sits under its owner when the owner is listed, else at the top level. */
function treeOf(agents, owners) {
  const listed = new Set(agents.map((a) => a.id))
  const kids = new Map()
  const roots = []
  for (const a of agents) {
    const owner = owners.get(a.id)
    if (owner && owner !== a.id && listed.has(owner)) kids.set(owner, [...(kids.get(owner) ?? []), a])
    else roots.push(a)
  }
  return { roots, kids }
}

function Node({ a, kids, onOpen }) {
  const children = kids.get(a.id) ?? []
  const type = typeOf(a)
  return (
    <li className="agent-node">
      <div className="agent-row">
        <span className="agent-type">{type}</span>
        <span className="live-tag" data-state={stateOf(a)}>
          {stateOf(a)}
        </span>
        <span className="live-tag mono">{a.id.slice(0, SHORT)}</span>
        <button type="button" aria-label={`Open log of ${type} ${a.id.slice(0, SHORT)}`} onClick={() => onOpen(a.id)}>
          Open log
        </button>
      </div>
      {children.length > 0 && (
        <ul className="agent-children">
          {children.map((c) => (
            <Node key={c.id} a={c} kids={kids} onOpen={onOpen} />
          ))}
        </ul>
      )}
    </li>
  )
}

/**
 * The conversation of one session: the breadcrumb `main › type › type`, the sub-agents list and the log that is open. `children` is the main thread's fallback
 * (Transcript's). Give it `key={sessionId}`: a new session starts at main (FR-130). The path is `[{id, type}]` from the main thread down.
 */
export default function Conversation({ sessionId, children }) {
  const [path, setPath] = useState([])
  const list = useAgents(sessionId)
  const owners = useOwners(sessionId, list.agents)
  const { roots, kids } = useMemo(() => treeOf(list.agents, owners), [list.agents, owners])
  const byId = useMemo(() => new Map(list.agents.map((a) => [a.id, a])), [list.agents])
  const logs = useMemo(() => new Map(list.agents.filter((a) => isName(a.parent_call)).map((a) => [a.parent_call, a.id])), [list.agents])
  const open = path.at(-1) ?? null
  const entry = (id) => ({ id, type: byId.has(id) ? typeOf(byId.get(id)) : 'sub-agent' })
  // from an Agent call in the log that is open: one level down from there
  const openFromLog = (id) => setPath((p) => (p.some((x) => x.id === id) ? p : [...p, entry(id)]))
  // from the list: the way down by the owners found, else the stream alone under main
  const openFromList = (id) => {
    const chain = [id]
    while (owners.has(chain[0]) && byId.has(owners.get(chain[0])) && !chain.includes(owners.get(chain[0]))) chain.unshift(owners.get(chain[0]))
    setPath(chain.map(entry))
  }
  const showTrail = list.status === 'ready' || path.length > 0
  const count = list.total
  return (
    <>
      {showTrail && (
        <nav aria-label="Log location" className="agent-trail">
          <ol>
            <li>
              {path.length === 0 ? (
                <span aria-current="location">main</span>
              ) : (
                <button type="button" onClick={() => setPath([])}>
                  main
                </button>
              )}
            </li>
            {path.map((p, i) => (
              <li key={p.id}>
                <span aria-hidden="true"> › </span>
                {i === path.length - 1 ? (
                  <span aria-current="location">{p.type}</span>
                ) : (
                  <button type="button" onClick={() => setPath(path.slice(0, i + 1))}>
                    {p.type}
                  </button>
                )}
              </li>
            ))}
          </ol>
        </nav>
      )}
      {list.status === 'ready' && (
        <section aria-label="Sub-agents" className="agent-list">
          <h3 className="agent-head">{`Sub-agents (${count})`}</h3>
          {list.agents.length === 0 ? (
            <p className="muted">No sub-agents in this session.</p>
          ) : (
            <>
              {count > list.agents.length && <p className="muted">{`Showing the newest ${list.agents.length} of ${count}.`}</p>}
              <ul className="agent-tree">
                {roots.map((a) => (
                  <Node key={a.id} a={a} kids={kids} onOpen={openFromList} />
                ))}
              </ul>
            </>
          )}
        </section>
      )}
      {list.status === 'error' && <p className="muted">The sub-agents cannot be listed right now.</p>}
      <Transcript key={open?.id ?? 'main'} sessionId={sessionId} agent={open?.id} logs={logs} onOpenLog={openFromLog}>
        {open ? null : children}
      </Transcript>
    </>
  )
}
