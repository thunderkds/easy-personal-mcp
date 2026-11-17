import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { LiveProvider } from './LiveView.jsx'
import WorkflowView from './WorkflowView.jsx'
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
// Events reach the page in batches of at most 250 ms and in order: a last call of a known type (the sentinel) shows when the batch before it has been drawn.
const SENTINEL = 'agent · frontend-developer'
const send = async (...events) => {
  for (const e of [...events, call('s1', { agent: 'frontend-developer' })]) stream().emit('live', e)
  await screen.findByText(SENTINEL)
}
const agentChips = () => screen.queryAllByText(/^agent ·/).map((el) => el.textContent)

const task = (id, status) => ({ id, title: `title of ${id}`, status })
const board = (...tasks) => ({ projects: [{ project: 'easy-personal', status: 'ok', board: 'linear', last_pull: null, abstained: 0, tasks }] })
const SOME_BOARD = board(
  task('T1', 'Todo'), task('T2', 'Todo'), task('T3', 'In Progress'), task('T4', 'Ready for Review'),
  task('T5', 'Ready for Review'), task('T6', 'Ready for Review'), task('T7', 'Done'), task('T8', 'Blocked'), task('T9', 'Closed (will not do)'),
)

async function renderView({ live = true, scope, ...overrides } = {}) {
  const f = fakeFetch(overrides)
  vi.stubGlobal('fetch', f)
  render(live ? <LiveProvider><WorkflowView scope={scope} /></LiveProvider> : <WorkflowView scope={scope} />)
  await screen.findByRole('heading', { name: 'Workflow' })
  if (live) {
    stream().readyState = 1
    stream().emit('open')
  }
  return f
}
const lane = (name) => screen.getByRole('article', { name })
const lanes = () => screen.getAllByRole('article').map((a) => within(a).getByRole('heading', { level: 3 }).textContent)
const chips = (el) => within(el).queryAllByRole('listitem').map((li) => li.textContent)

beforeEach(() => {
  window.location.hash = '#secret-token'
  FakeEventSource.instances = []
  vi.stubGlobal('EventSource', FakeEventSource)
})
afterEach(() => {
  vi.unstubAllGlobals()
})

describe('the map (AC4)', () => {
  it('draws one lane per stage in the order the route sends them, with skills and agents', async () => {
    await renderView()
    expect(await screen.findByText('Stage 3 · Execute')).toBeInTheDocument()
    expect(lanes()).toEqual(workflow.stages.map((s) => s.name))
    const build = lane('Stage 3 · Execute')
    expect(chips(within(build).getByRole('list', { name: 'Skills' }))).toEqual(['tdd', 'bugfix', 'diagnose', 'craft-spawn-prompt', 'migration-safety'])
    expect(chips(within(build).getByRole('list', { name: 'Agents' }))).toEqual(['common-infrastructure', 'backend-developer', 'frontend-developer'])
  })

  it('draws the gates between the lanes, the last one after Stage 5 (before: null)', async () => {
    await renderView()
    await screen.findByText('Stage 3 · Execute')
    const gates = [...document.querySelectorAll('.workflow-gate')].map((g) => g.textContent)
    expect(gates).toEqual(workflow.gates.map((g) => expect.stringContaining(g.text)))
    const plan = lane('Stage 2 · Plan').closest('li')
    expect(plan.nextElementSibling).toHaveTextContent('No TASK_GUIDE, no Stage 3')
    expect(lane('Stage 5 · Integrate').closest('li').nextElementSibling).toHaveTextContent('Verify passes before merge')
  })

  it('follows the route: another order, an extra field and a missing gate list change nothing else', async () => {
    const stages = [{ id: '3', name: 'Build', skills: ['tdd'], agents: [], extra: 1 }, { id: '0.5', name: 'Define', skills: [], agents: ['Supervisor'] }]
    await renderView({ workflow: { stages, gates: [{ after: '9', before: null, text: 'Lost gate' }], future: true } })
    expect(await screen.findByText('Build')).toBeInTheDocument()
    expect(lanes()).toEqual(['Build', 'Define'])
    expect(screen.getByText(/Lost gate/)).toBeInTheDocument() // a gate whose lane is unknown is kept, at the end
  })

  it('shows an error line, not a blank page, when the route fails', async () => {
    vi.stubGlobal('fetch', vi.fn(async (url) => ({ ok: false, status: 500, json: async () => ({}) })))
    render(<WorkflowView />)
    expect(await screen.findByRole('alert')).toHaveTextContent('Could not load workflow')
    expect(screen.getByRole('heading', { name: 'Workflow' })).toBeInTheDocument()
  })
})

