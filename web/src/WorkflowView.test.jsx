import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { LiveProvider } from './LiveView.jsx'
import WorkflowView, { columnsFor, placeSession, takenText, workingAgents } from './WorkflowView.jsx'
import { fakeFetch, workflow } from './test/fixtures.js'

// A hand-driven EventSource: the test decides what the "server" sends (the page's one /api/live stream).
class FakeEventSource {
  static instances = []
  constructor(url) {
    this.url = url
    this.readyState = 0
    this.listeners = {}
    FakeEventSource.instances.push(this)
  }
  addEventListener(type, fn) {
    ;(this.listeners[type] ??= []).push(fn)
  }
  close() {
    this.readyState = 2
  }
  emit(type, data) {
    const event = { type, data: data === undefined ? undefined : JSON.stringify(data) }
    act(() => (this.listeners[type] ?? []).forEach((fn) => fn(event)))
  }
}
const stream = () => FakeEventSource.instances.at(-1)

const session = (id, over = {}) => ({
  session_id: id, account: 'Claude work', project: 'alpha', last_ts: new Date().toISOString(),
  tool_calls: 0, tokens_in: 0, tokens_out: 0, cost_usd: 0, stage: null, stage_skill: null, ...over,
})
const snapshot = (...sessions) => stream().emit('snapshot', { active_window_s: 300, last_id: 0, sessions })
let counter = 0
const ago = (ms) => new Date(Date.now() - ms).toISOString()
const call = (sessionId, over = {}) => ({
  id: ++counter, session_id: sessionId, account: 'Claude work', project: 'alpha', ts: new Date().toISOString(),
  kind: 'tool_use', tool: 'Agent', is_error: false, is_sidechain: true, tokens_in: null, tokens_out: null, cost_usd: null, ...over,
})
// Events reach the page in batches of at most 250 ms and in order. Two last calls, each from a session of its own, show on whichever flow is chosen when the batch before them has been drawn:
// "sentinelkit" is assigned to easy-kit (no stage) and "sentinelplain" goes to plain with a call no step matches: each is listed under the line as "step not known yet".
const SENTINEL = /^step not known yet: .*sentinel(kit|plain)/
const send = async (...events) => {
  const last = [call('sk', { project: 'sentinelkit', tool: 'Zzz', is_sidechain: false }), call('sp', { project: 'sentinelplain', tool: 'Zzz', is_sidechain: false })]
  for (const e of [...events, ...last]) stream().emit('live', e)
  await screen.findByText(SENTINEL)
}
const own = (id, over = {}) => call(id, { tool: 'Edit', is_sidechain: false, ...over }) // a main-thread call


async function renderView({ live = true, ...overrides } = {}) {
  const base = overrides.workflow ?? workflow
  const f = fakeFetch({ ...overrides, workflow: { ...base, projects: { sentinelkit: 'easy-kit', ...base.projects } } })
  vi.stubGlobal('fetch', f)
  render(live ? <LiveProvider><WorkflowView /></LiveProvider> : <WorkflowView />)
  await screen.findByRole('heading', { name: 'Workflow' })
  if (live) {
    stream().readyState = 1
    stream().emit('open')
  }
  return f
}
const [KIT, PLAIN] = workflow.flows
const line = () => screen.getByRole('list', { name: 'Steps' })
const graph = () => document.querySelector('.workflow-graph')
const at = (li) => ({ left: parseFloat(li.style.left), top: parseFloat(li.style.top), width: parseFloat(li.style.width), height: parseFloat(li.style.height) })
// Every drawn card in flow order: the start card, the step cards (the Steps list), the end marker. A gate sits inside its source step's card, so its own offsets are relative to it.
const cards = () => [...graph().querySelectorAll('.workflow-start, .workflow-line > li, .workflow-end')]
const absolute = (el) => {
  const b = at(el)
  const host = el.closest('li')
  return host && host !== el ? { ...b, left: b.left + parseFloat(host.style.left), top: b.top + parseFloat(host.style.top) } : b
}
const gate = (text) => within(graph()).getByRole('button', { name: `Gate · ${text}` })
const nodes = () => [...line().querySelectorAll('.workflow-node')]
const nodeNames = () => nodes().map((n) => n.querySelector('.workflow-title')?.textContent ?? n.textContent)
const node = (name) => within(line()).getByRole('button', { name: new RegExp(`^${name}( ·|$)`) })
const pick = (name) => fireEvent.click(screen.getByRole('button', { name }))
const panel = () => screen.queryByRole('region')

beforeEach(() => {
  window.location.hash = '#secret-token'
  FakeEventSource.instances = []
  vi.stubGlobal('EventSource', FakeEventSource)
})
afterEach(() => {
  vi.unstubAllGlobals()
})

