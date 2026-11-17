import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { LiveProvider } from './LiveView.jsx'
import WorkflowView, { placeSession } from './WorkflowView.jsx'
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
// "sentinelkit" is assigned to easy-kit (no stage: its "step not known yet" group), "sentinelplain" goes to plain by its Edit call.
const SENTINEL = /^sentinel(kit|plain)$/
const send = async (...events) => {
  const last = [call('sk', { project: 'sentinelkit', tool: 'Edit', is_sidechain: false }), call('sp', { project: 'sentinelplain', tool: 'Edit', is_sidechain: false })]
  for (const e of [...events, ...last]) stream().emit('live', e)
  await screen.findByText(SENTINEL)
}
const agentChips = () => screen.queryAllByText(/^agent ·/).map((el) => el.textContent)
const own = (id, over = {}) => call(id, { tool: 'Edit', is_sidechain: false, ...over }) // a main-thread call

const task = (id, status) => ({ id, title: `title of ${id}`, status })
const board = (...tasks) => ({ projects: [{ project: 'easy-personal', status: 'ok', board: 'linear', last_pull: null, abstained: 0, tasks }] })
const SOME_BOARD = board(
  task('T1', 'Todo'), task('T2', 'Todo'), task('T3', 'In Progress'), task('T4', 'Ready for Review'),
  task('T5', 'Ready for Review'), task('T6', 'Ready for Review'), task('T7', 'Done'), task('T8', 'Blocked'), task('T9', 'Closed (will not do)'),
)

async function renderView({ live = true, scope, ...overrides } = {}) {
  const base = overrides.workflow ?? workflow
  const f = fakeFetch({ ...overrides, workflow: { ...base, projects: { sentinelkit: 'easy-kit', ...base.projects } } })
  vi.stubGlobal('fetch', f)
  render(live ? <LiveProvider><WorkflowView scope={scope} /></LiveProvider> : <WorkflowView scope={scope} />)
  await screen.findByRole('heading', { name: 'Workflow' })
  if (live) {
    stream().readyState = 1
    stream().emit('open')
  }
  return f
}
const [KIT, PLAIN] = workflow.flows
const stepArticle = (name) => screen.getByRole('article', { name })
const line = () => screen.getByRole('list', { name: 'Steps' })
const stepNames = () => within(line()).getAllByRole('article').map((a) => within(a).getByRole('heading', { level: 3 }).textContent)
const chips = (el) => within(el).queryAllByRole('listitem').map((li) => li.textContent)
const pick = (name) => fireEvent.click(screen.getByRole('button', { name }))

beforeEach(() => {
  window.location.hash = '#secret-token'
  FakeEventSource.instances = []
  vi.stubGlobal('EventSource', FakeEventSource)
})
afterEach(() => {
  vi.unstubAllGlobals()
})