describe('the live layer (AC5)', () => {
  it('puts a session on the lane of its stage, with project name and skill', async () => {
    await renderView()
    await screen.findByText('Stage 3 · Execute')
    snapshot(session('s1', { project: 'alpha', stage: '4', stage_skill: 'code-review' }))
    const review = lane('Stage 4 · Review')
    expect(await within(review).findByText('alpha · code-review')).toBeInTheDocument()
    expect(within(lane('Stage 3 · Execute')).queryByText(/alpha/)).toBeNull()
  })

  it('lists a session with stage null under "stage not known yet", not on any lane', async () => {
    await renderView()
    await screen.findByText('Stage 3 · Execute')
    expect(screen.queryByText(/stage not known yet/i)).toBeNull() // nothing to list: no empty group
    snapshot(session('s1', { project: 'beta' }))
    const group = await screen.findByRole('article', { name: /stage not known yet/i })
    expect(within(group).getByText('beta')).toBeInTheDocument()
    expect(lanes()).toHaveLength(workflow.stages.length + 1)
  })

  it('keeps two sessions of one project on their own lanes', async () => {
    await renderView()
    await screen.findByText('Stage 3 · Execute')
    snapshot(session('s1', { stage: '3', stage_skill: 'tdd' }), session('s2', { stage: '5', stage_skill: 'verify' }))
    expect(await within(lane('Stage 3 · Execute')).findByText('alpha · tdd')).toBeInTheDocument()
    expect(within(lane('Stage 5 · Integrate')).getByText('alpha · verify')).toBeInTheDocument()
  })

  it('gives a stage id the route lacks its own lane at the end', async () => {
    await renderView()
    await screen.findByText('Stage 3 · Execute')
    snapshot(session('s1', { stage: '6', stage_skill: 'future-skill' }))
    expect(await screen.findByRole('article', { name: 'Stage 6' })).toBeInTheDocument()
    expect(lanes().at(-1)).toBe('Stage 6')
    expect(within(lane('Stage 6')).getByText('alpha · future-skill')).toBeInTheDocument()
  })

  it('shows a sub-agent seen in the last 5 minutes on every lane that lists it, spelling aside', async () => {
    await renderView()
    await screen.findByText('Stage 3 · Execute')
    snapshot(session('s1', { stage: '3' }))
    await send(call('s1', { agent: 'Backend-Implementer' }), call('s1', { agent: 'qa-expert' }))
    expect(within(lane('Stage 3 · Execute')).getByText('agent · Backend-Implementer')).toBeInTheDocument()
    expect(within(lane('Stage 4 · Review')).getByText('agent · qa-expert')).toBeInTheDocument()
    expect(within(lane('Stage 5 · Integrate')).getByText('agent · qa-expert')).toBeInTheDocument()
    expect(within(lane('Stage 2 · Plan')).queryByText(/agent ·/)).toBeNull()
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
    expect(agentChips()).toEqual([SENTINEL])
  })

  it('does not place an agent type that no lane lists', async () => {
    await renderView()
    await screen.findByText('Stage 3 · Execute')
    snapshot(session('s1', { stage: '3' }))
    await send(call('s1', { agent: 'Explore' }))
    expect(agentChips()).toEqual([SENTINEL])
  })

  it('shows at most 5 live chips on a lane, then "+N more"', async () => {
    await renderView()
    await screen.findByText('Stage 3 · Execute')
    snapshot(...Array.from({ length: 8 }, (_, i) => session(`s${i}`, { project: `proj${i}`, stage: '3', stage_skill: 'tdd' })))
    const build = lane('Stage 3 · Execute')
    expect(await within(build).findByText('+3 more')).toBeInTheDocument()
    expect(within(build).getAllByText(/^proj\d · tdd$/)).toHaveLength(5)
  })

  it('shows only labels: no tool, no call label, no path or message text', async () => {
    await renderView()
    await screen.findByText('Stage 3 · Execute')
    snapshot(session('s1', { stage: '3', stage_skill: 'tdd' }))
    await send(call('s1', { agent: 'backend-developer', tool: 'Read', label: 'SECRET-LABEL', file_path: '/home/me/SECRET-PATH', message: 'SECRET-MESSAGE' }))
    expect(within(lane('Stage 3 · Execute')).getByText('agent · backend-developer')).toBeInTheDocument()
    expect(document.body.textContent).not.toMatch(/SECRET/)
  })

  it('says in one muted line that there is no live layer outside a stream provider', async () => {
    await renderView({ live: false })
    expect(await screen.findByText('Stage 3 · Execute')).toBeInTheDocument()
    expect(screen.getAllByText(/no live stream/i)).toHaveLength(1)
  })

  it('keeps the map and says in one muted line that the stream dropped', async () => {
    await renderView()
    await screen.findByText('Stage 3 · Execute')
    snapshot(session('s1', { stage: '3', stage_skill: 'tdd' }))
    await within(lane('Stage 3 · Execute')).findByText('alpha · tdd')
    act(() => {
      stream().readyState = 0
      stream().emit('error')
    })
    expect(await screen.findByText(/stream dropped/i)).toBeInTheDocument()
    expect(screen.getAllByText(/stream/i).filter((el) => el.className.includes('muted'))).toHaveLength(1)
    expect(lanes()).toHaveLength(workflow.stages.length)
  })
})