describe('placeSession: which flow and which step', () => {
  const flows = workflow.flows
  const s = (over = {}) => ({ session_id: 's1', project: 'alpha', stage: null, ...over })
  const ev = (over = {}) => ({ id: 1, session_id: 's1', tool: 'Read', ts: '2026-11-17T10:00:00Z', ...over })
  const place = (session, events = [], projects = {}) => {
    const r = placeSession(session, events, flows, projects)
    return { flow: r.flow.id, step: r.step ? r.step.id : null }
  }

  it('puts a session with a known stage on easy-kit, on the step whose signals hold that stage', () => {
    expect(place(s({ stage: '1.5' }))).toEqual({ flow: 'easy-kit', step: '1' })
    expect(place(s({ stage: '0.5' }))).toEqual({ flow: 'easy-kit', step: '0' })
    expect(place(s({ stage: '5' }))).toEqual({ flow: 'easy-kit', step: '5' })
  })

  it('gives a stage id no step holds its own extra step, not null', () => {
    const r = placeSession(s({ stage: '9' }), [], flows, {})
    expect(r.flow.id).toBe('easy-kit')
    expect(r.step).toMatchObject({ id: '9', name: 'Stage 9' })
  })

  it('puts a session with no stage on plain, by its newest matching event', () => {
    expect(place(s(), [ev({ tool: 'Edit' })])).toEqual({ flow: 'plain', step: 'change' })
    expect(place(s(), [ev({ tool: 'Read' })])).toEqual({ flow: 'plain', step: 'understand' })
    expect(place(s(), [ev({ tool: 'TodoWrite' })])).toEqual({ flow: 'plain', step: 'plan' })
    expect(place(s(), [ev({ tool: 'Bash', label: 'cargo' })])).toEqual({ flow: 'plain', step: 'check' })
    expect(place(s(), [ev({ tool: 'Bash', label: 'git' })])).toEqual({ flow: 'plain', step: 'ship' })
    expect(place(s(), [ev({ tool: 'Agent', agent: 'Explore' })])).toEqual({ flow: 'plain', step: 'understand' })
  })

  it('takes the newest event, not the first, and skips events that match nothing', () => {
    const older = ev({ id: 1, tool: 'Read', ts: '2026-11-17T10:00:00Z' })
    const newer = ev({ id: 2, tool: 'Edit', ts: '2026-11-17T10:05:00Z' })
    expect(place(s(), [newer, older]).step).toBe('change')
    expect(place(s(), [older, newer]).step).toBe('change')
    const noise = ev({ id: 3, tool: 'Bash', label: 'ls', ts: '2026-11-17T10:09:00Z' })
    expect(place(s(), [older, newer, noise]).step).toBe('change')
  })

  it('knows no step for no event, an unlabelled Bash call, an unknown command or another session', () => {
    expect(place(s())).toEqual({ flow: 'plain', step: null })
    expect(place(s(), [ev({ tool: 'Bash' })]).step).toBeNull()
    expect(place(s(), [ev({ tool: 'Bash', label: 'ls' })]).step).toBeNull()
    expect(place(s(), [ev({ session_id: 'other', tool: 'Edit' })]).step).toBeNull()
    expect(place(s(), [ev({ tool: 'Mystery' })]).step).toBeNull()
  })

  it('does not take a command label for a tool name or a tool name for a command', () => {
    expect(place(s(), [ev({ tool: 'Read', label: 'cargo' })]).step).toBe('understand')
    expect(place(s(), [ev({ tool: 'Edit', label: 'cargo' })]).step).toBe('change')
    expect(place(s(), [ev({ tool: 'cargo' })]).step).toBeNull()
  })

  it('sends a session to the flow its project is assigned to, and falls back when that flow is missing', () => {
    expect(place(s({ stage: '3' }), [ev({ tool: 'Edit' })], { alpha: 'plain' })).toEqual({ flow: 'plain', step: 'change' })
    expect(place(s({ stage: '3' }), [], { alpha: 'nope' })).toEqual({ flow: 'easy-kit', step: '3' })
    expect(place(s(), [ev({ tool: 'Edit' })], { alpha: 'nope' })).toEqual({ flow: 'plain', step: 'change' })
    expect(place(s(), [], { alpha: 'easy-kit' })).toEqual({ flow: 'easy-kit', step: null })
  })

  it('places on an owner flow by tool, command, sub-agent type or skill signal', () => {
    const mine = {
      id: 'mine',
      name: 'Mine',
      steps: [
        { id: 'a', name: 'A', parts: [], agents: [], signals: { stages: [], skills: ['my-skill'], agents: ['Reviewer'], tools: [], commands: [] } },
        { id: 'b', name: 'B', parts: [], agents: [], signals: { stages: [], skills: [], agents: [], tools: ['Grep'], commands: ['just'] } },
      ],
      gates: [],
    }
    const all = [...flows, mine]
    const at = (e) => placeSession(s(), [ev(e)], all, { alpha: 'mine' }).step?.id ?? null
    expect(at({ tool: 'Skill', label: 'my-skill' })).toBe('a')
    expect(at({ tool: 'Read', label: 'my-skill' })).toBeNull()
    expect(at({ tool: 'Agent', agent: 'Reviewer' })).toBe('a')
    expect(at({ tool: 'Grep' })).toBe('b')
    expect(at({ tool: 'Bash', label: 'just' })).toBe('b')
  })

  it('tolerates a route whose steps lack some signal keys', () => {
    const odd = { id: 'odd', name: 'Odd', steps: [{ id: 'x', name: 'X', signals: { tools: ['Edit'] } }], gates: [] }
    expect(placeSession(s(), [ev({ tool: 'Edit' })], [odd], { alpha: 'odd' }).step.id).toBe('x')
  })
})