describe('placeSession (AC5): which flow and which step', () => {
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

  it('draws easy-kit as one row of six steps in the route order, with parts and agents', async () => {
    await renderView()
    await screen.findByText('Stage 3 · Execute')
    expect(line().closest('.workflow-line')).not.toBeNull()
    expect(line()).toHaveAttribute('tabindex', '0') // a keyboard user can scroll the strip when the column is narrow
    expect(stepNames()).toEqual(KIT.steps.map((x) => x.name))
    expect(within(stepArticle('Stage 0 · Define')).getByText('Requirements · Grilling · Direction')).toBeInTheDocument()
    expect(within(stepArticle('Stage 1 · Set up')).getByText('Environment · Agent team')).toBeInTheDocument()
    expect(chips(within(stepArticle('Stage 3 · Execute')).getByRole('list', { name: 'Agents' }))).toEqual(['common-infrastructure', 'backend-developer', 'frontend-developer'])
  })

  it('draws the plain flow with its own steps and agents when picked', async () => {
    await renderView()
    await screen.findByText('Stage 3 · Execute')
    pick('Plain prompts (no kit)')
    expect(stepNames()).toEqual(['Understand', 'Plan', 'Change', 'Check', 'Ship'])
    expect(chips(within(stepArticle('Understand')).getByRole('list', { name: 'Agents' }))).toEqual(['Explore'])
    expect(within(stepArticle('Change')).queryByRole('list', { name: 'Agents' })).toBeNull()
  })

  it('shows no skill name, not even the one of a session', async () => {
    await renderView()
    await screen.findByText('Stage 3 · Execute')
    snapshot(session('s1', { stage: '3', stage_skill: 'tdd' }))
    await screen.findByText('alpha')
    for (const flow of [KIT.name, workflow.flows[1].name]) {
      pick(flow)
      expect(document.body.textContent).not.toMatch(/brainstorming|ideate|map-codebase|craft-agent|grill-with-docs|\btdd\b|bugfix|code-review|delivery-report|migration-safety/)
    }
  })

  it('follows the route: another flow list, an extra field and a gate whose step is unknown', async () => {
    const odd = { flows: [{ id: 'odd', name: 'Odd flow', future: 1, steps: [{ id: 'b', name: 'Build', signals: {} }, { id: 'a', name: 'Ask', parts: ['p'], agents: ['Supervisor'], signals: {} }], gates: [{ after: '9', before: null, text: 'Lost gate' }] }], projects: {}, future: true }
    await renderView({ workflow: odd })
    expect(await screen.findByText('Build')).toBeInTheDocument()
    expect(stepNames()).toEqual(['Build', 'Ask'])
    expect(screen.getByText(/Lost gate/)).toBeInTheDocument() // a gate whose step is unknown is kept, at the end
  })

  it('shows an error line, not a blank page, when the route fails', async () => {
    vi.stubGlobal('fetch', vi.fn(async () => ({ ok: false, status: 500, json: async () => ({}) })))
    render(<WorkflowView />)
    expect(await screen.findByRole('alert')).toHaveTextContent('Could not load workflow')
    expect(screen.getByRole('heading', { name: 'Workflow' })).toBeInTheDocument()
  })
})

describe('gates (AC7)', () => {
  it('draws each gate as visible text between its steps, the last one after Stage 5', async () => {
    await renderView()
    await screen.findByText('Stage 3 · Execute')
    const gates = [...document.querySelectorAll('.workflow-gate')].map((g) => g.textContent)
    expect(gates).toEqual(KIT.gates.map((g) => expect.stringContaining(g.text)))
    // each gate sits in the column of the step it follows (its edge is the boundary to the next step); the last one after Stage 5
    expect(stepArticle('Stage 2 · Plan').closest('li')).toHaveTextContent('No TASK_GUIDE, no Stage 3')
    expect(stepArticle('Stage 3 · Execute').closest('li')).not.toHaveTextContent('No TASK_GUIDE, no Stage 3')
    expect(stepArticle('Stage 5 · Integrate').closest('li')).toHaveTextContent('Verify passes before merge')
    expect(document.querySelectorAll('.workflow-gate [title], .workflow-gate[title]')).toHaveLength(0)
  })

  it('shows no gate on a flow with none', async () => {
    await renderView()
    await screen.findByText('Stage 3 · Execute')
    pick('Plain prompts (no kit)')
    expect(document.querySelectorAll('.workflow-gate')).toHaveLength(0)
  })
})

