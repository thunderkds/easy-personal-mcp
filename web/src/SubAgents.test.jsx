import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import LiveView from './LiveView.jsx'
import Conversation, { AGENTS_POLL_MS, stateOf } from './SubAgents.jsx'
import { TRANSCRIPT_POLL_MS } from './Transcript.jsx'
import { FakeEventSource, entryAssistant, entryTool, entryUser, liveRow, page, pastServer, snapshot, stream } from './test/liveStream.js'

// Sub-agent logs in the panel (T125, FR-130). The routes are mocked as T124 built them: `GET /api/live/sessions/{id}/agents` gives
// `{agents: [{id, type, parent_call, depth, finished, size, modified}], total}` and `.../transcript?agent=<id>` pages ONE file (offsets per file).
const SID = 'aaaaaaaa-1111-2222'
const agent = (id, over = {}) => ({ id, type: 'Explore', parent_call: null, depth: 1, finished: true, size: 10, modified: '2026-11-19T08:00:00.000000Z', ...over })
const spawn = (id, over = {}) => entryTool(id, { name: 'Agent', label: 'Explore', input: '', result: 'done', ...over })

beforeEach(() => {
  window.location.hash = '#secret-token'
  window.localStorage.clear()
  FakeEventSource.instances = []
  vi.stubGlobal('EventSource', FakeEventSource)
})
afterEach(() => {
  vi.useRealTimers()
  vi.unstubAllGlobals()
})

const reply = (status, body) => ({ ok: status < 400, status, json: async () => body })

/**
 * A fetch for both routes. `main` and `logs[<agent id>]` are a page or a function of the query (`URLSearchParams`); `agents` is the list answer (or a
 * function), `agentsStatus` makes the list route answer an error. Every call is in `calls`. Other paths are handed to `inner` when there is one.
 */
function server({ main = page([]), logs = {}, agents = { agents: [], total: 0 }, agentsStatus = null, inner = null } = {}) {
  const calls = []
  const fn = async (url, init = {}) => {
    const u = new URL(String(url), 'http://127.0.0.1')
    const isAgents = /^\/api\/live\/sessions\/[^/]+\/agents$/.test(u.pathname)
    const isRead = /^\/api\/live\/sessions\/[^/]+\/transcript$/.test(u.pathname)
    if (!isAgents && !isRead && inner) return inner(url, init)
    calls.push({ path: u.pathname, search: u.search, params: u.searchParams, init })
    if (isAgents) return agentsStatus ? reply(agentsStatus.status, agentsStatus.body) : reply(200, typeof agents === 'function' ? agents() : agents)
    if (isRead) {
      const id = u.searchParams.get('agent')
      const source = id === null ? main : logs[id]
      if (source === undefined) return reply(404, { error: 'the sub-agent log cannot be read' })
      if (source && source.status) return reply(source.status, source.body)
      return reply(200, typeof source === 'function' ? source(u.searchParams) : source)
    }
    return reply(404, { error: 'no such route' })
  }
  fn.calls = calls
  fn.agentReads = () => calls.filter((c) => c.path.endsWith('/agents'))
  fn.reads = (id) => calls.filter((c) => c.path.endsWith('/transcript') && c.params.get('agent') === (id ?? null))
  return fn
}

/** Render the conversation with the server and wait for the main thread. */
async function show(fetchFn, props = {}) {
  vi.stubGlobal('fetch', fetchFn)
  const view = render(<Conversation key={SID} sessionId={SID} {...props} />)
  await waitFor(() => expect(screen.queryByText('Loading the conversation')).toBeNull())
  return view
}
const log = () => screen.getByRole('log', { name: 'Conversation' })
const trail = () => screen.getByRole('navigation', { name: 'Log location' })
const crumbs = () => [...trail().querySelectorAll('li')].map((li) => li.textContent.replace(/\s+/g, ' ').trim())
const list = () => screen.getByRole('region', { name: 'Sub-agents' })
const settle = () =>
  act(async () => {
    await vi.advanceTimersByTimeAsync(0)
  })