describe('workingAgents (AC5, AC6): which agent works on which step, and for how long', () => {
  const NOW = Date.parse('2026-11-17T10:10:00Z')
  const at = (min) => new Date(NOW - min * 60_000).toISOString()
  let n = 0
  const e = (over = {}) => ({ id: ++n, session_id: 's1', tool: 'Edit', ts: at(1), sidechain: false, ...over })
  const sess = (over = {}) => ({ session_id: 's1', project: 'alpha', stage: null, ...over })
  const rows = (sessions, events, projects = {}) =>
    workingAgents(sessions, events, workflow.flows, projects, NOW)
      .map((r) => [r.project, r.agent, r.flow.id, r.step ? r.step.id : null])
      .sort()

  it('puts a session on easy-kit and its sub-agent on the same step: the one of the stage', () => {
    const got = rows([sess({ stage: '1.5' })], [e({ tool: 'Read' }), e({ tool: 'Agent', agent: 'backend-developer', sidechain: true })])
    expect(got).toEqual([
      ['alpha', 'backend-developer', 'easy-kit', '1'],
      ['alpha', 'main session', 'easy-kit', '1'],
    ])
  })

  it('keeps an easy-kit sub-agent on the stage step, whatever tool it calls', () => {
    expect(rows([sess({ stage: '3' })], [e({ tool: 'Edit', agent: 'frontend-developer', sidechain: true })])).toEqual([['alpha', 'frontend-developer', 'easy-kit', '3']])
  })

  it('puts a plain agent on the step of its own newest call, and the main thread on the step of its own', () => {
    const got = rows(
      [sess()],
      [e({ tool: 'Edit', ts: at(2) }), e({ tool: 'Grep', agent: 'Explore', sidechain: true, ts: at(1) })],
    )
    expect(got).toEqual([
      ['alpha', 'Explore', 'plain', 'understand'],
      ['alpha', 'main session', 'plain', 'change'],
    ])
  })

  it('falls back to the session step for an agent whose calls match no step, and to no step when the session has none', () => {
    expect(rows([sess()], [e({ tool: 'Edit', ts: at(2) }), e({ tool: 'Mystery', agent: 'Helper', sidechain: true })])).toEqual([
      ['alpha', 'Helper', 'plain', 'change'],
      ['alpha', 'main session', 'plain', 'change'],
    ])
    expect(rows([sess()], [e({ tool: 'Mystery' })])).toEqual([['alpha', 'main session', 'plain', null]])
  })

  it('names a sidechain call with no type "sub-agent"', () => {
    expect(rows([sess({ stage: '3' })], [e({ sidechain: true })])).toEqual([['alpha', 'sub-agent', 'easy-kit', '3']])
  })

  it('counts an agent as working for a call up to 5 minutes old, not older', () => {
    expect(rows([sess({ stage: '3' })], [e({ ts: at(5) })])).toHaveLength(1)
    expect(rows([sess({ stage: '3' })], [e({ ts: at(5.1) })])).toEqual([])
    // a newer call of the same agent makes it work again; an old one of another agent does not count
    expect(rows([sess({ stage: '3' })], [e({ ts: at(30) }), e({ ts: at(1) }), e({ tool: 'Agent', agent: 'qa-expert', sidechain: true, ts: at(9) })])).toEqual([['alpha', 'main session', 'easy-kit', '3']])
  })

  it('says nothing for a session with no call seen', () => {
    expect(rows([sess({ stage: '3' })], [])).toEqual([])
    expect(rows([sess({ stage: '3' })], [e({ session_id: 'other' })])).toEqual([])
  })

  it('sends a session to the flow its project is assigned to', () => {
    expect(rows([sess({ stage: '3' })], [e({ tool: 'Bash', label: 'cargo' })], { alpha: 'plain' })).toEqual([['alpha', 'main session', 'plain', 'check']])
  })

  it('gives an unknown stage id its own step', () => {
    expect(rows([sess({ stage: '9' })], [e()])).toEqual([['alpha', 'main session', 'easy-kit', '9']])
  })

  it('measures from the first call at the node to the newest call', () => {
    const [r] = workingAgents([sess({ stage: '3' })], [e({ ts: at(9) }), e({ ts: at(4) }), e({ ts: at(2) })], workflow.flows, {}, NOW)
    expect(Date.parse(r.last) - Date.parse(r.first)).toBe(7 * 60_000)
    // on plain only the calls that match the step count: the earlier Read is another step
    const [p] = workingAgents([sess()], [e({ tool: 'Read', ts: at(9) }), e({ tool: 'Edit', ts: at(4) }), e({ tool: 'Edit', ts: at(2) })], workflow.flows, {}, NOW)
    expect(p.step.id).toBe('change')
    expect(Date.parse(p.last) - Date.parse(p.first)).toBe(2 * 60_000)
  })
})

