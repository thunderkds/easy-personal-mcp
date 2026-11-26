import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import LiveView from './LiveView.jsx'
import { MAX_KEPT, TRANSCRIPT_POLL_MS } from './Transcript.jsx'
import { BRIDGE_OFF, QUEUE_NOTE } from './Composer.jsx'
import { FORCE_WAIT_MS, STILL_RUNNING, STOP_SENT } from './SessionPanel.jsx'
import { FakeEventSource, entryAssistant, entryReply, entrySkipped, entryTool, entryUser, liveRow, liveServer, page, snapshot, stream } from './test/liveStream.js'

// The one Live panel for a terminal session (T118, FR-128): header, the conversation read from the T115 route, the cards, the composer by state
// (T116's queue) and Stop. The routes are mocked as the T115 / T116 guides write them; nothing leaves the test.
const SID = 'aaaaaaaa-1111-2222'
const rowEl = () => document.getElementById(`live-row-${SID}`)
const panel = () => screen.getByRole('region', { name: /^Session in / })
const log = () => screen.getByRole('log', { name: 'Conversation' })

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

/** Render the Live view with one terminal session and open its panel. */
async function open(server, over = {}, props = {}) {
  vi.stubGlobal('fetch', server)
  render(<LiveView {...props} />)
  stream().emit('snapshot', snapshot([liveRow(SID, over)]))
  fireEvent.click(rowEl()) // the panel is there at once; its conversation comes with the first read
  if (!vi.isFakeTimers() && !server.state.transcriptStatus) await screen.findByRole('log', { name: 'Conversation' })
}
const settle = () =>
  act(async () => {
    await vi.advanceTimersByTimeAsync(0)
  })
const tick = (ms) =>
  act(async () => {
    await vi.advanceTimersByTimeAsync(ms)
  })

describe('header (AC 2)', () => {
  it('shows the name, project, branch, account, kind, busy state and the time of the last record', async () => {
    const server = liveServer()
    await open(server, { project: 'alpha', account: 'Claude work', branch: 'feat/live', proc: { busy: true, name: 'my review' } })
    const p = within(panel())
    expect(p.getByRole('heading', { name: 'my review' })).toBeInTheDocument()
    for (const [name, value] of [['Project', 'alpha'], ['Branch', 'feat/live'], ['Account', 'Claude work'], ['Kind', 'terminal'], ['Session', SID]])
      expect(p.getByText(name, { selector: 'dt' }).nextSibling).toHaveTextContent(value)
    expect(p.getByText('Last record', { selector: 'dt' }).nextSibling).toHaveTextContent(/ago|now|just|s$/)
    expect(p.getByText('busy', { selector: '.session-state' })).toBeInTheDocument()
  })

  it('the owner\'s name wins over Claude\'s, which wins over the project; idle is said when the process file says so', async () => {
    await open(liveServer(), { name: 'the owner name', proc: { busy: false, name: 'claude name' } })
    expect(within(panel()).getByRole('heading', { name: 'the owner name' })).toBeInTheDocument()
    expect(within(panel()).getByText('idle', { selector: '.session-state' })).toBeInTheDocument()
  })

  it('the kind: terminal; headless when the entrypoint starts with sdk', async () => {
    await open(liveServer(), { entrypoint: 'cli' })
    expect(within(panel()).getByText('Kind', { selector: 'dt' }).nextSibling).toHaveTextContent('terminal')
  })

  it('a headless session says headless', async () => {
    await open(liveServer(), { entrypoint: 'sdk-cli' })
    expect(within(panel()).getByText('Kind', { selector: 'dt' }).nextSibling).toHaveTextContent('headless')
  })

  it('moves the focus to the heading when the owner opens it', async () => {
    await open(liveServer())
    expect(within(panel()).getByRole('heading')).toHaveFocus()
  })
})

/** `server` as the Rust app answers it: the app-sessions list is there. */
function withApps(server) {
  const fn = async (url, init) => (url === '/api/app-sessions' ? { ok: true, status: 200, json: async () => ({ sessions: [] }) } : server(url, init))
  return Object.assign(fn, server)
}

describe('Changes (T137, FR-141)', () => {
  it('asks for the session folder\'s changes, named by the panel title', async () => {
    const onOpenChanges = vi.fn()
    await open(withApps(liveServer()), { name: 'the owner name' }, { appSessions: true, onOpenChanges })
    fireEvent.click(await within(panel()).findByRole('button', { name: 'Changes' }))
    expect(onOpenChanges).toHaveBeenCalledWith({ session: SID, title: 'the owner name' })
  })

  it('drops the button once the session leaves the list', async () => {
    await open(withApps(liveServer()), {}, { appSessions: true, onOpenChanges: vi.fn() })
    await within(panel()).findByRole('button', { name: 'Changes' })
    act(() => stream().emit('snapshot', snapshot([])))
    expect(within(panel()).getByText(/no longer active/)).toBeInTheDocument()
    expect(within(panel()).queryByRole('button', { name: 'Changes' })).toBeNull()
  })

  it('has no button on a server without app sessions', async () => {
    await open(liveServer(), {}, { onOpenChanges: vi.fn() })
    expect(within(panel()).queryByRole('button', { name: 'Changes' })).toBeNull()
  })
})