const tick = (ms) =>
  act(async () => {
    await vi.advanceTimersByTimeAsync(ms)
  })

describe('Open log on an Agent call (AC 1)', () => {
  it('an Agent call whose id is a listed parent_call has Open log; one click shows that sub-agent\'s conversation, read with ?agent=', async () => {
    const s = server({
      main: page([entryUser('go'), spawn('toolu_A'), entryTool('toolu_B', { name: 'Bash' })]),
      agents: { agents: [agent('a1', { parent_call: 'toolu_A' })], total: 1 },
      logs: { a1: page([entryUser('sub prompt'), entryAssistant('sub answer')]) },
    })
    await show(s)
    // exactly one Open log in the main log (the list has its own, named after the sub-agent)
    const buttons = within(log()).getAllByRole('button', { name: 'Open log' })
    expect(buttons).toHaveLength(1)
    fireEvent.click(buttons[0])
    expect(await screen.findByText('sub answer')).toBeInTheDocument()
    expect(screen.getByText('sub prompt')).toBeInTheDocument()
    expect(screen.queryByText('go')).toBeNull() // the main thread's entries are not in this log
    const [first] = s.reads('a1')
    expect(first.params.get('agent')).toBe('a1')
    expect(first.params.has('after')).toBe(false)
    expect(first.params.has('before')).toBe(false)
    expect(first.init.headers['X-Dashboard-Token']).toBe('secret-token')
    expect(first.search).not.toContain('secret-token')
  })

  it('a call that is not an Agent / Task call, or whose id no listed sub-agent names, has no Open log', async () => {
    const s = server({
      main: page([entryTool('toolu_A', { name: 'Bash' }), spawn('toolu_Z'), spawn('toolu_T', { name: 'Task' })]),
      agents: { agents: [agent('a1', { parent_call: 'toolu_A' }), agent('a2', { parent_call: 'toolu_T' })], total: 2 },
    })
    await show(s)
    // toolu_A is a Bash call (no Open log), toolu_Z is unlisted (none), toolu_T is a Task call (yes)
    expect(within(log()).getAllByRole('button', { name: 'Open log' })).toHaveLength(1)
  })

  it('an open log refreshes every 2 s from its own offset and pages back with Load earlier from its own start', async () => {
    vi.useFakeTimers()
    const answers = []
    const s = server({
      main: page([spawn('toolu_A')], { next: 500 }),
      agents: { agents: [agent('a1', { parent_call: 'toolu_A' })], total: 1 },
      logs: {
        a1: (q) => {
          answers.push(q.toString())
          if (q.has('before')) return page([entryAssistant('older one')], { start: 0, next: 40 })
          if (q.has('after')) return page([entryAssistant('newer one')], { next: 300 })
          return page([entryAssistant('first one')], { start: 120, next: 200 })
        },
      },
    })
    vi.stubGlobal('fetch', s)
    render(<Conversation key={SID} sessionId={SID} />)
    await settle()
    fireEvent.click(within(log()).getByRole('button', { name: 'Open log' }))
    await settle()
    expect(screen.getByText('first one')).toBeInTheDocument()
    await tick(TRANSCRIPT_POLL_MS)
    expect(screen.getByText('newer one')).toBeInTheDocument()
    expect(answers).toEqual(['agent=a1', 'after=200&agent=a1'])
    fireEvent.click(screen.getByRole('button', { name: 'Load earlier' }))
    await settle()
    expect(screen.getByText('older one')).toBeInTheDocument()
    expect(answers.at(-1)).toBe('before=120&agent=a1')
  })

  it('shows the entries as text: markup in a sub-agent\'s log is not interpreted', async () => {
    const s = server({
      main: page([spawn('toolu_A')]),
      agents: { agents: [agent('a1', { parent_call: 'toolu_A', type: 'Explore' })], total: 1 },
      logs: { a1: page([entryAssistant('<img src=x onerror=alert(1)> <b>bold</b>')]) },
    })
    await show(s)
    fireEvent.click(within(log()).getByRole('button', { name: 'Open log' }))
    expect(await screen.findByText('<img src=x onerror=alert(1)> <b>bold</b>')).toBeInTheDocument()
    expect(document.querySelector('img')).toBeNull()
    expect(document.querySelector('b')).toBeNull()
  })
})