describe('takenText', () => {
  const t = (ms) => takenText('2026-11-17T10:00:00Z', new Date(Date.parse('2026-11-17T10:00:00Z') + ms).toISOString())
  it('reads like the page\'s other durations: seconds under a minute, then 12m, then 1h 5m', () => {
    expect(t(0)).toBe('0s')
    expect(t(45_000)).toBe('45s')
    expect(t(12 * 60_000)).toBe('12m')
    expect(t(65 * 60_000)).toBe('1h 5m')
  })
})

describe('the flow picker and the line (AC4)', () => {
  it('has one button per flow, easy-kit first and pressed, the others not', async () => {
    await renderView()
    await screen.findByText('Stage 3 · Execute')
    const buttons = within(screen.getByRole('group', { name: 'Flow' })).getAllByRole('button')
    expect(buttons.map((b) => b.textContent)).toEqual(['Easy kit', 'Plain prompts (no kit)'])
    expect(buttons.map((b) => b.getAttribute('aria-pressed'))).toEqual(['true', 'false'])
    pick('Plain prompts (no kit)')
    expect(buttons.map((b) => b.getAttribute('aria-pressed'))).toEqual(['false', 'true'])
  })

  it('draws easy-kit as one row of six nodes in the route order, each showing only its step name', async () => {
    await renderView()
    await screen.findByText('Stage 3 · Execute')
    expect(nodeNames()).toEqual(KIT.steps.map((x) => x.name))
    expect(nodes().every((n) => n.tagName === 'BUTTON')).toBe(true)
    expect(nodes().every((n) => n.closest('.workflow-line') === line())).toBe(true)
  })

  it('shows no parts, agent list, skill or board count on the line, and reads no board', async () => {
    const f = await renderView({ board: { projects: [{ project: 'p', status: 'ok', board: 'linear', last_pull: null, abstained: 0, tasks: [{ id: 'T1', title: 'x', status: 'Todo' }] }] } })
    await screen.findByText('Stage 3 · Execute')
    snapshot(session('s1', { stage: '3', stage_skill: 'tdd' }))
    for (const flow of [KIT.name, PLAIN.name]) {
      pick(flow)
      expect(nodes().map((n) => n.textContent).join(' ')).not.toMatch(/Requirements|Grilling|Environment|Agent team|Supervisor|qa-expert|common-infrastructure|Explore|waiting for|In Progress|Ready for Review|done$/)
      expect(document.body.textContent).not.toMatch(/brainstorming|ideate|map-codebase|craft-agent|grill-with-docs|\btdd\b|bugfix|code-review|delivery-report|migration-safety/)
    }
    expect(f.calls.some((c) => c.url.startsWith('/api/board'))).toBe(false)
  })

  it('draws the plain flow with its own nodes when picked', async () => {
    await renderView()
    await screen.findByText('Stage 3 · Execute')
    pick('Plain prompts (no kit)')
    expect(nodeNames()).toEqual(['Understand', 'Plan', 'Change', 'Check', 'Ship'])
  })

  it('follows the route: another flow list, an extra field and a gate whose step is unknown', async () => {
    const odd = { flows: [{ id: 'odd', name: 'Odd flow', future: 1, steps: [{ id: 'b', name: 'Build', signals: {} }, { id: 'a', name: 'Ask', parts: ['p'], agents: ['Supervisor'], signals: {} }], gates: [{ after: '9', before: null, text: 'Lost gate' }] }], projects: {}, future: true }
    await renderView({ workflow: odd })
    expect(await screen.findByText('Build')).toBeInTheDocument()
    expect(nodeNames()).toEqual(['Build', 'Ask'])
    expect(screen.getByRole('button', { name: /Lost gate/ })).toBeInTheDocument() // a gate whose step is unknown is kept, at the end
  })

  it('shows an error line, not a blank page, when the route fails', async () => {
    vi.stubGlobal('fetch', vi.fn(async () => ({ ok: false, status: 500, json: async () => ({}) })))
    render(<WorkflowView />)
    expect(await screen.findByRole('alert')).toHaveTextContent('Could not load workflow')
    expect(screen.getByRole('heading', { name: 'Workflow' })).toBeInTheDocument()
  })
})