describe('the live layer (AC5)', () => {
  it('puts a session with stage 1.5 on easy-kit\'s Stage 1 step, with the project name only', async () => {
    await renderView()
    await screen.findByText('Stage 3 · Execute')
    snapshot(session('s1', { project: 'alpha', stage: '1.5', stage_skill: 'craft-agent' }))
    const step = stepArticle('Stage 1 · Set up')
    expect(await within(step).findByText('alpha')).toBeInTheDocument()
    expect(within(stepArticle('Stage 3 · Execute')).queryByText(/alpha/)).toBeNull()
    expect(document.body.textContent).not.toMatch(/craft-agent/)
  })

  it('puts a session with no stage on plain by its newest event: Edit on Change, a cargo Bash call on Check', async () => {
    await renderView()
    await screen.findByText('Stage 3 · Execute')
    snapshot(session('s1', { project: 'alpha' }), session('s2', { project: 'beta' }))
    await send(own('s1', { tool: 'Edit' }), own('s2', { tool: 'Bash', label: 'cargo' }))
    expect(screen.queryByText('alpha')).toBeNull() // not on the easy-kit line it does not belong to
    pick('Plain prompts (no kit)')
    expect(within(stepArticle('Change')).getByText('alpha')).toBeInTheDocument()
    expect(within(stepArticle('Check')).getByText('beta')).toBeInTheDocument()
    expect(within(stepArticle('Understand')).queryByText(/alpha|beta/)).toBeNull()
  })

  it('places a session on plain\'s Change by a sub-agent Edit call that names no agent type', async () => {
    await renderView()
    await screen.findByText('Stage 3 · Execute')
    snapshot(session('s1', { project: 'alpha' }))
    pick('Plain prompts (no kit)')
    await send(call('s1', { tool: 'Edit', is_sidechain: true, agent: undefined }))
    expect(within(stepArticle('Change')).getByText('alpha')).toBeInTheDocument()
  })

  it('moves a plain session to the step of its newest event', async () => {
    await renderView()
    await screen.findByText('Stage 3 · Execute')
    snapshot(session('s1'))
    pick('Plain prompts (no kit)')
    await send(own('s1', { tool: 'Read', ts: ago(2000) }), own('s1', { tool: 'Bash', label: 'git', ts: ago(1000) }))
    expect(within(stepArticle('Ship')).getByText('alpha')).toBeInTheDocument()
    expect(within(stepArticle('Understand')).queryByText('alpha')).toBeNull()
  })

  it('lists a session with no matching event under "step not known yet", on its flow, not on any step', async () => {
    await renderView()
    await screen.findByText('Stage 3 · Execute')
    expect(screen.queryByText(/step not known yet/i)).toBeNull() // nothing to list: no empty group
    snapshot(session('s1', { project: 'beta', stage: null }))
    pick('Plain prompts (no kit)')
    const group = await screen.findByRole('article', { name: /step not known yet/i })
    expect(within(group).getByText('beta')).toBeInTheDocument()
    expect(stepNames()).toEqual(['Understand', 'Plan', 'Change', 'Check', 'Ship'])
    pick('Easy kit')
    expect(screen.queryByText(/step not known yet/i)).toBeNull() // this session is on plain
  })

  it('keeps two sessions of one project on their own steps', async () => {
    await renderView()
    await screen.findByText('Stage 3 · Execute')
    snapshot(session('s1', { stage: '3' }), session('s2', { stage: '5' }))
    expect(await within(stepArticle('Stage 3 · Execute')).findByText('alpha')).toBeInTheDocument()
    expect(within(stepArticle('Stage 5 · Integrate')).getByText('alpha')).toBeInTheDocument()
  })

  it('gives a stage id the route lacks its own step at the end of easy-kit', async () => {
    await renderView()
    await screen.findByText('Stage 3 · Execute')
    snapshot(session('s1', { stage: '6' }))
    expect(await screen.findByRole('article', { name: 'Stage 6' })).toBeInTheDocument()
    expect(stepNames().at(-1)).toBe('Stage 6')
    expect(within(stepArticle('Stage 6')).getByText('alpha')).toBeInTheDocument()
  })

  it('puts a session on the flow its project is assigned to', async () => {
    await renderView({ workflow: { ...workflow, projects: { alpha: 'plain' } } })
    await screen.findByText('Stage 3 · Execute')
    snapshot(session('s1', { project: 'alpha', stage: '3' }), session('s2', { project: 'beta', stage: '3' }))
    expect(await within(stepArticle('Stage 3 · Execute')).findByText('beta')).toBeInTheDocument()
    expect(screen.queryByText('alpha')).toBeNull()
    pick('Plain prompts (no kit)')
    expect(within(screen.getByRole('article', { name: /step not known yet/i })).getByText('alpha')).toBeInTheDocument()
  })

  it('shows a sub-agent seen in the last 5 minutes on every step that lists it, spelling aside', async () => {
    await renderView()
    await screen.findByText('Stage 3 · Execute')
    snapshot(session('s1', { stage: '3' }))
    await send(call('s1', { agent: 'Backend-Implementer' }), call('s1', { agent: 'qa-expert' }))
    expect(within(stepArticle('Stage 3 · Execute')).getByText('agent · Backend-Implementer')).toBeInTheDocument()
    expect(within(stepArticle('Stage 4 · Review')).getByText('agent · qa-expert')).toBeInTheDocument()
    expect(within(stepArticle('Stage 5 · Integrate')).getByText('agent · qa-expert')).toBeInTheDocument()
    expect(within(stepArticle('Stage 2 · Plan')).queryByText(/agent ·/)).toBeNull()
  })

  it('shows a sub-agent on the chosen flow only: Explore on plain\'s Understand', async () => {
    await renderView()
    await screen.findByText('Stage 3 · Execute')
    snapshot(session('s1', { stage: '3' }))
    await send(call('s1', { agent: 'Explore' }))
    expect(agentChips()).toEqual([]) // no easy-kit step lists Explore
    pick('Plain prompts (no kit)')
    expect(within(stepArticle('Understand')).getByText('agent · Explore')).toBeInTheDocument()
    expect(agentChips()).toEqual(['agent · Explore'])
  })

  it('ignores a sub-agent older than 5 minutes, a main-thread call and a sub-agent with no type', async () => {
    await renderView()
    await screen.findByText('Stage 3 · Execute')
    snapshot(session('s1', { stage: '3' }))
    await send(
      call('s1', { agent: 'backend-developer', ts: ago(6 * 60_000) }),
      call('s1', { is_sidechain: false, tool: 'Bash' }),
      call('s1', { agent: undefined }),
    )
    expect(agentChips()).toEqual([])
  })

  it('shows at most 5 session chips on a step, then "+N more"', async () => {
    await renderView()
    await screen.findByText('Stage 3 · Execute')
    snapshot(...Array.from({ length: 8 }, (_, i) => session(`s${i}`, { project: `proj${i}`, stage: '3' })))
    const build = stepArticle('Stage 3 · Execute')
    expect(await within(build).findByText('+3 more')).toBeInTheDocument()
    expect(within(build).getAllByText(/^proj\d$/)).toHaveLength(5)
  })

  it('shows only labels: no tool, no call label, no path or message text', async () => {
    await renderView()
    await screen.findByText('Stage 3 · Execute')
    snapshot(session('s1', { stage: '3' }))
    await send(call('s1', { agent: 'backend-developer', tool: 'Read', label: 'SECRET-LABEL', file_path: '/home/me/SECRET-PATH', message: 'SECRET-MESSAGE' }))
    expect(within(stepArticle('Stage 3 · Execute')).getByText('agent · backend-developer')).toBeInTheDocument()
    pick('Plain prompts (no kit)')
    expect(document.body.textContent).not.toMatch(/SECRET/)
  })

  it('says in one muted line that there is no live layer outside a stream provider', async () => {
    await renderView({ live: false })
    expect(await screen.findByText('Stage 3 · Execute')).toBeInTheDocument()
    expect(screen.getAllByText(/no live stream/i)).toHaveLength(1)
  })

  it('keeps the line and says in one muted line that the stream dropped', async () => {
    await renderView()
    await screen.findByText('Stage 3 · Execute')
    snapshot(session('s1', { stage: '3' }))
    await within(stepArticle('Stage 3 · Execute')).findByText('alpha')
    act(() => {
      stream().readyState = 0
      stream().emit('error')
    })
    expect(await screen.findByText(/stream dropped/i)).toBeInTheDocument()
    expect(screen.getAllByText(/stream/i).filter((el) => el.className.includes('muted'))).toHaveLength(1)
    expect(stepNames()).toHaveLength(KIT.steps.length)
  })
})