describe('the Sub-agents list (AC 2)', () => {
  it('shows the count, each type and running / finished', async () => {
    const s = server({ agents: { agents: [agent('a1', { type: 'Explore', finished: true }), agent('a2', { type: 'Plan', finished: false, parent_call: 'toolu_P' })], total: 2 } })
    await show(s)
    expect(within(list()).getByRole('heading', { name: 'Sub-agents (2)' })).toBeInTheDocument()
    const rows = [...list().querySelectorAll('.agent-row')].map((r) => r.textContent)
    expect(rows[0]).toContain('Explore')
    expect(rows[0]).toContain('finished')
    expect(rows[1]).toContain('Plan')
    expect(rows[1]).toContain('running')
  })

  it('nests a sub-agent under the sub-agent whose log holds its parent call, at any depth; an unknown parent is at the top level', async () => {
    const s = server({
      agents: {
        agents: [
          agent('a1', { type: 'Explore', parent_call: 'toolu_1', depth: 1 }),
          agent('a2', { type: 'Plan', parent_call: 'toolu_2', depth: 2 }),
          agent('a3', { type: 'Review', parent_call: 'toolu_3', depth: 3 }),
          agent('a9', { type: 'Orphan', parent_call: 'toolu_lost', depth: 2 }),
        ],
        total: 4,
      },
      logs: {
        a1: page([spawn('toolu_2')]), // a1's log holds the call that started a2
        a2: page([spawn('toolu_3')]),
        a9: page([]),
      },
    })
    await show(s)
    await waitFor(() => expect(list().querySelectorAll('.agent-children')).toHaveLength(2))
    const top = [...list().querySelector('.agent-tree').children].map((li) => li.querySelector('.agent-type').textContent)
    expect(top).toEqual(['Explore', 'Orphan']) // Plan and Review are nested; Orphan's parent call is in no log
    const explore = list().querySelector('.agent-tree').children[0]
    const plan = explore.querySelector(':scope > .agent-children > li')
    expect(plan.querySelector('.agent-type')).toHaveTextContent('Plan')
    expect(plan.querySelector(':scope > .agent-children > li .agent-type')).toHaveTextContent('Review')
  })

  it('a sub-agent whose parent sub-agent is not in the list (the newest 200 cut) is at the top level, and the cut is said', async () => {
    const s = server({ agents: { agents: [agent('a2', { type: 'Plan', parent_call: 'toolu_gone', depth: 2 })], total: 250 } })
    await show(s)
    expect(within(list()).getByRole('heading', { name: 'Sub-agents (250)' })).toBeInTheDocument()
    expect(within(list()).getByText('Showing the newest 1 of 250.')).toBeInTheDocument()
    expect(list().querySelector('.agent-tree').children).toHaveLength(1)
  })

  it('an empty list says so with a count of 0 and offers no Open log', async () => {
    await show(server())
    expect(within(list()).getByRole('heading', { name: 'Sub-agents (0)' })).toBeInTheDocument()
    expect(within(list()).getByText('No sub-agents in this session.')).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: /^Open log/ })).toBeNull()
  })

  it('a sub-agent whose meta could not be read is not called running', async () => {
    expect(stateOf({ id: 'x', type: 'sub-agent', parent_call: null, depth: null, finished: false })).toBe('state unknown')
    expect(stateOf({ id: 'x', type: 'Explore', parent_call: 't', depth: 1, finished: false })).toBe('running')
    expect(stateOf({ id: 'x', type: 'sub-agent', parent_call: null, depth: null, finished: true })).toBe('finished')
    await show(server({ agents: { agents: [agent('a1', { type: 'sub-agent', parent_call: null, depth: null, finished: false })], total: 1 } }))
    expect(list().querySelector('.agent-row')).toHaveTextContent('state unknown')
    expect(list().querySelector('.agent-row')).not.toHaveTextContent('running')
  })

  it('reads the list now and then every 5 s while open, never every 2 s, and stops when it closes', async () => {
    vi.useFakeTimers()
    const s = server()
    vi.stubGlobal('fetch', s)
    const view = render(<Conversation key={SID} sessionId={SID} />)
    await settle()
    expect(s.agentReads()).toHaveLength(1)
    expect(AGENTS_POLL_MS).toBe(5000)
    await tick(4900) // the transcript polled twice by now, the list not
    expect(s.agentReads()).toHaveLength(1)
    await tick(100)
    expect(s.agentReads()).toHaveLength(2)
    await tick(5000)
    expect(s.agentReads()).toHaveLength(3)
    view.unmount()
    await tick(15000)
    expect(s.agentReads()).toHaveLength(3)
  })

  it('a new sub-agent shows up on the next read', async () => {
    vi.useFakeTimers()
    let now = { agents: [], total: 0 }
    const s = server({ agents: () => now })
    vi.stubGlobal('fetch', s)
    render(<Conversation key={SID} sessionId={SID} />)
    await settle()
    expect(within(list()).getByRole('heading', { name: 'Sub-agents (0)' })).toBeInTheDocument()
    now = { agents: [agent('a1', { type: 'Plan', finished: false, parent_call: 't1' })], total: 1 }
    await tick(AGENTS_POLL_MS)
    expect(within(list()).getByRole('heading', { name: 'Sub-agents (1)' })).toBeInTheDocument()
  })

  it('without the route (the Python server, an older app) there is no list and no breadcrumb, and the log is the main thread', async () => {
    const s = server({ main: page([entryUser('hello')]), agentsStatus: { status: 404, body: { error: 'no such route' } } })
    await show(s)
    expect(screen.queryByRole('region', { name: 'Sub-agents' })).toBeNull()
    expect(screen.queryByRole('navigation', { name: 'Log location' })).toBeNull()
    expect(screen.getByText('hello')).toBeInTheDocument()
  })

  it('a list that cannot be read for another reason says so in one line and the conversation still shows', async () => {
    const s = server({ main: page([entryUser('hello')]), agentsStatus: { status: 500, body: {} } })
    await show(s)
    expect(screen.getByText('The sub-agents cannot be listed right now.')).toBeInTheDocument()
    expect(screen.getByText('hello')).toBeInTheDocument()
  })
})