describe('the drawing (owner reference)', () => {
  const edge = (to) => document.querySelector(`.workflow-edge[data-to="${to}"]`)
  const mid = (b) => b.top + b.height / 2
  const nums = (el) => el.getAttribute('d').match(/-?\d+(?:\.\d+)?/g).map(Number)
  // Every edge leaves the right port of its source and enters the left port of its target; one that changes row goes down through the gap between the two rows. Returns how many changed row.
  const expectPortToPort = () => {
    const boxes = cards().map(at)
    const paths = [...document.querySelectorAll('.workflow-edge')]
    expect(paths).toHaveLength(boxes.length - 1)
    let down = 0
    paths.forEach((path, i) => {
      const [a, b] = [boxes[i], boxes[i + 1]]
      const n = nums(path)
      expect(n.slice(0, 2)).toEqual([a.left + a.width, a.top + a.height / 2])
      expect(n.slice(-2)).toEqual([b.left, b.top + b.height / 2])
      if (mid(b) !== mid(a)) {
        down += 1
        expect(n[7]).toBeGreaterThan(a.top + a.height) // the turn is below the source card ...
        expect(n[7]).toBeLessThan(b.top) // ... and above the target card: it never crosses one
      }
    })
    return down
  }

  it('draws the start card with the count of live sessions placed on the flow, or "no live stream"', async () => {
    await renderView()
    await screen.findByText('Stage 3 · Execute')
    const start = graph().querySelector('.workflow-start')
    expect(start).toHaveTextContent('Live sessions')
    expect(start).toHaveTextContent('0 live')
    snapshot(session('s1', { stage: '3', stage_skill: 'tdd' }), session('s2', { stage: '1' }), session('s3'))
    expect(graph().querySelector('.workflow-start')).toHaveTextContent('2 live') // s3 has no stage: it is on plain
    pick('Plain prompts (no kit)')
    expect(graph().querySelector('.workflow-start')).toHaveTextContent('1 live')
  })

  it('says "no live stream" on the start card outside a stream provider', async () => {
    await renderView({ live: false })
    await screen.findByText('Stage 3 · Execute')
    expect(graph().querySelector('.workflow-start')).toHaveTextContent('no live stream')
  })

  it('gives the edge into a working node the active class and the edge into an idle node the idle class', async () => {
    await renderView()
    await screen.findByText('Stage 3 · Execute')
    expect(document.querySelectorAll('.workflow-edge.active')).toHaveLength(0)
    snapshot(session('s1', { stage: '3', stage_skill: 'tdd' }))
    await send(own('s1'))
    expect(edge('node:3')).toHaveClass('workflow-edge', 'active')
    expect(edge('node:2')).toHaveClass('workflow-edge', 'idle')
    expect(edge('node:2')).not.toHaveClass('active')
    expect(node('Stage 3 · Execute').closest('li')).toHaveTextContent('1 working')
    expect(node('Stage 2 · Plan').closest('li')).toHaveTextContent('idle')
  })

  it('draws one edge between neighbours, hidden from assistive tech, and the start edge active once any agent works on the flow', async () => {
    await renderView()
    await screen.findByText('Stage 3 · Execute')
    const edges = () => [...document.querySelectorAll('.workflow-edge')]
    expect(edges()).toHaveLength(cards().length - 1)
    expect(edges().every((p) => p.closest('svg').getAttribute('aria-hidden') === 'true')).toBe(true)
    expect(edge('node:0')).toHaveClass('idle')
    snapshot(session('s1', { stage: '3', stage_skill: 'tdd' }))
    await send(own('s1'))
    expect(edge('node:0')).toHaveClass('active')
  })

  it('keeps the start edge idle while the only working agent has no step yet', async () => {
    await renderView()
    await screen.findByText('Stage 3 · Execute')
    snapshot()
    await send() // the two sentinel sessions work on easy-kit and plain with no step
    await screen.findByText(/step not known yet/)
    expect(edge('node:0')).toHaveClass('idle')
    expect(document.querySelectorAll('.workflow-edge.active')).toHaveLength(0)
    expect(document.querySelectorAll('.workflow-node.working')).toHaveLength(0)
  })

  it('lists only the steps in the Steps list: the start card and the end marker sit outside it', async () => {
    await renderView()
    await screen.findByText('Stage 3 · Execute')
    expect(line().querySelectorAll(':scope > li')).toHaveLength(KIT.steps.length)
    expect(line().querySelector('.workflow-start, .workflow-end')).toBeNull()
    expect(graph().querySelector('.workflow-start')).not.toBeNull()
    expect(graph().querySelector('.workflow-end')).not.toBeNull()
    expect(within(line()).getAllByRole('listitem')).toHaveLength(KIT.steps.length)
  })

  it('takes the state of its edge: a gate on the edge into a working node is accent, the others idle', async () => {
    await renderView()
    await screen.findByText('Stage 3 · Execute')
    expect(gate('No TASK_GUIDE, no Stage 3')).toHaveClass('idle')
    snapshot(session('s1', { stage: '3', stage_skill: 'tdd' }))
    await send(own('s1'))
    expect(gate('No TASK_GUIDE, no Stage 3')).toHaveClass('active')
    expect(gate('Reviews before verify')).toHaveClass('idle')
  })

  it('draws every edge from the right port of its source to the left port of its target, a same-row and a next-row one alike', async () => {
    await renderView()
    await screen.findByText('Stage 3 · Execute')
    const boxes = cards().map(at)
    expect(mid(boxes[1])).toBe(mid(boxes[0])) // start and Stage 0 share a row ...
    expect(mid(boxes[2])).toBeGreaterThan(mid(boxes[1])) // ... Stage 1 is on the next one
    expect(expectPortToPort()).toBeGreaterThan(0)
  })

  it('wraps the row to the canvas width: 700 px gives two columns, the next row starts under the first', async () => {
    expect(columnsFor(700)).toBe(2)
    expect(columnsFor(339)).toBe(1)
    expect(columnsFor(0)).toBe(1)
    expect(columnsFor(1280)).toBe(4)
    await renderView()
    await screen.findByText('Stage 3 · Execute')
    const boxes = cards().map(at)
    const lefts = [...new Set(boxes.map((b) => b.left))]
    expect(lefts).toHaveLength(2)
    expect(boxes[2].left).toBe(boxes[0].left)
    expect(boxes[2].top).toBeGreaterThan(boxes[0].top)
    expect(parseFloat(graph().style.height)).toBeGreaterThanOrEqual(boxes.at(-1).top + boxes.at(-1).height)
    expect(parseFloat(graph().style.width)).toBeLessThanOrEqual(700)
  })

  it('puts every node in one column on a narrow canvas', async () => {
    const was = Object.getOwnPropertyDescriptor(HTMLElement.prototype, 'clientWidth')
    Object.defineProperty(HTMLElement.prototype, 'clientWidth', { configurable: true, get() { return this.classList?.contains('workflow-canvas') ? 339 : 0 } })
    try {
      await renderView()
      await screen.findByText('Stage 3 · Execute')
      const boxes = cards().map(at)
      expect(new Set(boxes.map((b) => b.left)).size).toBe(1)
      expect(boxes.every((b, i) => i === 0 || b.top > boxes[i - 1].top)).toBe(true)
      expect(expectPortToPort()).toBe(boxes.length - 1) // one column: every edge changes row
    } finally {
      if (was) Object.defineProperty(HTMLElement.prototype, 'clientWidth', was)
      else delete HTMLElement.prototype.clientWidth
    }
  })

  it('shortens a gate to a pill and keeps its full text for assistive tech and the panel', async () => {
    await renderView()
    await screen.findByText('Stage 3 · Execute')
    const pill = within(graph()).getByRole('button', { name: 'Gate · Ready for Review starts the reviews' })
    expect(pill.textContent.length).toBeLessThan('Ready for Review starts the reviews'.length)
    expect(pill.textContent).toMatch(/^Ready for Review/) // the first words, not "Ready…"
    expect(gate('No TASK_GUIDE, no Stage 3')).toHaveTextContent('No TASK_GUIDE, no Stage 3') // a short gate shows whole
    expect(gate('Reviews before verify')).toHaveTextContent('Reviews before verify')
    fireEvent.click(pill)
    expect(panel()).toHaveTextContent('Ready for Review starts the reviews')
  })
})