describe('the board layer (AC6)', () => {
  it('counts the page scope by status: Todo before Stage 3, In Progress at 3, Ready for Review at 4-5, Done last', async () => {
    const f = await renderView({ board: SOME_BOARD, scope: { workspace: 'personal', account: '' } })
    expect(await screen.findByText('2 waiting for Stage 3')).toBeInTheDocument()
    expect(within(lane('Stage 3 · Execute')).getByText('In Progress 1')).toBeInTheDocument()
    expect(within(lane('Stage 4 · Review')).getByText('Ready for Review 3 · Stage 4–5')).toBeInTheDocument()
    expect(screen.getByText('1 done')).toBeInTheDocument()
    expect(screen.queryByText(/blocked|closed/i)).toBeNull() // other statuses are not counted
    expect(f.calls.some((c) => c.url === '/api/board?workspace=personal')).toBe(true)
    // order on the page: waiting just before Stage 3, done after Stage 5
    const items = [...document.querySelectorAll('.workflow-lanes > li')].map((li) => li.textContent)
    const at = (needle) => items.findIndex((t) => t.includes(needle))
    expect(at('2 waiting for Stage 3')).toBe(at('Stage 3 · Execute') - 1)
    expect(at('1 done')).toBeGreaterThan(at('Stage 5 · Integrate'))
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

  it('hides the counts with one muted line when the board fails, and still draws the map', async () => {
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

describe('keyboard and structure (UI criteria)', () => {
  it('has no button or hover-only control: lanes and chips are plain text', async () => {
    await renderView({ board: SOME_BOARD })
    await screen.findByText('Stage 3 · Execute')
    expect(screen.queryAllByRole('button')).toHaveLength(0)
    expect(document.querySelectorAll('[title]')).toHaveLength(0)
    fireEvent.keyDown(document.body, { key: 'Tab' })
  })
})