describe('the breadcrumb (AC 3)', () => {
  const nested = () =>
    server({
      main: page([entryUser('main one'), spawn('toolu_1')], { next: 900 }),
      agents: { agents: [agent('a1', { type: 'Explore', parent_call: 'toolu_1' }), agent('a2', { type: 'Plan', parent_call: 'toolu_2', depth: 2 })], total: 2 },
      logs: {
        a1: page([entryAssistant('explore says'), spawn('toolu_2')], { next: 50 }),
        a2: page([entryAssistant('plan says')], { next: 70 }),
      },
    })

  it('shows main alone on the main thread, then main › type › type down the path, the last one not a button', async () => {
    await show(nested())
    expect(crumbs()).toEqual(['main'])
    expect(within(trail()).queryByRole('button')).toBeNull()
    fireEvent.click(within(log()).getByRole('button', { name: 'Open log' }))
    await screen.findByText('explore says')
    expect(crumbs()).toEqual(['main', '› Explore'])
    fireEvent.click(within(log()).getByRole('button', { name: 'Open log' }))
    await screen.findByText('plan says')
    expect(crumbs()).toEqual(['main', '› Explore', '› Plan'])
    expect(trail().querySelector('[aria-current="location"]')).toHaveTextContent('Plan')
    expect(within(trail()).getAllByRole('button').map((b) => b.textContent)).toEqual(['main', 'Explore'])
  })

  it('each crumb leads back, and every log is read from its own start: an offset of one file is never sent for another', async () => {
    const s = nested()
    await show(s)
    fireEvent.click(within(log()).getByRole('button', { name: 'Open log' }))
    await screen.findByText('explore says')
    fireEvent.click(within(log()).getByRole('button', { name: 'Open log' }))
    await screen.findByText('plan says')
    fireEvent.click(within(trail()).getByRole('button', { name: 'Explore' }))
    await screen.findByText('explore says')
    expect(screen.queryByText('plan says')).toBeNull()
    expect(crumbs()).toEqual(['main', '› Explore'])
    fireEvent.click(within(trail()).getByRole('button', { name: 'main' }))
    await screen.findByText('main one')
    expect(crumbs()).toEqual(['main'])
    expect(screen.queryByText('explore says')).toBeNull()
    // every first read of a file carries no offset (a1 was read twice from scratch: opened, then come back to)
    for (const id of ['a1', 'a2']) expect(s.reads(id).every((c) => !c.params.has('after') && !c.params.has('before'))).toBe(true)
    expect(s.reads(null).filter((c) => c.params.has('after')).every((c) => c.params.get('after') === '900')).toBe(true)
  })

  it('opening a nested sub-agent from the list puts its whole way in the breadcrumb', async () => {
    const s = nested()
    await show(s)
    await waitFor(() => expect(list().querySelectorAll('.agent-children')).toHaveLength(1))
    fireEvent.click(within(list()).getByRole('button', { name: /^Open log of Plan/ }))
    await screen.findByText('plan says')
    expect(crumbs()).toEqual(['main', '› Explore', '› Plan'])
  })

  it('a new session (a new key) starts at main', async () => {
    const s = nested()
    vi.stubGlobal('fetch', s)
    const view = render(<Conversation key="one" sessionId="one" />)
    await screen.findByText('main one')
    fireEvent.click(within(log()).getByRole('button', { name: 'Open log' }))
    await screen.findByText('explore says')
    view.rerender(<Conversation key="two" sessionId="two" />)
    await screen.findByText('main one')
    expect(crumbs()).toEqual(['main'])
    expect(s.reads(null).at(-1).path).toContain('/two/')
    expect(s.reads(null).at(-1).params.has('after')).toBe(false)
  })
})