describe('gates (AC7)', () => {
  it('puts each gate as a focusable label on the edge after its step, right after that step in the DOM, the last one on a short edge after Stage 5', async () => {
    await renderView()
    await screen.findByText('Stage 3 · Execute')
    const marks = [...graph().querySelectorAll('.workflow-gate')]
    expect(marks.map((m) => m.getAttribute('aria-label'))).toEqual(KIT.gates.map((g) => `Gate · ${g.text}`))
    expect(marks.every((m) => m.tagName === 'BUTTON')).toBe(true)
    // Tab order follows the flow: a gate comes straight after its source step and before the next step.
    const order = [...graph().querySelectorAll('.workflow-node, .workflow-gate')].map((el) => el.getAttribute('aria-label'))
    expect(order).toEqual([
      'Stage 0 · Define', 'Stage 1 · Set up', 'Stage 2 · Plan', 'Gate · No TASK_GUIDE, no Stage 3', 'Stage 3 · Execute', 'Gate · Ready for Review starts the reviews',
      'Stage 4 · Review', 'Gate · Reviews before verify', 'Stage 5 · Integrate', 'Gate · Verify passes before merge',
    ])
    // The label sits on the middle of the edge between the two cards it sits between.
    const a = at(node('Stage 2 · Plan').closest('li'))
    const b = at(node('Stage 3 · Execute').closest('li'))
    const m = absolute(gate('No TASK_GUIDE, no Stage 3'))
    expect(m.left + m.width / 2).toBeCloseTo((a.left + a.width + b.left) / 2)
    expect(m.top + m.height / 2).toBeCloseTo((a.top + a.height / 2 + b.top + b.height / 2) / 2)
    const last = at(node('Stage 5 · Integrate').closest('li'))
    expect(absolute(gate('Verify passes before merge')).left).toBeGreaterThanOrEqual(last.left) // on the trailing edge, at the end
  })

  it('shows the gate text on hover, on focus and on a tap, and closes it on Escape', async () => {
    await renderView()
    await screen.findByText('Stage 3 · Execute')
    expect(panel()).toBeNull()
    fireEvent.mouseEnter(gate('Reviews before verify'))
    expect(panel()).toHaveTextContent('Reviews before verify')
    fireEvent.mouseLeave(gate('Reviews before verify'))
    expect(panel()).toBeNull()
    act(() => gate('Reviews before verify').focus())
    expect(panel()).toHaveTextContent('Reviews before verify')
    fireEvent.keyDown(gate('Reviews before verify'), { key: 'Escape' })
    expect(panel()).toBeNull()
    fireEvent.click(gate('Reviews before verify'))
    expect(panel()).toHaveTextContent('Reviews before verify')
  })

  it('shows no gate on a flow with none', async () => {
    await renderView()
    await screen.findByText('Stage 3 · Execute')
    pick('Plain prompts (no kit)')
    expect(graph().querySelectorAll('.workflow-gate')).toHaveLength(0)
  })
})