describe('the conversation (AC 2)', () => {
  it('is read once when the panel opens: the newest page, the token in the header only, nothing but the route in the URL', async () => {
    const server = liveServer({ pages: [page([entryUser('fix the bug'), entryAssistant('on it'), entryReply('also run the tests'), entrySkipped(3)])] })
    await open(server)
    expect(await within(log()).findByText('fix the bug')).toBeInTheDocument()
    expect(within(log()).getByText('on it')).toBeInTheDocument()
    expect(within(log()).getByText('Reply from the app')).toBeInTheDocument()
    expect(within(log()).getByText('also run the tests')).toBeInTheDocument()
    expect(within(log()).getByText('3 other records')).toBeInTheDocument()
    expect(within(log()).getAllByText(/^(You|Claude|Reply from the app)$/).map((e) => e.textContent)).toEqual(['You', 'Claude', 'Reply from the app'])
    const [first] = server.reads()
    expect(first.url).toBe(`/api/live/sessions/${SID}/transcript`)
    expect(first.init.headers['X-Dashboard-Token']).toBe('secret-token')
    for (const c of server.calls) expect(c.url).not.toContain('secret-token')
  })

  it('every text is React text, never HTML', async () => {
    const hostile = '<img src=x onerror="window.__pwned=1"> **not bold**'
    await open(liveServer({ pages: [page([entryUser(hostile), entryAssistant(hostile), entryTool('t1', { input: hostile, result: hostile })])] }))
    expect(await within(log()).findAllByText(hostile)).toHaveLength(2)
    fireEvent.click(within(log()).getByRole('button', { name: /Bash/ }))
    expect(within(log()).getAllByText(hostile)).toHaveLength(4)
    expect(document.querySelector('#live-panel img')).toBeNull()
    expect(window.__pwned).toBeUndefined()
  })

  it('is read again every 2 s with the offset of the last answer, new entries are added, and it stops when the panel closes', async () => {
    vi.useFakeTimers()
    const server = liveServer({ pages: [page([entryUser('first')], { next: 100 }), page([entryAssistant('second')], { next: 180 }), page([], { next: 180 })] })
    await open(server)
    await settle()
    expect(within(log()).getByText('first')).toBeInTheDocument()
    expect(server.reads()).toHaveLength(1)
    await tick(TRANSCRIPT_POLL_MS)
    expect(server.reads()).toHaveLength(2)
    expect(server.reads()[1].search).toBe('?after=100')
    expect(within(log()).getByText('first')).toBeInTheDocument()
    expect(within(log()).getByText('second')).toBeInTheDocument()
    await tick(TRANSCRIPT_POLL_MS)
    expect(server.reads()[2].search).toBe('?after=180')
    fireEvent.click(rowEl()) // closed
    const before = server.reads().length
    await tick(20_000)
    expect(server.reads()).toHaveLength(before)
  })

  it('a tool call is folded to its label; a click opens its input and result; an error is tagged', async () => {
    await open(liveServer({ pages: [page([entryTool('t1', { name: 'Bash', label: 'ls', input: 'ls -la\n  indented', result: 'total 0', is_error: true }), entryTool('t2', { name: 'mcp__gh__list', label: { server: 'gh', tool: 'list' }, result: 'x' })])] }))
    const fold = await within(log()).findByRole('button', { name: /Bash · ls/ })
    expect(fold).toHaveAttribute('aria-expanded', 'false')
    expect(within(log()).getByText('error')).toBeInTheDocument()
    expect(within(log()).queryByText('total 0')).toBeNull()
    expect(within(log()).getByRole('button', { name: /mcp__gh__list · gh · list/ })).toBeInTheDocument()
    fireEvent.click(fold)
    expect(fold).toHaveAttribute('aria-expanded', 'true')
    expect(within(log()).getByLabelText('Input of Bash · ls').textContent).toBe('ls -la\n  indented') // pre-wrap keeps the lines
    expect(within(log()).getByLabelText('Result of Bash · ls')).toHaveTextContent('total 0')
    fireEvent.click(fold)
    expect(within(log()).queryByText('total 0')).toBeNull()
  })

  describe('TodoWrite as a checklist (T128, FR-133)', () => {
    const todoInput = (todos) => JSON.stringify({ todos }, null, 2)
    const todoTool = (input, over = {}) => entryTool('td1', { name: 'TodoWrite', label: null, input, result: 'Todos have been modified successfully', ...over })

    it('draws one row per todo with a mark for pending, in progress and done, and no JSON', async () => {
      const todos = [
        { content: 'Write the test', status: 'completed', activeForm: 'Writing the test' },
        { content: 'Make it pass', status: 'in_progress', activeForm: 'Making it pass' },
        { content: 'Ship it', status: 'pending', activeForm: 'Shipping it' },
      ]
      await open(liveServer({ pages: [page([todoTool(todoInput(todos))])] }))
      const list = await within(log()).findByRole('list', { name: 'Todo list' })
      expect(within(list).getAllByRole('listitem').map((li) => [li.getAttribute('data-status'), li.textContent])).toEqual([
        ['completed', '●Write the test'],
        ['in_progress', '◐Make it pass'],
        ['pending', '○Ship it'],
      ])
      expect(within(list).getByRole('img', { name: 'done' })).toHaveTextContent('●')
      expect(within(list).getByRole('img', { name: 'in progress' })).toHaveTextContent('◐')
      expect(within(list).getByRole('img', { name: 'pending' })).toHaveTextContent('○')
      expect(within(log()).queryByText(/"todos"/)).toBeNull()
      expect(within(log()).queryByLabelText(/^Input of/)).toBeNull()
    })

    it('the checklist is there without opening the fold; the fold still opens the result', async () => {
      await open(liveServer({ pages: [page([todoTool(todoInput([{ content: 'One', status: 'pending', activeForm: 'Doing one' }]))])] }))
      await within(log()).findByRole('list', { name: 'Todo list' })
      const fold = within(log()).getByRole('button', { name: /TodoWrite/ })
      expect(fold).toHaveAttribute('aria-expanded', 'false')
      fireEvent.click(fold)
      expect(within(log()).getByLabelText('Result of TodoWrite')).toHaveTextContent('Todos have been modified successfully')
      expect(within(log()).queryByLabelText('Input of TodoWrite')).toBeNull()
    })

    it('a todo text is React text, never HTML', async () => {
      await open(liveServer({ pages: [page([todoTool(todoInput([{ content: '<img src=x onerror=alert(1)> <b>bold</b>', status: 'pending', activeForm: 'x' }]))])] }))
      await within(log()).findByRole('list', { name: 'Todo list' })
      expect(within(log()).getByText('<img src=x onerror=alert(1)> <b>bold</b>')).toBeInTheDocument()
      expect(log().querySelector('img[src="x"], b')).toBeNull()
    })

    it.each([
      ['text that is not JSON', 'not json'],
      ['a cut JSON text', '{\n  "todos": [\n    {"content": "a", "stat'],
      ['JSON of another shape', JSON.stringify({ todo: [] })],
      ['todos that are not a list', JSON.stringify({ todos: 'x' })],
      ['a todo with an unknown status', JSON.stringify({ todos: [{ content: 'a', status: 'blocked' }] })],
      ['a todo with no text', JSON.stringify({ todos: [{ content: '', status: 'pending' }] })],
      ['a todo that is not an object', JSON.stringify({ todos: ['a'] })],
      ['a hidden character shown as an escape (not valid JSON)', '{\n  "todos": [{"content": "a \\u{202e} b", "status": "pending"}]\n}'],
    ])('falls back to the JSON text for %s', async (_name, input) => {
      await open(liveServer({ pages: [page([todoTool(input)])] }))
      const fold = await within(log()).findByRole('button', { name: /TodoWrite/ })
      expect(within(log()).queryByRole('list', { name: 'Todo list' })).toBeNull()
      fireEvent.click(fold)
      expect(within(log()).getByLabelText('Input of TodoWrite').textContent).toBe(input)
    })

    it('an input that has not arrived yet (a result alone) falls back, and a later whole call turns it into a checklist', async () => {
      await open(liveServer({ pages: [page([{ kind: 'tool', id: 'td1', orphan: true, name: null, label: null, input: null, result: 'ok', is_error: false }])] }))
      await within(log()).findByText('result of an earlier call')
      expect(within(log()).queryByRole('list', { name: 'Todo list' })).toBeNull()
    })

    it('only a tool called TodoWrite is a checklist: another tool with the same JSON stays JSON', async () => {
      await open(liveServer({ pages: [page([entryTool('x1', { name: 'Other', label: null, input: todoInput([{ content: 'a', status: 'pending' }]) })])] }))
      await within(log()).findByRole('button', { name: /Other/ })
      expect(within(log()).queryByRole('list', { name: 'Todo list' })).toBeNull()
    })

    it('an empty list says so', async () => {
      await open(liveServer({ pages: [page([todoTool(todoInput([]))])] }))
      const list = await within(log()).findByRole('list', { name: 'Todo list' })
      expect(within(list).getByText('The list is empty.')).toBeInTheDocument()
    })
  })

  describe('a result and its call, in either order (T115 orphan results)', () => {
    const fold = (re) => within(log()).getByRole('button', { name: re })
    const detail = (name, re) => within(log()).getByLabelText(`${name} of ${re}`)

    it('call then result: one entry, the result joins the call and the call\'s name, label and input stay', async () => {
      vi.useFakeTimers()
      const server = liveServer({ pages: [page([entryTool('t1', { result: null })], { next: 50 }), page([{ kind: 'tool', id: 't1', orphan: true, name: null, label: null, input: null, result: 'done now', is_error: false }], { next: 90 })] })
      await open(server)
      await settle()
      await tick(TRANSCRIPT_POLL_MS)
      expect(within(log()).getAllByRole('button', { name: /Bash/ })).toHaveLength(1)
      fireEvent.click(fold(/Bash · ls/))
      expect(detail('Input', 'Bash · ls')).toHaveTextContent('ls -la')
      expect(detail('Result', 'Bash · ls')).toHaveTextContent('done now')
      expect(within(log()).queryByText('result of an earlier call')).toBeNull()
    })

    it('result then call: the later call fills the name, label and input of the entry the result made, in the same place', async () => {
      vi.useFakeTimers()
      const orphan = { kind: 'tool', id: 't1', orphan: true, name: null, label: null, input: null, result: 'the output', is_error: false }
      const server = liveServer({ pages: [page([entryUser('first'), orphan], { next: 50 }), page([entryTool('t1', { result: null })], { next: 90 })] })
      await open(server)
      await settle()
      expect(within(log()).getByRole('button', { name: /result of an earlier call/ })).toBeInTheDocument()
      await tick(TRANSCRIPT_POLL_MS)
      expect(within(log()).queryByText('result of an earlier call')).toBeNull()
      expect(within(log()).getAllByRole('button', { name: /Bash · ls/ })).toHaveLength(1)
      fireEvent.click(fold(/Bash · ls/))
      expect(detail('Input', 'Bash · ls')).toHaveTextContent('ls -la')
      expect(detail('Result', 'Bash · ls')).toHaveTextContent('the output') // a null result never replaces the one already shown
    })

    it('a result without its call is shown as "result of an earlier call", with its error tag', async () => {
      await open(liveServer({ pages: [page([{ kind: 'tool', id: 'gone', orphan: true, name: null, label: null, input: null, result: 'late output', is_error: true }])] }))
      const orphan = await within(log()).findByRole('button', { name: /result of an earlier call/ })
      expect(within(log()).getByText('error')).toBeInTheDocument()
      expect(within(log()).queryByText('a tool')).toBeNull()
      fireEvent.click(orphan)
      expect(detail('Result', 'result of an earlier call')).toHaveTextContent('late output')
      expect(within(log()).getByText('No input.')).toBeInTheDocument()
    })
  })

  it('a long block scrolls inside itself, and a cut block says so', async () => {
    await open(liveServer({ pages: [page([entryUser('x'.repeat(16_000), { cut: true }), entryTool('t1', { result: 'y'.repeat(16_000), cut: true })])] }))
    expect(await within(log()).findAllByText('Cut at 16,000 characters')).toHaveLength(1)
    fireEvent.click(within(log()).getByRole('button', { name: /Bash/ }))
    expect(within(log()).getAllByText('Cut at 16,000 characters')).toHaveLength(2) // the tool call says it once, whichever block was cut
    expect(within(log()).getByLabelText('Result of Bash · ls')).toHaveClass('turn-pre') // max-height + overflow in CSS
  })

  it('a tool call with no result yet is said, and the result that comes in a later answer replaces it in place', async () => {
    vi.useFakeTimers()
    const server = liveServer({ pages: [page([entryTool('t1', { result: null })], { next: 50 }), page([entryTool('t1', { result: 'done now' })], { next: 90 })] })
    await open(server)
    await settle()
    fireEvent.click(within(log()).getByRole('button', { name: /Bash/ }))
    expect(within(log()).getByText('No result yet.')).toBeInTheDocument()
    await tick(TRANSCRIPT_POLL_MS)
    expect(within(log()).getAllByRole('button', { name: /Bash/ })).toHaveLength(1) // not twice
    expect(within(log()).getByText('done now')).toBeInTheDocument()
  })

  it('Load earlier asks for the entries before the first one shown and puts them on top; it is offered only where the server said where the page starts', async () => {
    const server = liveServer({
      pages: (q) => (q.has('before') ? page([entryUser('older one'), entryAssistant('older two')], { start: 0, next: 400 }) : page([entryUser('newest')], { start: 400, next: 900 })),
    })
    await open(server)
    fireEvent.click(await screen.findByRole('button', { name: 'Load earlier' }))
    expect(await within(log()).findByText('older one')).toBeInTheDocument()
    expect(server.reads().at(-1).search).toBe('?before=400')
    const texts = [...log().querySelectorAll('.turn-text')].map((p) => p.textContent)
    expect(texts).toEqual(['older one', 'older two', 'newest'])
    expect(screen.queryByRole('button', { name: 'Load earlier' })).toBeNull() // the file starts here
  })

  it('with no `start` the first entry\'s `offset` says where the page starts; offset 0 means the file starts here', async () => {
    const server = liveServer({
      pages: (q) => (q.has('before') ? page([entryUser('older', { offset: 0 })], { start: 0 }) : page([entryUser('newest', { offset: 300 })], { next: 900 })),
    })
    await open(server)
    fireEvent.click(await screen.findByRole('button', { name: 'Load earlier' }))
    expect(await within(log()).findByText('older')).toBeInTheDocument()
    expect(server.reads().at(-1).search).toBe('?before=300')
    expect(screen.queryByRole('button', { name: 'Load earlier' })).toBeNull()
  })

  it('no start offset, no Load earlier button', async () => {
    await open(liveServer({ pages: [page([entryUser('only')])] }))
    await within(log()).findByText('only')
    expect(screen.queryByRole('button', { name: 'Load earlier' })).toBeNull()
  })

  it('an unreadable conversation says the server\'s one line and gives the tool calls seen since the page opened instead', async () => {
    const server = liveServer({ transcriptStatus: { status: 404, body: { error: 'that session\'s transcript cannot be read' } } })
    await open(server)
    expect(await screen.findByText('The conversation cannot be shown: that session\'s transcript cannot be read')).toBeInTheDocument()
    expect(within(panel()).getByText('No tool calls seen since this page opened.')).toBeInTheDocument()
    expect(screen.queryByRole('log', { name: 'Conversation' })).toBeNull()
  })

  it('a 404 (the file is gone) is not asked again; a network error is', async () => {
    vi.useFakeTimers()
    const server = liveServer({ pages: [page([entryUser('still here')])] })
    await open(server)
    await settle()
    server.state.transcriptStatus = { status: 404, body: { error: 'that session\'s transcript cannot be read' } }
    await tick(TRANSCRIPT_POLL_MS)
    const asked = server.reads().length
    expect(screen.getByText(/Cannot refresh the conversation right now/)).toBeInTheDocument()
    await tick(TRANSCRIPT_POLL_MS * 5)
    expect(server.reads()).toHaveLength(asked) // stopped
    expect(within(log()).getByText('still here')).toBeInTheDocument()
  })

  it('a network error keeps being retried and the next good answer clears the note', async () => {
    vi.useFakeTimers()
    const server = liveServer({ pages: [page([entryUser('still here')])] })
    await open(server)
    await settle()
    server.state.transcriptStatus = { status: 500, body: {} }
    await tick(TRANSCRIPT_POLL_MS * 2)
    expect(screen.getByText(/Cannot refresh the conversation right now/)).toBeInTheDocument()
    server.state.transcriptStatus = null
    await tick(TRANSCRIPT_POLL_MS)
    expect(screen.queryByText(/Cannot refresh the conversation right now/)).toBeNull()
  })

  it('a poll with no new entry leaves the drawn entries as they are (same nodes)', async () => {
    vi.useFakeTimers()
    const server = liveServer({ pages: [page([entryUser('one'), entryAssistant('two')], { next: 10 }), page([], { next: 10 })] })
    await open(server)
    await settle()
    const before = [...log().querySelectorAll('li')]
    await tick(TRANSCRIPT_POLL_MS * 3)
    expect(server.reads().length).toBeGreaterThan(2)
    const after = [...log().querySelectorAll('li')]
    expect(after).toHaveLength(before.length)
    after.forEach((li, i) => expect(li).toBe(before[i]))
  })

  it('only the newest MAX_KEPT entries are kept, and Load earlier starts at the first one kept', async () => {
    const entries = Array.from({ length: MAX_KEPT + 100 }, (_, i) => entryUser(`m${i}`, { offset: i * 10 }))
    const server = liveServer({ pages: (q) => (q.has('before') ? page([entryUser('older', { offset: 0 })], { start: 0 }) : page(entries, { start: 0, next: 99999 })) })
    await open(server)
    await within(log()).findByText(`m${MAX_KEPT + 99}`)
    expect(log().querySelectorAll('.turn-user')).toHaveLength(MAX_KEPT)
    expect(within(log()).queryByText('m99')).toBeNull()
    expect(within(log()).getByText('m100')).toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: 'Load earlier' }))
    await within(log()).findByText('older')
    expect(server.reads().filter((r) => r.search.startsWith('?before'))[0].search).toBe('?before=1000') // m100 is at offset 1000
  })

  it('a trim at a tool entry whose result came on a later poll starts at the call\'s offset, not the result\'s', async () => {
    vi.useFakeTimers()
    const filler = Array.from({ length: MAX_KEPT - 3 }, (_, i) => entryUser(`f${i}`, { offset: 10_000 + i }))
    const first = page([entryUser('gone one', { offset: 0 }), entryUser('gone two', { offset: 10 }), entryTool('t1', { result: null, offset: 100 }), ...filler], { start: 0, next: 50_000 })
    const second = page([{ kind: 'tool', id: 't1', orphan: true, name: null, label: null, input: null, result: 'late', is_error: false, offset: 60_000 }, entryUser('new one', { offset: 60_010 }), entryUser('new two', { offset: 60_020 })], { next: 60_100 })
    const server = liveServer({ pages: (q) => (q.has('before') ? page([entryUser('older', { offset: 0 })], { start: 0 }) : q.has('after') ? second : first) })
    await open(server)
    await settle()
    await tick(TRANSCRIPT_POLL_MS) // the result joins the call and two entries come: MAX_KEPT + 2, the trim cuts at the tool entry
    expect(within(log()).queryByText('gone two')).toBeNull()
    expect(within(log()).getByRole('button', { name: /Bash · ls/ })).toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: 'Load earlier' }))
    await act(async () => {
      await vi.advanceTimersByTimeAsync(0)
    })
    expect(server.reads().find((r) => r.search.startsWith('?before')).search).toBe('?before=100') // not 60000: the call is not drawn twice
    expect(within(log()).getAllByRole('button', { name: /Bash · ls/ })).toHaveLength(1)
  })

  it('a tool call joined across a Load earlier page: the call on the earlier page fills the result that was alone', async () => {
    const server = liveServer({
      pages: (q) =>
        q.has('before')
          ? page([entryUser('older', { offset: 0 }), entryTool('t9', { result: null, offset: 5 })], { start: 0 })
          : page([{ kind: 'tool', id: 't9', orphan: true, name: null, label: null, input: null, result: 'late output', is_error: false, offset: 400 }], { start: 400, next: 900 }),
    })
    await open(server)
    fireEvent.click(await screen.findByRole('button', { name: 'Load earlier' }))
    await within(log()).findByText('older')
    expect(within(log()).getAllByRole('button', { name: /Bash|result of an earlier call/ })).toHaveLength(1)
    const fold = within(log()).getByRole('button', { name: /Bash · ls/ })
    fireEvent.click(fold)
    expect(within(log()).getByLabelText('Input of Bash · ls')).toHaveTextContent('ls -la')
    expect(within(log()).getByLabelText('Result of Bash · ls')).toHaveTextContent('late output')
  })

  it('an answer for session A that comes after the owner switched to B does not show in B', async () => {
    const A = 'aaaaaaaa-1111-2222'
    const B = 'bbbbbbbb-3333-4444'
    const base = liveServer({ pages: (q) => page([entryUser('placeholder')]) })
    let releaseA
    const heldA = new Promise((resolve) => {
      releaseA = resolve
    })
    vi.stubGlobal('fetch', async (url, init) => {
      const path = new URL(String(url), 'http://127.0.0.1').pathname
      if (path === `/api/live/sessions/${A}/transcript`) {
        await heldA
        return { ok: true, status: 200, json: async () => page([entryUser('text of A')]) }
      }
      if (path === `/api/live/sessions/${B}/transcript`) return { ok: true, status: 200, json: async () => page([entryUser('text of B')]) }
      return base(url, init)
    })
    render(<LiveView />)
    stream().emit('snapshot', snapshot([liveRow(A, { project: 'alpha' }), liveRow(B, { project: 'beta' })]))
    fireEvent.click(document.getElementById(`live-row-${A}`))
    fireEvent.click(document.getElementById(`live-row-${B}`))
    expect(await screen.findByText('text of B')).toBeInTheDocument()
    await act(async () => {
      releaseA()
      await Promise.resolve()
    })
    expect(within(log()).queryByText('text of A')).toBeNull()
    expect(within(log()).getByText('text of B')).toBeInTheDocument()
  })

  it('a failed refresh keeps what was shown and says so', async () => {
    vi.useFakeTimers()
    const server = liveServer({ pages: [page([entryUser('still here')])] })
    await open(server)
    await settle()
    server.state.transcriptStatus = { status: 500, body: {} }
    await tick(TRANSCRIPT_POLL_MS)
    expect(within(log()).getByText('still here')).toBeInTheDocument()
    expect(screen.getByText(/Cannot refresh the conversation right now/)).toBeInTheDocument()
  })

  it('an empty conversation says so', async () => {
    await open(liveServer({ pages: [page([])] }))
    expect(await within(log()).findByText('Nothing yet.')).toBeInTheDocument()
  })

  it('nothing of the conversation goes to localStorage', async () => {
    const set = vi.spyOn(Storage.prototype, 'setItem')
    await open(liveServer({ pages: [page([entryUser('private words'), entryTool('t1', { result: 'secret output' })])] }))
    await within(log()).findByText('private words')
    fireEvent.click(within(log()).getByRole('button', { name: /Bash/ }))
    expect(set).not.toHaveBeenCalled()
    expect(JSON.stringify(Object.entries(window.localStorage))).not.toMatch(/private|secret output/)
    set.mockRestore()
  })
})