describe('a log that cannot be read (AC 6)', () => {
  it('a 404 from ?agent= is one line, the crumbs still lead back, and nothing crashes', async () => {
    const s = server({
      main: page([entryUser('main one'), spawn('toolu_1')]),
      agents: { agents: [agent('a1', { parent_call: 'toolu_1' })], total: 1 },
      logs: { a1: { status: 404, body: { error: 'the sub-agent log cannot be read' } } },
    })
    await show(s)
    fireEvent.click(within(log()).getByRole('button', { name: 'Open log' }))
    expect(await screen.findByText('The conversation cannot be shown: the sub-agent log cannot be read')).toBeInTheDocument()
    expect(screen.queryAllByText(/cannot be shown/)).toHaveLength(1)
    expect(crumbs()).toEqual(['main', '› Explore'])
    fireEvent.click(within(trail()).getByRole('button', { name: 'main' }))
    expect(await screen.findByText('main one')).toBeInTheDocument()
  })

  it('a 404 stops the 2 s polling of that file', async () => {
    vi.useFakeTimers()
    const s = server({
      main: page([spawn('toolu_1')]),
      agents: { agents: [agent('a1', { parent_call: 'toolu_1' })], total: 1 },
      logs: { a1: { status: 404, body: { error: 'the sub-agent log cannot be read' } } },
    })
    vi.stubGlobal('fetch', s)
    render(<Conversation key={SID} sessionId={SID} />)
    await settle()
    fireEvent.click(within(log()).getByRole('button', { name: 'Open log' }))
    await settle()
    await tick(TRANSCRIPT_POLL_MS * 3)
    expect(s.reads('a1')).toHaveLength(1)
  })
})

