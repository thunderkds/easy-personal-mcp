import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import LiveView from './LiveView.jsx'
import { TRANSCRIPT_POLL_MS } from './Transcript.jsx'
import { BRIDGE_OFF, QUEUE_NOTE } from './Composer.jsx'
import { STOP_SENT } from './SessionPanel.jsx'
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

  it('moves the focus to the heading when the owner opens it', async () => {
    await open(liveServer())
    expect(within(panel()).getByRole('heading')).toHaveFocus()
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