describe('working nodes and the panel (AC5-AC7)', () => {
  it('highlights the node of a working agent, with its count in the accessible name, and no other', async () => {
    await renderView()
    await screen.findByText('Stage 3 · Execute')
    snapshot(session('s1', { stage: '3' }), session('s2', { project: 'beta', stage: '1.5' }))
    await send(own('s1', { tool: 'Edit' }), call('s1', { agent: 'backend-developer' }), own('s2', { tool: 'Read' }))
    expect(node('Stage 3 · Execute')).toHaveAccessibleName('Stage 3 · Execute · 2 working')
    expect(node('Stage 3 · Execute')).toHaveClass('working')
    expect(node('Stage 1 · Set up')).toHaveAccessibleName('Stage 1 · Set up · 1 working')
    expect(node('Stage 2 · Plan')).toHaveAccessibleName('Stage 2 · Plan')
    expect(node('Stage 2 · Plan')).not.toHaveClass('working')
    expect(nodes().filter((n) => n.classList.contains('working'))).toHaveLength(2)
  })

  it('does not highlight a node whose agent last called more than 5 minutes ago', async () => {
    await renderView()
    await screen.findByText('Stage 3 · Execute')
    snapshot(session('s1', { stage: '3' }))
    await send(own('s1', { ts: ago(6 * 60_000) }))
    expect(nodes().filter((n) => n.classList.contains('working'))).toHaveLength(0)
  })

  it('opens the panel on hover, focus and click, with one row per working agent: project, agent, time', async () => {
    await renderView()
    await screen.findByText('Stage 3 · Execute')
    snapshot(session('s1', { project: 'alpha', stage: '3' }), session('s2', { project: 'beta', stage: '3' }))
    await send(
      own('s1', { ts: ago(4 * 60_000) }), own('s1', { ts: ago(1 * 60_000) }),
      call('s1', { agent: 'backend-developer', ts: ago(30_000) }),
      own('s2', { ts: ago(10_000) }),
    )
    const build = node('Stage 3 · Execute')
    expect(panel()).toBeNull()
    fireEvent.mouseEnter(build)
    const rows = within(panel()).getAllByRole('listitem').map((li) => li.textContent)
    expect(rows).toEqual(['alpha · backend-developer · 0s', 'alpha · main session · 3m', 'beta · main session · 0s'])
    fireEvent.mouseLeave(build)
    expect(panel()).toBeNull()
    act(() => build.focus())
    expect(within(panel()).getAllByRole('listitem')).toHaveLength(3)
    act(() => build.blur())
    expect(panel()).toBeNull()
    fireEvent.click(build)
    expect(within(panel()).getAllByRole('listitem')).toHaveLength(3)
  })

  it('closes the panel on Escape and when another node opens', async () => {
    await renderView()
    await screen.findByText('Stage 3 · Execute')
    snapshot(session('s1', { stage: '3' }))
    await send(own('s1'))
    fireEvent.click(node('Stage 3 · Execute'))
    expect(panel()).not.toBeNull()
    fireEvent.keyDown(node('Stage 3 · Execute'), { key: 'Escape' })
    expect(panel()).toBeNull()
    fireEvent.click(node('Stage 3 · Execute'))
    fireEvent.click(node('Stage 4 · Review'))
    expect(panel()).toHaveAccessibleName(/Stage 4 · Review/)
    expect(screen.getAllByRole('region')).toHaveLength(1)
  })

  it('says so in the panel of a node with no agent working', async () => {
    await renderView()
    await screen.findByText('Stage 3 · Execute')
    fireEvent.mouseEnter(node('Stage 2 · Plan'))
    expect(panel()).toHaveTextContent(/no agent working/i)
    expect(within(panel()).queryAllByRole('listitem')).toHaveLength(0)
  })

  it('opens the panel under its node, clamped inside the stage, so it never widens the page', async () => {
    const rect = (left, top, width, height) => ({ left, top, width, height, right: left + width, bottom: top + height, x: left, y: top, toJSON() {} })
    const spy = vi.spyOn(Element.prototype, 'getBoundingClientRect').mockImplementation(function box() {
      if (this.classList.contains('workflow-stage')) return rect(10, 100, 343, 400) // a 375 px page: the stage is narrower than the graph
      if (this.dataset?.key === 'node:4') return rect(300, 220, 168, 52) // a node near the right edge, in the stage's second half
      if (this.dataset?.key === 'node:1') return rect(12, 150, 168, 52)
      return rect(0, 0, 0, 0)
    })
    const was = Object.getOwnPropertyDescriptor(HTMLElement.prototype, 'offsetWidth')
    Object.defineProperty(HTMLElement.prototype, 'offsetWidth', { configurable: true, get() { return this.classList?.contains('workflow-panel') ? 200 : 0 } })
    try {
      await renderView()
      await screen.findByText('Stage 3 · Execute')
      fireEvent.click(node('Stage 4 · Review'))
      const open = panel()
      expect(parseFloat(open.style.left)).toBe(143) // 343 - 200: inside the stage, not at the node's x of 290
      expect(parseFloat(open.style.top)).toBeGreaterThan(220 - 100 + 52 - 1) // under the node (its bottom is 272, the stage's top is 100) ...
      expect(parseFloat(open.style.top)).toBeLessThan(220 - 100 + 52 + 24) // ... close to it, not under the whole graph
      fireEvent.click(node('Stage 4 · Review'))
      fireEvent.click(node('Stage 1 · Set up'))
      expect(parseFloat(panel().style.left)).toBe(2) // the node's own left (12 - 10) when it fits
    } finally {
      spy.mockRestore()
      if (was) Object.defineProperty(HTMLElement.prototype, 'offsetWidth', was)
      else delete HTMLElement.prototype.offsetWidth
    }
  })

  it('ties the node to its panel with aria-expanded', async () => {
    await renderView()
    await screen.findByText('Stage 3 · Execute')
    expect(node('Stage 3 · Execute')).toHaveAttribute('aria-expanded', 'false')
    fireEvent.click(node('Stage 3 · Execute'))
    expect(node('Stage 3 · Execute')).toHaveAttribute('aria-expanded', 'true')
  })

  it('puts a plain sub-agent and its session\'s main thread on their own nodes', async () => {
    await renderView()
    await screen.findByText('Stage 3 · Execute')
    snapshot(session('s1', { project: 'alpha' }))
    pick('Plain prompts (no kit)')
    await send(own('s1', { tool: 'Edit', ts: ago(2000) }), call('s1', { tool: 'Grep', agent: 'Explore', ts: ago(1000) }))
    fireEvent.click(node('Understand'))
    expect(within(panel()).getByText(/^alpha · Explore · /)).toBeInTheDocument()
    fireEvent.click(node('Change'))
    expect(within(panel()).getByText(/^alpha · main session · /)).toBeInTheDocument()
  })

  it('places a session with a sub-agent Edit call that names no type on plain\'s Change', async () => {
    await renderView()
    await screen.findByText('Stage 3 · Execute')
    snapshot(session('s1', { project: 'alpha' }))
    pick('Plain prompts (no kit)')
    await send(call('s1', { tool: 'Edit', is_sidechain: true, agent: undefined }))
    expect(node('Change')).toHaveAccessibleName('Change · 1 working')
    fireEvent.click(node('Change'))
    expect(within(panel()).getByText(/^alpha · sub-agent · /)).toBeInTheDocument()
  })

  it('gives a stage id the route lacks its own node at the end of easy-kit', async () => {
    await renderView()
    await screen.findByText('Stage 3 · Execute')
    snapshot(session('s1', { stage: '6' }))
    await send(own('s1'))
    expect(nodeNames().at(-1)).toBe('Stage 6')
    expect(node('Stage 6')).toHaveAccessibleName('Stage 6 · 1 working')
  })

  it('sends a project assigned to a flow to that flow', async () => {
    await renderView({ workflow: { ...workflow, projects: { alpha: 'plain' } } })
    await screen.findByText('Stage 3 · Execute')
    snapshot(session('s1', { project: 'alpha', stage: '3' }))
    await send(own('s1', { tool: 'Bash', label: 'cargo' }))
    expect(nodes().filter((n) => n.classList.contains('working'))).toHaveLength(0) // not on easy-kit
    pick('Plain prompts (no kit)')
    expect(node('Check')).toHaveAccessibleName('Check · 1 working')
  })

  it('lists a session with no step under the line, on its own flow only, and nothing when there is none', async () => {
    await renderView()
    await screen.findByText('Stage 3 · Execute')
    expect(screen.queryByText(/step not known yet/i)).toBeNull()
    snapshot(session('s1', { project: 'beta' }))
    pick('Plain prompts (no kit)')
    expect(await screen.findByText(/^step not known yet: beta$/)).toBeInTheDocument()
    expect(line()).not.toHaveTextContent(/step not known yet/)
    pick('Easy kit')
    expect(screen.queryByText(/step not known yet: beta/)).toBeNull()
  })

  it('shows only labels: no tool, no call label, no path or message text', async () => {
    await renderView()
    await screen.findByText('Stage 3 · Execute')
    snapshot(session('s1', { stage: '3' }))
    await send(call('s1', { agent: 'backend-developer', tool: 'Read', label: 'SECRET-LABEL', file_path: '/home/me/SECRET-PATH', message: 'SECRET-MESSAGE' }))
    fireEvent.click(node('Stage 3 · Execute'))
    expect(within(panel()).getByText(/^alpha · backend-developer · /)).toBeInTheDocument()
    expect(document.body.textContent).not.toMatch(/SECRET/)
  })

  it('says in one muted line that there is no live layer outside a stream provider', async () => {
    await renderView({ live: false })
    expect(await screen.findByText('Stage 3 · Execute')).toBeInTheDocument()
    expect(screen.getAllByText(/no live stream on this page/i)).toHaveLength(1) // the start card says it too, shorter
  })

  it('keeps the line and says in one muted line that the stream dropped', async () => {
    await renderView()
    await screen.findByText('Stage 3 · Execute')
    snapshot(session('s1', { stage: '3' }))
    act(() => {
      stream().readyState = 0
      stream().emit('error')
    })
    expect(await screen.findByText(/stream dropped/i)).toBeInTheDocument()
    expect(screen.getAllByText(/stream/i).filter((el) => el.className.includes('muted'))).toHaveLength(1)
    expect(nodeNames()).toHaveLength(KIT.steps.length)
  })
})