describe('in the Live and Past panels (AC 4)', () => {
  const S = 'bbbbbbbb-3333-4444'
  const withAgents = (inner, over = {}) =>
    server({
      inner,
      main: page([entryUser('past main'), spawn('toolu_1')]),
      agents: { agents: [agent('a1', { type: 'Explore', parent_call: 'toolu_1' })], total: 1 },
      logs: { a1: page([entryAssistant('past sub says')]) },
      ...over,
    })

  it('the Live panel of a terminal session has the list, the breadcrumb and Open log', async () => {
    const s = withAgents(null)
    vi.stubGlobal('fetch', s)
    render(<LiveView />)
    stream().emit('snapshot', snapshot([liveRow(SID)]))
    fireEvent.click(document.getElementById(`live-row-${SID}`))
    await screen.findByText('past main')
    expect(within(list()).getByRole('heading', { name: 'Sub-agents (1)' })).toBeInTheDocument()
    fireEvent.click(within(log()).getByRole('button', { name: 'Open log' }))
    expect(await screen.findByText('past sub says')).toBeInTheDocument()
    expect(s.agentReads()[0].path).toBe(`/api/live/sessions/${SID}/agents`)
    expect(crumbs()).toEqual(['main', '› Explore'])
  })

  it('a past session (read-only) opens a sub-agent the same way', async () => {
    const inner = pastServer({
      past: () => ({ sessions: [{ id: S, project: 'alpha', branch: 'main', first: '2026-11-18T07:00:00.000000Z', last: '2026-11-18T08:00:00.000000Z', kind: 'terminal', account: 'Claude work', model: 'sonnet', name: null }], next: null, window: { from: null, to: null, default: true } }),
    })
    const s = withAgents(inner)
    vi.stubGlobal('fetch', s)
    render(<LiveView />)
    stream().emit('snapshot', snapshot([]))
    fireEvent.click(screen.getByRole('button', { name: 'Past' }))
    await waitFor(() => expect(document.getElementById(`past-row-${S}`)).not.toBeNull())
    fireEvent.click(document.getElementById(`past-row-${S}`))
    await screen.findByText('past main')
    expect(screen.getByText('Past session: read-only')).toBeInTheDocument()
    fireEvent.click(within(log()).getByRole('button', { name: 'Open log' }))
    expect(await screen.findByText('past sub says')).toBeInTheDocument()
    expect(s.reads('a1')[0].path).toBe(`/api/live/sessions/${S}/transcript`)
    expect(crumbs()).toEqual(['main', '› Explore'])
    fireEvent.click(within(trail()).getByRole('button', { name: 'main' }))
    expect(await screen.findByText('past main')).toBeInTheDocument()
  })
})