describe('the board layer (AC6)', () => {
  it('counts the page scope by status on easy-kit: Todo before Stage 3, In Progress at 3, Ready for Review at 4-5, Done last', async () => {
    const f = await renderView({ board: SOME_BOARD, scope: { workspace: 'personal', account: '' } })
    expect(await screen.findByText('2 waiting for Stage 3')).toBeInTheDocument()
    expect(within(stepArticle('Stage 3 · Execute')).getByText('In Progress 1')).toBeInTheDocument()
    expect(within(stepArticle('Stage 4 · Review')).getByText('Ready for Review 3 · Stage 4–5')).toBeInTheDocument()
    expect(screen.getByText('1 done')).toBeInTheDocument()
    expect(screen.queryByText(/blocked|closed/i)).toBeNull() // other statuses are not counted
    expect(f.calls.some((c) => c.url === '/api/board?workspace=personal')).toBe(true)
    // "waiting" closes the column just before Stage 3, "done" the last column
    expect(stepArticle('Stage 2 · Plan').closest('li')).toHaveTextContent('2 waiting for Stage 3')
    expect(stepArticle('Stage 5 · Integrate').closest('li')).toHaveTextContent('1 done')
    expect([...line().children]).toHaveLength(6) // no extra item on the line: the counts sit in the step columns
  })

  it('shows no board count on another flow', async () => {
    await renderView({ board: SOME_BOARD })
    await screen.findByText('2 waiting for Stage 3')
    pick('Plain prompts (no kit)')
    expect(screen.queryByText(/waiting for Stage 3|done$|In Progress|Ready for Review/)).toBeNull()
  })

  it('counts every project the board answers for', async () => {
    const two = { projects: [board(task('A', 'Todo')).projects[0], { ...board(task('B', 'Todo'), task('C', 'Done')).projects[0], project: 'other' }] }
    await renderView({ board: two })
    expect(await screen.findByText('2 waiting for Stage 3')).toBeInTheDocument()
    expect(screen.getByText('1 done')).toBeInTheDocument()
  })

  it('hides the counts with one muted line when the board has no task', async () => {
    await renderView({ board: { projects: [] } })
    expect(await screen.findByText(/no board tasks/i)).toBeInTheDocument()
    expect(screen.queryByText(/waiting for Stage 3/)).toBeNull()
    expect(screen.queryByText(/ done$/)).toBeNull()
  })

  it('hides the counts with one muted line when the board fails, and still draws the line', async () => {
    await renderView() // no board given: the fake answers 404
    expect(await screen.findByText(/board counts are not available/i)).toBeInTheDocument()
    expect(screen.getByText('Stage 3 · Execute')).toBeInTheDocument()
    expect(screen.queryByText(/waiting for Stage 3/)).toBeNull()
  })

  it('reads the board again when the scope changes', async () => {
    const f = fakeFetch({ board: SOME_BOARD })
    vi.stubGlobal('fetch', f)
    const { rerender } = render(<WorkflowView scope={{ workspace: '', account: '' }} />)
    await screen.findByText('2 waiting for Stage 3')
    rerender(<WorkflowView scope={{ workspace: 'personal', account: '' }} />)
    await waitFor(() => expect(f.calls.some((c) => c.url === '/api/board?workspace=personal')).toBe(true))
  })
})

describe('structure (UI criteria)', () => {
  it('has only the picker as buttons and no hover-only text', async () => {
    await renderView({ board: SOME_BOARD })
    await screen.findByText('Stage 3 · Execute')
    expect(screen.getAllByRole('button')).toHaveLength(workflow.flows.length)
    expect(document.querySelectorAll('[title]')).toHaveLength(0)
  })
})