describe('cards (AC 3)', () => {
  const question = { id: 'p1', kind: 'question', questions: [{ question: 'Which database?', header: 'DB', multi_select: false, options: [{ label: 'Postgres', description: '' }, { label: 'SQLite', description: '' }] }] }

  it('a terminal session\'s card is pinned above the composer, outside the scroll box of the conversation', async () => {
    await open(liveServer({ pages: [page([entryUser('x')])] }), { pending_answer: question, reply_bridge: true, proc: { busy: true } })
    await within(log()).findByText('x')
    const card = within(panel()).getByRole('region', { name: 'Question from a terminal session' })
    expect(log().contains(card)).toBe(false)
    const composer = screen.getByRole('textbox', { name: /Queue a message/ })
    expect(card.compareDocumentPosition(composer) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy() // the card comes first
    expect(log().compareDocumentPosition(card) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
  })

  it('the row of a session with a card says "answer needed" even before it is opened', () => {
    vi.stubGlobal('fetch', liveServer())
    render(<LiveView />)
    stream().emit('snapshot', snapshot([liveRow(SID, { pending_answer: question })]))
    expect(within(rowEl()).getByText('answer needed')).toBeInTheDocument()
  })
})

describe('the composer by state (AC 4)', () => {
  const box = () => screen.getByRole('textbox', { name: /Queue a message/ })

  it('a busy terminal session with the bridge on queues: Enter posts the text, the queued text is shown with Edit, Cancel and "sent at the turn end"', async () => {
    const server = liveServer({ pages: [page([])] })
    await open(server, { reply_bridge: true, proc: { busy: true } })
    fireEvent.change(box(), { target: { value: 'then run the linter' } })
    fireEvent.keyDown(box(), { key: 'Enter' })
    expect(await screen.findByText('then run the linter', { selector: '.queue-shown .turn-text' })).toBeInTheDocument()
    const posts = server.posts('/api/reply/queue')
    expect(posts).toHaveLength(1)
    expect(posts[0].body).toEqual({ session_id: SID, text: 'then run the linter', confirmed: true })
    const shown = within(panel()).getByText(new RegExp(QUEUE_NOTE))
    expect(shown).toBeInTheDocument()
    expect(within(panel()).getByRole('button', { name: 'Edit' })).toBeInTheDocument()
    expect(within(panel()).getByRole('button', { name: 'Cancel queued text' })).toBeInTheDocument()
    expect(screen.queryByRole('textbox', { name: /Queue a message/ })).toBeNull()
  })

  it('shows a text the snapshot already carries (`queued`), Edit replaces it, Cancel posts null', async () => {
    const server = liveServer({ pages: [page([])] })
    await open(server, { reply_bridge: true, proc: { busy: true }, queued: { text: 'wait for me' } })
    expect(within(panel()).getByText('wait for me')).toBeInTheDocument()
    fireEvent.click(within(panel()).getByRole('button', { name: 'Edit' }))
    const edit = screen.getByRole('textbox', { name: 'Edit the queued text' })
    expect(edit).toHaveValue('wait for me')
    fireEvent.change(edit, { target: { value: 'wait, then merge' } })
    fireEvent.click(screen.getByRole('button', { name: 'Replace queued text' }))
    await waitFor(() => expect(server.posts('/api/reply/queue')).toHaveLength(1))
    expect(server.posts('/api/reply/queue')[0].body.text).toBe('wait, then merge')
    expect(await within(panel()).findByText('wait, then merge')).toBeInTheDocument()
    fireEvent.click(within(panel()).getByRole('button', { name: 'Cancel queued text' }))
    await waitFor(() => expect(server.posts('/api/reply/queue')).toHaveLength(2))
    expect(server.posts('/api/reply/queue')[1].body).toEqual({ session_id: SID, text: null, confirmed: true })
    expect(await screen.findByRole('textbox', { name: /Queue a message/ })).toBeInTheDocument()
  })

  it('the snapshot is the truth again once it changes: a delivered text goes away', async () => {
    await open(liveServer({ pages: [page([])] }), { reply_bridge: true, proc: { busy: true }, queued: { text: 'wait for me' } })
    expect(within(panel()).getByText('wait for me')).toBeInTheDocument()
    act(() => stream().emit('snapshot', snapshot([liveRow(SID, { reply_bridge: true, proc: { busy: true } })])))
    expect(within(panel()).queryByText('wait for me')).toBeNull()
    expect(screen.getByRole('textbox', { name: /Queue a message/ })).toBeInTheDocument()
  })

  it('a queued text delivered while it is being edited ends the edit: the plain box comes back with no "Keep the queued text"', async () => {
    await open(liveServer({ pages: [page([])] }), { reply_bridge: true, proc: { busy: true }, queued: { text: 'wait for me' } })
    fireEvent.click(within(panel()).getByRole('button', { name: 'Edit' }))
    expect(screen.getByRole('button', { name: 'Keep the queued text' })).toBeInTheDocument()
    act(() => stream().emit('snapshot', snapshot([liveRow(SID, { reply_bridge: true, proc: { busy: true } })])))
    expect(screen.queryByRole('button', { name: 'Keep the queued text' })).toBeNull()
    expect(screen.getByRole('button', { name: 'Queue' })).toBeInTheDocument()
  })

  it('Shift+Enter and an input method composition do not queue; an empty box cannot', async () => {
    const server = liveServer({ pages: [page([])] })
    await open(server, { reply_bridge: true, proc: { busy: true } })
    fireEvent.change(box(), { target: { value: 'ni' } })
    fireEvent.keyDown(box(), { key: 'Enter', shiftKey: true })
    fireEvent.keyDown(box(), { key: 'Enter', isComposing: true })
    fireEvent.keyDown(box(), { key: 'Enter', keyCode: 229 })
    expect(server.posts('/api/reply/queue')).toHaveLength(0)
    fireEvent.change(box(), { target: { value: '   ' } })
    expect(screen.getByRole('button', { name: 'Queue' })).toBeDisabled()
  })

  it('more than 8,000 characters is not sent', async () => {
    const server = liveServer({ pages: [page([])] })
    await open(server, { reply_bridge: true, proc: { busy: true } })
    fireEvent.change(box(), { target: { value: 'a'.repeat(8001) } })
    expect(screen.getByRole('alert')).toHaveTextContent('Too long to send')
    fireEvent.keyDown(box(), { key: 'Enter' })
    expect(server.posts('/api/reply/queue')).toHaveLength(0)
  })

  it('a refused queue (409) shows the server\'s line and keeps the typed text', async () => {
    const server = liveServer({ pages: [page([])], queue: () => ({ status: 409, body: { error: 'this session cannot take a queued text' } }) })
    await open(server, { reply_bridge: true, proc: { busy: true } })
    fireEvent.change(box(), { target: { value: 'still here' } })
    fireEvent.keyDown(box(), { key: 'Enter' })
    expect(await screen.findByRole('alert')).toHaveTextContent('this session cannot take a queued text')
    expect(box()).toHaveValue('still here')
  })

  it('a session at your turn that holds a reply keeps the reply box of FR-122', async () => {
    await open(liveServer({ pages: [page([])] }), { reply_bridge: true, pending_reply: { id: 'r1', until: Date.now() / 1000 + 80, last: 'Anything else?' } })
    expect(screen.getByRole('button', { name: 'Hand back to terminal' })).toBeInTheDocument()
    expect(screen.getByText('Anything else?')).toBeInTheDocument()
    expect(screen.queryByRole('textbox', { name: /Queue a message/ })).toBeNull()
  })

  it('with the bridge off it says so and links to Settings; nothing can be typed', async () => {
    const onOpenSettings = vi.fn()
    await open(liveServer({ pages: [page([])] }), { proc: { busy: true } }, { onOpenSettings })
    expect(within(panel()).getByText(BRIDGE_OFF, { exact: false })).toBeInTheDocument()
    expect(within(panel()).queryByRole('textbox')).toBeNull()
    fireEvent.click(within(panel()).getByRole('button', { name: 'Open Settings' }))
    expect(onOpenSettings).toHaveBeenCalled()
  })

  it('an idle session with the bridge on offers the mark, not a queue', async () => {
    await open(liveServer({ pages: [page([])] }), { reply_bridge: true, proc: { busy: false } })
    expect(screen.queryByRole('textbox')).toBeNull()
    expect(screen.getByRole('button', { name: 'Reply from the app' })).toBeInTheDocument()
  })

  it('the queued text and the typed text are React text and never kept in the browser', async () => {
    const set = vi.spyOn(Storage.prototype, 'setItem')
    const server = liveServer({ pages: [page([])] })
    await open(server, { reply_bridge: true, proc: { busy: true } })
    fireEvent.change(box(), { target: { value: '<b>queued</b>' } })
    fireEvent.keyDown(box(), { key: 'Enter' })
    await screen.findByText('<b>queued</b>')
    expect(document.querySelector('.queue-shown b')).toBeNull()
    expect(set).not.toHaveBeenCalled()
    for (const c of server.calls) expect(c.url).not.toContain('secret-token')
    set.mockRestore()
  })
})

describe('Stop of a terminal session (AC 5)', () => {
  it('one confirm names the project and the short id; only then the stop is posted', async () => {
    const server = liveServer({ pages: [page([])] })
    await open(server, { project: 'alpha' })
    fireEvent.click(within(panel()).getByRole('button', { name: 'Stop' }))
    const confirm = within(panel()).getByRole('group', { name: 'Confirm stop' })
    expect(confirm).toHaveTextContent('Stop session aaaaaaaa in alpha?')
    expect(server.posts('/stop')).toHaveLength(0)
    fireEvent.click(within(confirm).getByRole('button', { name: 'Stop session aaaaaaaa' }))
    expect(await screen.findByText(STOP_SENT)).toBeInTheDocument()
    expect(server.posts('/stop')).toHaveLength(1)
    expect(server.posts('/stop')[0].path).toBe(`/api/live/sessions/${SID}/stop`)
    expect(server.posts('/stop')[0].body).toEqual({ confirmed: true })
  })

  it('"Keep it running" posts nothing', async () => {
    const server = liveServer({ pages: [page([])] })
    await open(server)
    fireEvent.click(within(panel()).getByRole('button', { name: 'Stop' }))
    fireEvent.click(within(panel()).getByRole('button', { name: 'Keep it running' }))
    expect(within(panel()).queryByRole('group', { name: 'Confirm stop' })).toBeNull()
    expect(server.posts('/stop')).toHaveLength(0)
  })

  it('a refusal (409) shows the server\'s one line and offers Stop again', async () => {
    const server = liveServer({ pages: [page([])], stop: () => ({ status: 409, body: { error: 'that process is not a claude session of yours' } }) })
    await open(server)
    fireEvent.click(within(panel()).getByRole('button', { name: 'Stop' }))
    fireEvent.click(within(panel()).getByRole('button', { name: 'Stop session aaaaaaaa' }))
    expect(await screen.findByRole('alert')).toHaveTextContent('that process is not a claude session of yours')
    expect(within(panel()).getByRole('button', { name: 'Stop' })).toBeEnabled()
  })

  it('a stop that was accepted and a session still listed after 5 s says so and offers Force stop; its confirm warns, then posts force: true', async () => {
    vi.useFakeTimers()
    const server = liveServer({ pages: [page([])] })
    await open(server, { project: 'alpha' })
    fireEvent.click(within(panel()).getByRole('button', { name: 'Stop' }))
    await act(async () => {
      fireEvent.click(within(panel()).getByRole('button', { name: 'Stop session aaaaaaaa' }))
    })
    expect(server.posts('/stop')[0].body).toEqual({ confirmed: true })
    expect(screen.getByText(STOP_SENT)).toBeInTheDocument()
    expect(within(panel()).queryByRole('button', { name: 'Force stop' })).toBeNull()
    await tick(FORCE_WAIT_MS - 100)
    expect(within(panel()).queryByRole('button', { name: 'Force stop' })).toBeNull() // not before 5 s
    await tick(200)
    expect(screen.getByText(STILL_RUNNING)).toBeInTheDocument()
    fireEvent.click(within(panel()).getByRole('button', { name: 'Force stop' }))
    const confirm = within(panel()).getByRole('group', { name: 'Confirm force stop' })
    expect(confirm).toHaveTextContent('may leave the last transcript line half written')
    expect(server.posts('/stop')).toHaveLength(1) // nothing more is sent before the confirm
    await act(async () => {
      fireEvent.click(within(confirm).getByRole('button', { name: 'Force stop session aaaaaaaa' }))
    })
    expect(server.posts('/stop')).toHaveLength(2)
    expect(server.posts('/stop')[1].body).toEqual({ confirmed: true, force: true })
  })

  it('a session that leaves the list within 5 s never offers Force stop', async () => {
    vi.useFakeTimers()
    const server = liveServer({ pages: [page([])] })
    await open(server)
    fireEvent.click(within(panel()).getByRole('button', { name: 'Stop' }))
    await act(async () => {
      fireEvent.click(within(panel()).getByRole('button', { name: 'Stop session aaaaaaaa' }))
    })
    act(() => stream().emit('snapshot', snapshot([])))
    await tick(FORCE_WAIT_MS * 2)
    expect(screen.queryByRole('button', { name: 'Force stop' })).toBeNull()
    expect(within(panel()).getByText(/no longer active/)).toBeInTheDocument() // T121 (F6): the panel stays, says so, and drops Stop
    expect(within(panel()).queryByRole('button', { name: 'Stop' })).toBeNull()
  })

  it('"Keep waiting" in the force confirm posts nothing and goes back to Force stop', async () => {
    vi.useFakeTimers()
    const server = liveServer({ pages: [page([])] })
    await open(server)
    fireEvent.click(within(panel()).getByRole('button', { name: 'Stop' }))
    await act(async () => {
      fireEvent.click(within(panel()).getByRole('button', { name: 'Stop session aaaaaaaa' }))
    })
    await tick(FORCE_WAIT_MS + 100)
    fireEvent.click(within(panel()).getByRole('button', { name: 'Force stop' }))
    fireEvent.click(within(panel()).getByRole('button', { name: 'Keep waiting' }))
    expect(within(panel()).getByRole('button', { name: 'Force stop' })).toBeInTheDocument()
    expect(server.posts('/stop')).toHaveLength(1)
  })

  it('Esc while the Stop confirm is open closes the panel and posts nothing', async () => {
    const server = liveServer({ pages: [page([])] })
    await open(server)
    fireEvent.click(within(panel()).getByRole('button', { name: 'Stop' }))
    const keep = within(panel()).getByRole('button', { name: 'Keep it running' })
    keep.focus()
    fireEvent.keyDown(keep, { key: 'Escape' })
    expect(document.getElementById('live-panel')).toBeNull()
    expect(server.posts('/stop')).toHaveLength(0)
  })

  it('a terminal session has Stop and Resume / Fork, and no Interrupt or Close', async () => {
    await open(liveServer({ pages: [page([])] }), {}, { appSessions: true, onResumeInApp: () => {} })
    expect(within(panel()).queryByRole('button', { name: 'Interrupt' })).toBeNull()
    expect(within(panel()).queryByRole('button', { name: 'Close' })).toBeNull()
    expect(within(panel()).getByRole('button', { name: 'Stop' })).toBeInTheDocument()
  })
})

describe('keyboard and phone (AC 7, 8)', () => {
  it('every control of the panel is a native button or box, reachable in order; Back returns the focus to the row', async () => {
    await open(liveServer({ pages: [page([entryTool('t1')])] }), { reply_bridge: true, proc: { busy: true } })
    await within(log()).findByRole('button', { name: /Bash/ })
    const controls = [...panel().querySelectorAll('button, textarea, input, [tabindex="0"]')]
    expect(controls.length).toBeGreaterThan(4)
    expect(panel().querySelectorAll('[tabindex]:not([tabindex="0"]):not([tabindex="-1"])')).toHaveLength(0) // no positive tabindex
    fireEvent.click(within(panel()).getByRole('button', { name: 'Back to sessions' }))
    expect(rowEl()).toHaveFocus()
  })
})