describe('finding the owners while the list changes (Stage 4 fixes)', () => {
  const nestedUnder = (parentType, childType) => {
    const tree = list().querySelector('.agent-tree')
    const parent = [...tree.children].find((li) => li.querySelector(':scope > .agent-row .agent-type')?.textContent === parentType)
    return parent?.querySelector(`:scope > .agent-children > li .agent-type`)?.textContent === childType
  }

  it('an owner that was found is shown even when the list changes while the scan waits and the next run has nothing left to find', async () => {
    vi.useFakeTimers()
    let release
    const gate = new Promise((resolve) => (release = resolve))
    let version = 1
    const s = server({
      agents: () =>
        version === 1
          ? { agents: [agent('p1', { type: 'Explore', parent_call: 't1' }), agent('p2', { type: 'Plan', parent_call: 't9', modified: 'm1' }), agent('c2', { type: 'Review', parent_call: 'toolu_2', depth: 2 }), agent('c3', { type: 'Lost', parent_call: 'toolu_3', depth: 2 })], total: 4 }
          : { agents: [agent('p1', { type: 'Explore', parent_call: 't1' }), agent('p2', { type: 'Plan', parent_call: 't9', modified: 'm2' }), agent('c2', { type: 'Review', parent_call: 'toolu_2', depth: 2 })], total: 3 }, // c3 is cut, p2 changed
      logs: { p1: page([spawn('toolu_2')]), p2: () => gate.then(() => page([])), c2: page([]), c3: page([]) },
    })
    vi.stubGlobal('fetch', s)
    render(<Conversation key={SID} sessionId={SID} />)
    await settle()
    // c2 was found in p1's log while c3's scan waits on p2's log
    expect(nestedUnder('Explore', 'Review')).toBe(true)
    version = 2
    await tick(AGENTS_POLL_MS) // the restarted run has no child left to find
    expect(nestedUnder('Explore', 'Review')).toBe(true)
    release()
    await settle()
    expect(nestedUnder('Explore', 'Review')).toBe(true)
  })

  it('a log that could not be read is tried again on the next read of the list (an error is not remembered); nothing is read again once the owner is known', async () => {
    vi.useFakeTimers()
    let fail = true
    let modified = 'm1'
    const s = server({
      agents: () => ({ agents: [agent('p1', { type: 'Explore', parent_call: 't1', modified }), agent('c2', { type: 'Review', parent_call: 'toolu_2', depth: 2 })], total: 2 }),
      logs: { p1: page([spawn('toolu_2')]) },
    })
    // while `fail` is set the log answers 500
    const base = s
    const wrapped = async (url, init) => {
      const u = new URL(String(url), 'http://127.0.0.1')
      if (u.searchParams.get('agent') === 'p1' && fail) {
        base.calls.push({ path: u.pathname, search: u.search, params: u.searchParams, init })
        return reply(500, { error: 'busy' })
      }
      return base(url, init)
    }
    wrapped.reads = base.reads
    vi.stubGlobal('fetch', wrapped)
    render(<Conversation key={SID} sessionId={SID} />)
    await settle()
    expect(nestedUnder('Explore', 'Review')).toBe(false)
    expect(base.reads('p1')).toHaveLength(1)
    fail = false
    await tick(AGENTS_POLL_MS)
    expect(base.reads('p1')).toHaveLength(2)
    expect(nestedUnder('Explore', 'Review')).toBe(true)
    await tick(AGENTS_POLL_MS) // read, nothing changed, owner known: no more reads
    expect(base.reads('p1')).toHaveLength(2)
    modified = 'm2'
    await tick(AGENTS_POLL_MS) // changed, but the owner is known and a call never moves: still not read
    expect(base.reads('p1')).toHaveLength(2)
    expect(nestedUnder('Explore', 'Review')).toBe(true)
  })

  it('a changed log is read again for a child still without an owner, and the old read is replaced', async () => {
    vi.useFakeTimers()
    let modified = 'm1'
    let has = false
    const s = server({
      agents: () => ({ agents: [agent('p1', { type: 'Explore', parent_call: 't1', modified }), agent('c2', { type: 'Review', parent_call: 'toolu_2', depth: 2 })], total: 2 }),
      logs: { p1: () => page(has ? [spawn('toolu_2')] : []) },
    })
    vi.stubGlobal('fetch', s)
    render(<Conversation key={SID} sessionId={SID} />)
    await settle()
    expect(s.reads('p1')).toHaveLength(1)
    await tick(AGENTS_POLL_MS) // unchanged: not read again
    expect(s.reads('p1')).toHaveLength(1)
    expect(nestedUnder('Explore', 'Review')).toBe(false)
    modified = 'm2'
    has = true
    await tick(AGENTS_POLL_MS)
    expect(s.reads('p1')).toHaveLength(2)
    expect(nestedUnder('Explore', 'Review')).toBe(true)
  })
})
