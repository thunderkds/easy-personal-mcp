import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { Profiler } from 'react'
import { act, fireEvent, render, screen, within } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import LiveView, { FEED_MAX } from './LiveView.jsx'
import { ago } from './format.js'

// `ago` is called once each time a session row draws: the spy counts row renders (NFR-015).
vi.mock('./format.js', async (importOriginal) => {
  const actual = await importOriginal()
  return { ...actual, ago: vi.fn(actual.ago) }
})

// A hand-driven EventSource: the test decides what the "server" sends and when the link drops.
class FakeEventSource {
  static instances = []
  constructor(url) {
    this.url = url
    this.readyState = 0
    this.listeners = {}
    this.closed = false
    FakeEventSource.instances.push(this)
  }
  addEventListener(type, fn) {
    ;(this.listeners[type] ??= []).push(fn)
  }
  close() {
    this.closed = true
    this.readyState = 2
  }
  emit(type, data, lastEventId = '') {
    const event = { type, data: data === undefined ? undefined : JSON.stringify(data), lastEventId }
    act(() => (this.listeners[type] ?? []).forEach((fn) => fn(event)))
  }
  open() {
    this.readyState = 1
    this.emit('open')
  }
  drop(readyState = 0) {
    this.readyState = readyState // 0 = the browser retries on its own, 2 = it gave up
    this.emit('error')
  }
}
const stream = () => FakeEventSource.instances.at(-1)

const session = (id, over = {}) => ({
  session_id: id, account: 'Claude work', project: 'alpha', last_ts: new Date().toISOString(),
  tool_calls: 0, tokens_in: 0, tokens_out: 0, cost_usd: 0, ...over,
})
const snapshot = (sessions, last_id = 0) => ({ active_window_s: 300, last_id, sessions })
let counter = 0
const live = (sessionId, over = {}) => ({
  id: ++counter, session_id: sessionId, account: 'Claude work', project: 'alpha', ts: new Date().toISOString(),
  kind: 'tool_use', tool: 'Bash', is_error: false, is_sidechain: false, tokens_in: null, tokens_out: null, cost_usd: null, ...over,
})
// Live events reach the page in batches of at most 250 ms: `send` delivers one and lets the batch window pass.
const flush = () => act(() => vi.advanceTimersByTime(250))
const send = (event) => {
  stream().emit('live', event, String(event.id))
  flush()
}
const card = (idPrefix) => screen.getByText(idPrefix, { selector: '.live-id' }).closest('li')
const rowButton = (idPrefix) => card(idPrefix).querySelector('button.live-row')
const rows = () => [...document.querySelectorAll('.live-row')]
// The feed and the totals of one session live in the side panel: these open its row first (a second call keeps it open).
const openRow = (idPrefix) => {
  const button = rowButton(idPrefix)
  if (button.getAttribute('aria-expanded') !== 'true') fireEvent.click(button)
}
const panelOf = (idPrefix) => {
  openRow(idPrefix)
  return within(document.getElementById('live-panel'))
}
const feedOf = (idPrefix) => {
  openRow(idPrefix)
  return [...document.querySelectorAll('#live-panel .live-feed > li')].map((li) => li.textContent)
}

beforeEach(() => {
  counter = 0
  FakeEventSource.instances = []
  ago.mockClear()
  window.location.hash = '#secret-token'
  vi.useFakeTimers()
  vi.stubGlobal('EventSource', FakeEventSource)
})
afterEach(() => {
  vi.useRealTimers()
  vi.unstubAllGlobals()
})

describe('connection', () => {
  it('opens one EventSource on /api/live with the dashboard token as ?token=', () => {
    render(<LiveView />)
    expect(FakeEventSource.instances).toHaveLength(1)
    expect(stream().url).toBe('/api/live?token=secret-token')
  })

  it('closes the stream when the view is left', () => {
    const { unmount } = render(<LiveView />)
    unmount()
    expect(stream().closed).toBe(true)
  })

  it('AC3: a drop shows reconnecting, keeps what is on the page, and open brings it back', () => {
    render(<LiveView />)
    expect(screen.getByRole('status')).toHaveTextContent('Connecting')
    stream().open()
    stream().emit('snapshot', snapshot([session('aaaaaaaa-1111', { tool_calls: 2 })]))
    expect(screen.getByRole('status')).toHaveTextContent('Live')
    stream().drop(0)
    expect(screen.getByRole('status')).toHaveTextContent('Reconnecting')
    expect(card('aaaaaaaa')).toBeInTheDocument()
    stream().open()
    expect(screen.getByRole('status')).toHaveTextContent('Live')
    expect(FakeEventSource.instances).toHaveLength(1) // the browser resumes by itself: one stream, no second one
  })

  it('a stream the browser closed says so and offers a manual reconnect', () => {
    render(<LiveView />)
    stream().drop(2)
    expect(screen.getByRole('status')).toHaveTextContent('closed')
    fireEvent.click(screen.getByRole('button', { name: 'Reconnect' }))
    expect(FakeEventSource.instances).toHaveLength(2)
    expect(screen.getByRole('status')).toHaveTextContent('Connecting')
  })
})

describe('sessions', () => {
  it('shows an empty state with no active session', () => {
    render(<LiveView />)
    stream().open()
    stream().emit('snapshot', snapshot([]))
    expect(screen.getByText(/No active sessions/)).toHaveTextContent('last 5 minutes')
    expect(rows()).toHaveLength(0)
    expect(document.getElementById('live-panel')).toBeNull()
  })

  it('groups sessions by account; a row names its project', () => {
    const at = (s) => new Date(Date.now() - s * 1000).toISOString()
    render(<LiveView />)
    stream().emit('snapshot', snapshot([
      session('aaaaaaaa-1', { account: 'Claude work', project: 'alpha', last_ts: at(5) }),
      session('bbbbbbbb-2', { account: 'Claude work', project: 'beta', last_ts: at(10) }),
      session('cccccccc-3', { account: 'Claude home', project: 'alpha', last_ts: at(5) }),
    ]))
    const accounts = screen.getAllByRole('heading', { level: 2 }).map((h) => h.textContent).filter((t) => t.startsWith('Claude'))
    expect(accounts).toEqual(['Claude home', 'Claude work'])
    const work = screen.getByRole('heading', { level: 2, name: 'Claude work' }).closest('section')
    expect([...work.querySelectorAll('.live-row .live-name')].map((n) => n.textContent)).toEqual(['alpha', 'beta'])
    expect(work.querySelectorAll('.live-row')).toHaveLength(2)
  })

  it('a live event for an unknown session adds it under its own account with its project', () => {
    render(<LiveView />)
    stream().emit('snapshot', snapshot([]))
    send(live('dddddddd-4', { account: 'Claude new', project: 'gamma' }))
    const section = screen.getByRole('heading', { level: 2, name: 'Claude new' }).closest('section')
    expect(within(section).getByText('gamma')).toBeInTheDocument()
    expect(card('dddddddd')).toBeInTheDocument()
  })

  it('marks a session idle once it is older than the active window', () => {
    render(<LiveView />)
    stream().emit('snapshot', snapshot([
      session('aaaaaaaa-1'),
      session('bbbbbbbb-2', { last_ts: new Date(Date.now() - 400_000).toISOString() }),
    ]))
    expect(within(card('aaaaaaaa')).getByText(/active/)).toBeInTheDocument()
    expect(within(card('bbbbbbbb')).getByText(/idle/)).toBeInTheDocument()
  })

  it('truncates a long project name in one line and keeps the full name in a title', () => {
    const long = 'a-very-long-project-folder-name-that-would-break-the-layout-if-it-wrapped'
    render(<LiveView />)
    stream().emit('snapshot', snapshot([session('aaaaaaaa-1', { project: long })]))
    const name = screen.getByText(long)
    expect(name).toHaveAttribute('title', long)
    expect(name).toHaveClass('live-name')
  })

  it('scrolls 60 sessions inside their own box', () => {
    render(<LiveView />)
    stream().emit('snapshot', snapshot(Array.from({ length: 60 }, (_, i) => session(`s${String(i).padStart(7, '0')}-x`, { project: `p${i}` }))))
    expect(rows()).toHaveLength(60)
    expect(rows()[0].closest('.live-sessions')).not.toBeNull()
  })
})

describe('rows (FR-072)', () => {
  const bar = (idPrefix, name) => within(card(idPrefix)).getByRole('img', { name })
  const fill = (el) => el.querySelector('.live-bar-fill')

  it('AC1: three sessions give three rows with project, 8-char id, active/idle text and three bars; no feed is drawn', () => {
    render(<LiveView />)
    stream().emit('snapshot', snapshot([
      session('aaaaaaaa-1', { project: 'alpha', tool_calls: 4, tokens_in: 1000, tokens_out: 200, cost_usd: 0.5 }),
      session('bbbbbbbb-2', { project: 'beta', last_ts: new Date(Date.now() - 400_000).toISOString() }),
      session('cccccccc-3', { project: 'gamma' }),
    ]))
    expect(rows()).toHaveLength(3)
    const first = rows().find((r) => r.textContent.includes('aaaaaaaa'))
    expect(first).toHaveTextContent('alpha')
    expect(first).toHaveTextContent(/active/)
    expect(first.querySelector('.live-id')).toHaveAttribute('title', 'aaaaaaaa-1')
    expect(rows().find((r) => r.textContent.includes('bbbbbbbb'))).toHaveTextContent(/idle/)
    for (const id of ['aaaaaaaa', 'bbbbbbbb', 'cccccccc']) expect(card(id).querySelectorAll('.live-bar')).toHaveLength(3)
    expect(document.querySelectorAll('.live-feed')).toHaveLength(0)
  })

  it('AC2: the largest session has a full bar for each measure, half of it has half a bar, and each bar is named with its number', () => {
    render(<LiveView />)
    stream().emit('snapshot', snapshot([
      session('aaaaaaaa-1', { tool_calls: 10, tokens_in: 1_000_000, tokens_out: 200_000, cost_usd: 2 }),
      session('bbbbbbbb-2', { project: 'beta', tool_calls: 5, tokens_in: 500_000, tokens_out: 100_000, cost_usd: 1 }),
      session('cccccccc-3', { project: 'gamma', tool_calls: 0 }),
    ]))
    expect(fill(bar('aaaaaaaa', 'tokens 1.2M')).style.width).toBe('100%')
    expect(fill(bar('aaaaaaaa', 'cost $2.00')).style.width).toBe('100%')
    expect(fill(bar('aaaaaaaa', 'tool calls 10')).style.width).toBe('100%')
    expect(fill(bar('bbbbbbbb', 'tokens 600K')).style.width).toBe('50%')
    expect(fill(bar('bbbbbbbb', 'cost $1.00')).style.width).toBe('50%')
    expect(fill(bar('bbbbbbbb', 'tool calls 5')).style.width).toBe('50%')
    expect(fill(bar('cccccccc', 'tokens 0')).style.width).toBe('0%')
    // the number is also drawn beside the bar
    expect(card('aaaaaaaa')).toHaveTextContent('1.2M')
  })

  it('AC2: a live event moves the bars: the scale follows the largest session', () => {
    render(<LiveView />)
    stream().emit('snapshot', snapshot([
      session('aaaaaaaa-1', { tool_calls: 10 }),
      session('bbbbbbbb-2', { project: 'beta', tool_calls: 5 }),
    ]))
    expect(fill(bar('bbbbbbbb', 'tool calls 5')).style.width).toBe('50%')
    for (let i = 0; i < 10; i++) send(live('bbbbbbbb-2'))
    expect(fill(bar('bbbbbbbb', 'tool calls 15')).style.width).toBe('100%')
    expect(fill(bar('aaaaaaaa', 'tool calls 10')).style.width).toBe('66.7%')
  })

  it('AC3: an unknown price says "unknown price" and draws no cost bar; a priced $0 draws an empty bar', () => {
    render(<LiveView />)
    stream().emit('snapshot', snapshot([
      session('aaaaaaaa-1', { tokens_in: 100, tokens_out: 50, cost_usd: null }),
      session('bbbbbbbb-2', { project: 'beta', tokens_in: 100, tokens_out: 50, cost_usd: 0 }),
      session('cccccccc-3', { project: 'gamma', tokens_in: 100, tokens_out: 50, cost_usd: 3 }),
    ]))
    const unknown = bar('aaaaaaaa', 'cost unknown price')
    expect(unknown).toHaveTextContent('unknown price')
    expect(unknown).not.toHaveTextContent('$0')
    expect(fill(unknown)).toBeNull()
    expect(unknown.querySelector('.live-bar-track')).not.toBeNull()
    const zero = bar('bbbbbbbb', 'cost $0.00')
    expect(fill(zero).style.width).toBe('0%')
    expect(fill(bar('cccccccc', 'cost $3.00')).style.width).toBe('100%')
  })

  it('an unknown price next to a known part still shows the known part, named', () => {
    render(<LiveView />)
    stream().emit('snapshot', snapshot([session('aaaaaaaa-1')]))
    send(live('aaaaaaaa-1', { kind: 'usage', tool: null, tokens_in: 100, tokens_out: 50, cost_usd: null }))
    send(live('aaaaaaaa-1', { kind: 'usage', tool: null, tokens_in: 1, tokens_out: 1, cost_usd: 0.4 }))
    const cost = bar('aaaaaaaa', 'cost $0.40 + unknown price')
    expect(fill(cost).style.width).toBe('100%')
  })

  it('all measures at zero give empty bars, never NaN', () => {
    render(<LiveView />)
    stream().emit('snapshot', snapshot([session('aaaaaaaa-1'), session('bbbbbbbb-2', { project: 'beta' })]))
    const widths = [...document.querySelectorAll('.live-bar-fill')].map((f) => f.style.width)
    expect(widths).toHaveLength(6)
    expect(new Set(widths)).toEqual(new Set(['0%']))
  })

  it('AC4: a row is a button that opens its panel (Enter and Space click a native button); aria-expanded and aria-controls follow', () => {
    render(<LiveView />)
    stream().emit('snapshot', snapshot([session('aaaaaaaa-1', { project: 'alpha' }), session('bbbbbbbb-2', { project: 'beta' })]))
    const a = rowButton('aaaaaaaa')
    const b = rowButton('bbbbbbbb')
    expect(a.tagName).toBe('BUTTON')
    expect(a).toHaveAttribute('type', 'button')
    expect(a).toHaveAttribute('aria-expanded', 'false')
    expect(a).toHaveAttribute('aria-controls', 'live-panel')
    expect(document.getElementById('live-panel')).toBeNull()

    fireEvent.click(a)
    expect(a).toHaveAttribute('aria-expanded', 'true')
    const panel = screen.getByRole('region', { name: /Session aaaaaaaa/ })
    expect(panel).toBe(document.getElementById('live-panel'))
    expect(within(panel).getByText('Tool calls')).toBeInTheDocument() // the session's Totals
    expect(within(panel).getByRole('list', { name: /Tool calls, newest first/ })).toBeInTheDocument() // and its Feed

    fireEvent.click(b) // another row closes the first
    expect(a).toHaveAttribute('aria-expanded', 'false')
    expect(b).toHaveAttribute('aria-expanded', 'true')
    expect(document.querySelectorAll('.live-panel')).toHaveLength(1)
    expect(screen.getByRole('region', { name: /Session bbbbbbbb/ })).toBeInTheDocument()

    fireEvent.click(screen.getByRole('button', { name: 'Close' }))
    expect(document.getElementById('live-panel')).toBeNull()
    expect(b).toHaveAttribute('aria-expanded', 'false')
    expect(b).toHaveFocus() // the keyboard stays where it was
  })

  it('AC4: the panel feed is newest first and holds at most FEED_MAX items', () => {
    render(<LiveView />)
    stream().emit('snapshot', snapshot([session('aaaaaaaa-1')]))
    for (let i = 0; i < FEED_MAX + 5; i++) stream().emit('live', live('aaaaaaaa-1', { tool: `T${i}` }), String(i + 1))
    flush()
    const items = feedOf('aaaaaaaa')
    expect(items).toHaveLength(FEED_MAX)
    expect(items[0]).toContain(`T${FEED_MAX + 4}`)
  })

  it('AC5: live events for the open session appear in its panel; other sessions do not', () => {
    render(<LiveView />)
    stream().emit('snapshot', snapshot([session('aaaaaaaa-1'), session('bbbbbbbb-2', { project: 'beta' })]))
    openRow('aaaaaaaa')
    send(live('aaaaaaaa-1', { tool: 'Edit' }))
    send(live('bbbbbbbb-2', { tool: 'Grep' }))
    const items = [...document.querySelectorAll('#live-panel .live-feed > li')].map((li) => li.textContent)
    expect(items).toHaveLength(1)
    expect(items[0]).toContain('Edit')
  })

  it('AC5: when the open session leaves the list the panel says so, and a later snapshot that lists it brings it back', () => {
    render(<LiveView />)
    stream().emit('snapshot', snapshot([session('aaaaaaaa-1'), session('bbbbbbbb-2', { project: 'beta' })], 1))
    openRow('aaaaaaaa')
    stream().emit('snapshot', snapshot([session('bbbbbbbb-2', { project: 'beta' })], 2))
    const panel = document.getElementById('live-panel')
    expect(panel).toHaveTextContent('This session is no longer active.')
    expect(document.querySelectorAll('.live-feed')).toHaveLength(0)
    expect(within(panel).getByRole('button', { name: 'Close' })).toBeInTheDocument()
    stream().emit('snapshot', snapshot([session('aaaaaaaa-1'), session('bbbbbbbb-2', { project: 'beta' })], 3))
    expect(document.getElementById('live-panel')).not.toHaveTextContent('no longer active')
    fireEvent.click(within(document.getElementById('live-panel')).getByRole('button', { name: 'Close' }))
    expect(document.getElementById('live-panel')).toBeNull()
  })

  it('AC5: the last session leaving still shows the panel notice', () => {
    render(<LiveView />)
    stream().emit('snapshot', snapshot([session('aaaaaaaa-1')], 1))
    openRow('aaaaaaaa')
    stream().emit('snapshot', snapshot([], 2))
    expect(document.getElementById('live-panel')).toHaveTextContent('This session is no longer active.')
    expect(screen.getByText(/No active sessions/)).toBeInTheDocument()
  })

  it('a dropped and resumed stream keeps the panel on the same session, feed included', () => {
    render(<LiveView />)
    stream().open()
    stream().emit('snapshot', snapshot([session('aaaaaaaa-1'), session('bbbbbbbb-2', { project: 'beta' })]))
    openRow('bbbbbbbb')
    send(live('bbbbbbbb-2', { tool: 'Read' }))
    stream().drop(0)
    stream().open()
    expect(screen.getByRole('region', { name: /Session bbbbbbbb/ })).toBeInTheDocument()
    expect(feedOf('bbbbbbbb')).toHaveLength(1)
  })

  it('AC7: 100 live events within 250 ms are applied in one batch; a row without an event in it does not draw again', () => {
    const commits = []
    render(
      <Profiler id="live" onRender={() => commits.push(1)}>
        <LiveView />
      </Profiler>,
    )
    const quiet = new Date(Date.now() - 60_000).toISOString()
    stream().emit('snapshot', snapshot([session('aaaaaaaa-1'), session('bbbbbbbb-2', { project: 'beta', last_ts: quiet })]))
    const drawn = commits.length
    ago.mockClear()
    for (let i = 0; i < 100; i++) stream().emit('live', live('aaaaaaaa-1'), String(i + 1))
    act(() => vi.advanceTimersByTime(249))
    expect(commits.length).toBe(drawn) // still waiting: nothing reached the page yet
    act(() => vi.advanceTimersByTime(1))
    expect(commits.length - drawn).toBeLessThanOrEqual(2)
    expect(commits.length - drawn).toBeGreaterThan(0)
    expect(panelOf('aaaaaaaa').getByText('Tool calls').nextSibling).toHaveTextContent('100')
    const rowsDrawn = ago.mock.calls.map(([ts]) => ts)
    expect(rowsDrawn.length).toBeGreaterThan(0)
    expect(rowsDrawn).not.toContain(quiet) // session b had no event in the batch
  })

  it('AC7: a snapshot that repeats a session unchanged does not draw its row again', () => {
    render(<LiveView />)
    const quiet = new Date(Date.now() - 60_000).toISOString()
    const a = session('aaaaaaaa-1', { tool_calls: 1 })
    const b = session('bbbbbbbb-2', { project: 'beta', last_ts: quiet })
    stream().emit('snapshot', snapshot([a, b], 1))
    ago.mockClear()
    stream().emit('snapshot', snapshot([{ ...a, tool_calls: 2 }, { ...b }], 2))
    const rowsDrawn = ago.mock.calls.map(([ts]) => ts)
    expect(rowsDrawn).toContain(a.last_ts)
    expect(rowsDrawn).not.toContain(quiet)
  })

  it('AC7: a snapshot flushes the events queued before it, so order holds', () => {
    render(<LiveView />)
    stream().emit('snapshot', snapshot([session('aaaaaaaa-1')], 0))
    stream().emit('live', live('aaaaaaaa-1', { tool: 'Old' }), '1')
    stream().emit('snapshot', snapshot([session('aaaaaaaa-1', { tool_calls: 1 })], 1)) // covers event 1
    flush()
    expect(panelOf('aaaaaaaa').getByText('Tool calls').nextSibling).toHaveTextContent('1')
  })

  it('AC7: leaving the view drops events still waiting for their batch', () => {
    const { unmount } = render(<LiveView />)
    stream().emit('snapshot', snapshot([session('aaaaaaaa-1')]))
    stream().emit('live', live('aaaaaaaa-1'), '1')
    unmount()
    expect(vi.getTimerCount()).toBe(0)
  })

  it('AC8: at most one .live-feed list is in the DOM, and only for the open session', () => {
    render(<LiveView />)
    stream().emit('snapshot', snapshot([session('aaaaaaaa-1'), session('bbbbbbbb-2', { project: 'beta' })]))
    expect(document.querySelectorAll('.live-feed')).toHaveLength(0)
    fireEvent.click(rows()[0])
    expect(document.querySelectorAll('.live-feed')).toHaveLength(1)
    fireEvent.click(rows()[1])
    expect(document.querySelectorAll('.live-feed')).toHaveLength(1)
    expect(document.querySelectorAll('[aria-live]')).toHaveLength(1) // one polite region, not one per session
    fireEvent.click(rows()[1])
    expect(document.querySelectorAll('.live-feed')).toHaveLength(0)
  })

  it('the overall totals stay when a panel is open', () => {
    render(<LiveView />)
    stream().emit('snapshot', snapshot([session('aaaaaaaa-1', { tool_calls: 3 }), session('bbbbbbbb-2', { project: 'beta', tool_calls: 4 })]))
    openRow('aaaaaaaa')
    const total = screen.getByRole('region', { name: 'All sessions' })
    expect(within(total).getByText('Tool calls').nextSibling).toHaveTextContent('7')
  })
})

describe('feed', () => {
  it('AC1: new tool calls land in the right session, newest first', () => {
    render(<LiveView />)
    stream().emit('snapshot', snapshot([session('aaaaaaaa-1'), session('bbbbbbbb-2', { project: 'beta' })]))
    send(live('aaaaaaaa-1', { tool: 'Read' }))
    send(live('bbbbbbbb-2', { tool: 'Grep' }))
    send(live('aaaaaaaa-1', { tool: 'Bash' }))
    const a = feedOf('aaaaaaaa')
    expect(a).toHaveLength(2)
    expect(a[0]).toContain('Bash')
    expect(a[1]).toContain('Read')
    expect(feedOf('bbbbbbbb')).toHaveLength(1)
    expect(feedOf('bbbbbbbb')[0]).toContain('Grep')
  })

  it('AC1: the feed holds at most FEED_MAX items and drops the oldest', () => {
    render(<LiveView />)
    stream().emit('snapshot', snapshot([session('aaaaaaaa-1')]))
    for (let i = 0; i < FEED_MAX + 5; i++) send(live('aaaaaaaa-1', { tool: `T${i}` }))
    const items = feedOf('aaaaaaaa')
    expect(items).toHaveLength(FEED_MAX)
    expect(items[0]).toContain(`T${FEED_MAX + 4}`)
    expect(items.at(-1)).toContain('T5')
    expect(items.some((t) => /T[0-4](?!\d)/.test(t))).toBe(false)
  })

  it('shows an error result and an agent (sidechain) call, and counts only tool_use as a call', () => {
    render(<LiveView />)
    stream().emit('snapshot', snapshot([session('aaaaaaaa-1')]))
    send(live('aaaaaaaa-1', { tool: 'Task', is_sidechain: true }))
    send(live('aaaaaaaa-1', { kind: 'tool_result', tool: 'Bash', is_error: true }))
    send(live('aaaaaaaa-1', { kind: 'tool_result', tool: 'Read', is_error: false }))
    send(live('aaaaaaaa-1', { kind: 'usage', tool: null, tokens_in: 5, tokens_out: 5, cost_usd: 0.01 }))
    const items = feedOf('aaaaaaaa')
    expect(items).toHaveLength(2) // the tool_use and the failed result; a good result and a usage record are totals only
    expect(items[0]).toMatch(/Bash/)
    expect(items[0]).toMatch(/error/)
    expect(items[1]).toMatch(/Task/)
    expect(items[1]).toMatch(/agent/)
    expect(panelOf('aaaaaaaa').getByText('Tool calls').nextSibling).toHaveTextContent('1')
  })

  it('says the feed only holds calls seen since the page opened (the snapshot carries no history)', () => {
    render(<LiveView />)
    stream().emit('snapshot', snapshot([session('aaaaaaaa-1', { tool_calls: 7 })]))
    expect(panelOf('aaaaaaaa').getByText('No tool calls seen since this page opened.')).toBeInTheDocument()
  })

  it('announces new items politely and labels the feed', () => {
    render(<LiveView />)
    stream().emit('snapshot', snapshot([session('aaaaaaaa-1')]))
    const feed = panelOf('aaaaaaaa').getByRole('list', { name: /Tool calls, newest first/ })
    expect(feed).toHaveAttribute('aria-live', 'polite')
  })

  it('ignores an event it has already seen (same id)', () => {
    render(<LiveView />)
    stream().emit('snapshot', snapshot([session('aaaaaaaa-1')]))
    const event = live('aaaaaaaa-1', { tokens_in: 10, tokens_out: 0, cost_usd: 1 })
    send(event)
    send(event)
    expect(feedOf('aaaaaaaa')).toHaveLength(1)
    expect(panelOf('aaaaaaaa').getByText('Tokens in').nextSibling).toHaveTextContent('10')
  })

  it('the same event twice inside one batch is counted once', () => {
    render(<LiveView />)
    stream().emit('snapshot', snapshot([session('aaaaaaaa-1')]))
    const event = live('aaaaaaaa-1')
    stream().emit('live', event, String(event.id))
    stream().emit('live', event, String(event.id))
    flush()
    expect(feedOf('aaaaaaaa')).toHaveLength(1)
  })

  it('a malformed message is ignored', () => {
    render(<LiveView />)
    stream().emit('snapshot', snapshot([session('aaaaaaaa-1')]))
    act(() => stream().listeners.live.forEach((fn) => fn({ data: '{not json', lastEventId: '9' })))
    flush()
    expect(card('aaaaaaaa')).toBeInTheDocument()
  })
})

describe('totals', () => {
  it('AC2: per-session and overall totals follow the token and cost fields', () => {
    render(<LiveView />)
    stream().emit('snapshot', snapshot([
      session('aaaaaaaa-1', { tool_calls: 3, tokens_in: 1000, tokens_out: 200, cost_usd: 0.5 }),
      session('bbbbbbbb-2', { project: 'beta', tool_calls: 1, tokens_in: 10, tokens_out: 5, cost_usd: 0.25 }),
    ]))
    const total = screen.getByRole('region', { name: 'All sessions' })
    expect(within(total).getByText('Tool calls').nextSibling).toHaveTextContent('4')
    expect(within(total).getByText('Tokens in').nextSibling).toHaveTextContent('1,010')
    expect(within(total).getByText('Tokens out').nextSibling).toHaveTextContent('205')
    expect(within(total).getByText('API-equivalent cost').nextSibling).toHaveTextContent('$0.75')

    send(live('aaaaaaaa-1', { kind: 'usage', tool: null, tokens_in: 100, tokens_out: 50, cost_usd: 0.1 }))
    send(live('aaaaaaaa-1', { tool: 'Read' }))
    const a = panelOf('aaaaaaaa')
    expect(a.getByText('Tool calls').nextSibling).toHaveTextContent('4')
    expect(a.getByText('Tokens in').nextSibling).toHaveTextContent('1,100')
    expect(a.getByText('Tokens out').nextSibling).toHaveTextContent('250')
    expect(a.getByText('API-equivalent cost').nextSibling).toHaveTextContent('$0.60')
    expect(panelOf('bbbbbbbb').getByText('API-equivalent cost').nextSibling).toHaveTextContent('$0.25')
    expect(within(total).getByText('Tool calls').nextSibling).toHaveTextContent('5')
    expect(within(total).getByText('Tokens in').nextSibling).toHaveTextContent('1,110')
    expect(within(total).getByText('API-equivalent cost').nextSibling).toHaveTextContent('$0.85')
  })

  it('labels the cost as API-equivalent and not what a subscription pays', () => {
    render(<LiveView />)
    stream().emit('snapshot', snapshot([session('aaaaaaaa-1')]))
    expect(screen.getByText(/tokens times the API price table/)).toHaveTextContent('not what a subscription pays')
  })

  it('a price that is unknown is named, never shown as $0', () => {
    render(<LiveView />)
    stream().emit('snapshot', snapshot([session('aaaaaaaa-1')]))
    send(live('aaaaaaaa-1', { kind: 'usage', tool: null, tokens_in: 100, tokens_out: 50, cost_usd: null }))
    const text = panelOf('aaaaaaaa').getByText('API-equivalent cost').nextSibling.textContent
    expect(text).toContain('abstained')
    expect(text).not.toContain('$0')
    send(live('aaaaaaaa-1', { kind: 'usage', tool: null, tokens_in: 1, tokens_out: 1, cost_usd: 0.4 }))
    expect(panelOf('aaaaaaaa').getByText('API-equivalent cost').nextSibling.textContent).toMatch(/^\$0\.40 \+ abstained/)
  })

  it('a fresh snapshot replaces the totals and keeps the feed of a session it still lists', () => {
    render(<LiveView />)
    stream().emit('snapshot', snapshot([session('aaaaaaaa-1')]))
    send(live('aaaaaaaa-1', { tool: 'Read', tokens_in: 5, tokens_out: 5, cost_usd: 0.05 }))
    stream().emit('snapshot', snapshot([session('aaaaaaaa-1', { tool_calls: 9, tokens_in: 900, tokens_out: 100, cost_usd: 2 })], 1)) // the snapshot covers event 1
    expect(feedOf('aaaaaaaa')).toHaveLength(1)
    expect(panelOf('aaaaaaaa').getByText('Tool calls').nextSibling).toHaveTextContent('9')
    expect(panelOf('aaaaaaaa').getByText('API-equivalent cost').nextSibling).toHaveTextContent('$2.00')
    send(live('aaaaaaaa-1', { tool: 'Edit' }))
    expect(feedOf('aaaaaaaa')).toHaveLength(2)
  })
})

describe('resume after a snapshot (last_id)', () => {
  it('an event with id <= snapshot.last_id is already in the totals and is not counted again', () => {
    render(<LiveView />)
    stream().emit('snapshot', snapshot([session('aaaaaaaa-1', { tool_calls: 4, tokens_in: 100, tokens_out: 0, cost_usd: 1 })], 10))
    send(live('aaaaaaaa-1', { id: 9, tool: 'Old', tokens_in: 50, tokens_out: 0, cost_usd: 0.5 }))
    send(live('aaaaaaaa-1', { id: 10, tool: 'Old2', tokens_in: 50, tokens_out: 0, cost_usd: 0.5 }))
    const a = panelOf('aaaaaaaa')
    expect(a.getByText('Tool calls').nextSibling).toHaveTextContent('4')
    expect(a.getByText('Tokens in').nextSibling).toHaveTextContent('100')
    expect(a.getByText('API-equivalent cost').nextSibling).toHaveTextContent('$1.00')
    expect(screen.queryByText('Old')).toBeNull()
  })

  it('an event with a later id is counted once', () => {
    render(<LiveView />)
    stream().emit('snapshot', snapshot([session('aaaaaaaa-1', { tool_calls: 4, tokens_in: 100, tokens_out: 0, cost_usd: 1 })], 10))
    const event = live('aaaaaaaa-1', { id: 11, tool: 'New', tokens_in: 50, tokens_out: 0, cost_usd: 0.5 })
    send(event)
    send(event)
    const a = panelOf('aaaaaaaa')
    expect(a.getByText('Tool calls').nextSibling).toHaveTextContent('5')
    expect(a.getByText('Tokens in').nextSibling).toHaveTextContent('150')
    expect(a.getByText('API-equivalent cost').nextSibling).toHaveTextContent('$1.50')
    expect(feedOf('aaaaaaaa')).toHaveLength(1)
  })

  it('a second snapshot moves the line to its own last_id', () => {
    render(<LiveView />)
    stream().emit('snapshot', snapshot([session('aaaaaaaa-1')], 5))
    send(live('aaaaaaaa-1', { id: 6 }))
    stream().emit('snapshot', snapshot([session('aaaaaaaa-1', { tool_calls: 8 })], 20))
    send(live('aaaaaaaa-1', { id: 7 })) // covered by the new snapshot
    send(live('aaaaaaaa-1', { id: 21 }))
    expect(panelOf('aaaaaaaa').getByText('Tool calls').nextSibling).toHaveTextContent('9')
  })
})

describe('server restart', () => {
  it('a snapshot with a lower last_id means ids restarted: old feed rows go, new ids are counted', () => {
    render(<LiveView />)
    stream().emit('snapshot', snapshot([session('aaaaaaaa-1')], 5))
    send(live('aaaaaaaa-1', { id: 6, tool: 'Before' }))
    expect(feedOf('aaaaaaaa')[0]).toContain('Before') // the panel is open on it
    stream().emit('snapshot', snapshot([session('aaaaaaaa-1')], 1))
    expect(screen.queryByText('Before')).toBeNull()
    send(live('aaaaaaaa-1', { id: 2, tool: 'After' }))
    expect(feedOf('aaaaaaaa')).toHaveLength(1)
    expect(feedOf('aaaaaaaa')[0]).toContain('After')
  })
})

describe('no content', () => {
  it('shows only metadata even if the server sent extra fields', () => {
    render(<LiveView />)
    stream().emit('snapshot', snapshot([session('aaaaaaaa-1')]))
    send({ ...live('aaaaaaaa-1', { tool: 'Bash' }), input: { command: 'cat SECRET-planted' }, text: 'SECRET-planted body' })
    expect(document.body.textContent).not.toContain('SECRET')
    openRow('aaaaaaaa') // and not in the panel either
    expect(document.body.textContent).not.toContain('SECRET')
  })
})

describe('attention (FR-050)', () => {
  const failing = { state: 'failing', tool: 'Edit', count: 3, is_sidechain: true, since: 't' }
  const waiting = { state: 'waiting', tool: 'AskUserQuestion', count: null, is_sidechain: false, since: 't' }
  const stuck = { state: 'needs_attention', tool: 'Bash', count: null, is_sidechain: false, since: 't' }
  const pills = (idPrefix) => [...card(idPrefix).querySelectorAll('.live-pill')].map((p) => p.textContent)
  const order = () => [...document.querySelectorAll('.live-id')].map((h) => h.textContent)

  it('AC1: a failing state in a subagent stream renders the tool, the count and "agent" in the row button', () => {
    render(<LiveView />)
    stream().emit('snapshot', snapshot([session('aaaaaaaa-1', { attention: [failing] })]))
    expect(pills('aaaaaaaa')).toEqual(['failing: Edit ×3 · agent'])
    expect(card('aaaaaaaa').querySelector('button.live-row .live-pill')).not.toBeNull()
  })

  it('AC2: waiting and needs attention are worded as the guide says', () => {
    render(<LiveView />)
    stream().emit('snapshot', snapshot([
      session('aaaaaaaa-1', { attention: [waiting] }),
      session('bbbbbbbb-2', { project: 'beta', attention: [stuck] }),
    ]))
    expect(pills('aaaaaaaa')).toEqual(['waiting for you: question'])
    expect(pills('bbbbbbbb')).toEqual(['needs attention: Bash · hung or waiting for permission'])
  })

  it('AC2: an unknown tool reads "unknown tool"; a failing streak of mixed tools reads "N in a row"', () => {
    render(<LiveView />)
    stream().emit('snapshot', snapshot([
      session('aaaaaaaa-1', { attention: [{ ...stuck, tool: null }] }),
      session('bbbbbbbb-2', { project: 'beta', attention: [{ ...failing, tool: null, count: 3, is_sidechain: false }] }),
    ]))
    expect(pills('aaaaaaaa')).toEqual(['needs attention: unknown tool · hung or waiting for permission'])
    expect(pills('bbbbbbbb')).toEqual(['failing: 3 in a row'])
  })

  it('two streams failing in one session (main + subagent) give two pills and no duplicate React key', () => {
    const errors = vi.spyOn(console, 'error').mockImplementation(() => {})
    render(<LiveView />)
    stream().emit('snapshot', snapshot([session('aaaaaaaa-1', { attention: [{ ...failing, is_sidechain: false, tool: 'Bash' }, failing] })]))
    expect(pills('aaaaaaaa')).toEqual(['failing: Bash ×3', 'failing: Edit ×3 · agent'])
    expect(errors.mock.calls.flat().join(' ')).not.toMatch(/same key/)
    errors.mockRestore()
  })

  it('several states on one session give one pill each, failing first; the border takes the first state', () => {
    render(<LiveView />)
    stream().emit('snapshot', snapshot([session('aaaaaaaa-1', { attention: [stuck, waiting, { ...failing, is_sidechain: false }] })]))
    expect(pills('aaaaaaaa')).toEqual([
      'failing: Edit ×3',
      'waiting for you: question',
      'needs attention: Bash · hung or waiting for permission',
    ])
    expect(card('aaaaaaaa')).toHaveAttribute('data-state', 'failing')
  })

  it('the border state is on the row, one value per state, and absent without a state', () => {
    render(<LiveView />)
    stream().emit('snapshot', snapshot([
      session('aaaaaaaa-1', { attention: [waiting] }),
      session('bbbbbbbb-2', { project: 'beta', attention: [stuck] }),
      session('cccccccc-3', { project: 'gamma' }),
    ]))
    expect(card('aaaaaaaa')).toHaveAttribute('data-state', 'waiting')
    expect(card('bbbbbbbb')).toHaveAttribute('data-state', 'needs_attention')
    expect(card('cccccccc')).not.toHaveAttribute('data-state')
    expect(pills('cccccccc')).toEqual([])
  })

  it('a long tool name is cut by CSS but the whole text is in the title', () => {
    const tool = 'mcp__claude_ai_Linear__save_issue'
    render(<LiveView />)
    stream().emit('snapshot', snapshot([session('aaaaaaaa-1', { attention: [{ ...stuck, tool }] })]))
    const pill = card('aaaaaaaa').querySelector('.live-pill')
    expect(pill).toHaveAttribute('title', `needs attention: ${tool} · hung or waiting for permission`)
  })

  it('AC3: a re-sent snapshot without the state removes the pill and border, and keeps the feed', () => {
    render(<LiveView />)
    stream().emit('snapshot', snapshot([session('aaaaaaaa-1', { attention: [failing] })], 1))
    send(live('aaaaaaaa-1', { id: 2, tool: 'Read' }))
    stream().emit('snapshot', snapshot([session('aaaaaaaa-1')], 2))
    expect(pills('aaaaaaaa')).toEqual([])
    expect(card('aaaaaaaa')).not.toHaveAttribute('data-state')
    expect(feedOf('aaaaaaaa')).toHaveLength(1)
    expect(screen.queryByText(/need you/)).toBeNull()
  })

  it('a live event does not change the states of a session', () => {
    render(<LiveView />)
    stream().emit('snapshot', snapshot([session('aaaaaaaa-1', { attention: [failing] }), session('bbbbbbbb-2', { project: 'beta' })]))
    send(live('aaaaaaaa-1', { tool: 'Read' }))
    send(live('bbbbbbbb-2', { kind: 'tool_result', tool: 'Bash', is_error: true }))
    expect(pills('aaaaaaaa')).toEqual(['failing: Edit ×3 · agent'])
    expect(pills('bbbbbbbb')).toEqual([])
  })

  it('AC4: the header counts sessions with any state in a status region, and hides it at zero', () => {
    render(<LiveView />)
    stream().emit('snapshot', snapshot([
      session('aaaaaaaa-1', { attention: [failing, waiting] }), // two states, one session
      session('bbbbbbbb-2', { project: 'beta', attention: [stuck] }),
      session('cccccccc-3', { project: 'gamma' }),
    ]))
    const count = screen.getByText('2 sessions need you')
    expect(count.closest('[role="status"]')).not.toBeNull()
    stream().emit('snapshot', snapshot([session('aaaaaaaa-1', { attention: [failing] })]))
    expect(screen.getByText('1 session needs you').closest('[role="status"]')).not.toBeNull()
    stream().emit('snapshot', snapshot([session('aaaaaaaa-1')]))
    expect(screen.queryByText(/need/)).toBeNull()
    expect(screen.getAllByRole('status')).toHaveLength(1) // only the link status is left
  })

  it('AC5: in an account a row with a state comes first; ties keep the newest first', () => {
    const at = (s) => new Date(Date.now() - s * 1000).toISOString()
    render(<LiveView />)
    stream().emit('snapshot', snapshot([
      session('newest00-1', { last_ts: at(1) }),
      session('older000-2', { last_ts: at(30), attention: [waiting] }),
      session('oldest00-3', { last_ts: at(60), attention: [stuck] }),
      session('middle00-4', { last_ts: at(10) }),
      session('other000-5', { project: 'beta', last_ts: at(5) }),
    ]))
    expect(order()).toEqual(['older000', 'oldest00', 'newest00', 'other000', 'middle00'])
  })

  it('AC5: a row that needs you moves to the top when a snapshot says so, in the account it belongs to', () => {
    const at = (s) => new Date(Date.now() - s * 1000).toISOString()
    render(<LiveView />)
    stream().emit('snapshot', snapshot([
      session('newest00-1', { last_ts: at(1) }),
      session('older000-2', { last_ts: at(30) }),
      session('home0000-3', { account: 'Claude home', last_ts: at(1) }),
    ]))
    expect(order()).toEqual(['home0000', 'newest00', 'older000'])
    stream().emit('snapshot', snapshot([
      session('newest00-1', { last_ts: at(1) }),
      session('older000-2', { last_ts: at(30), attention: [waiting] }),
      session('home0000-3', { account: 'Claude home', last_ts: at(1) }),
    ], 1))
    expect(order()).toEqual(['home0000', 'older000', 'newest00'])
  })

  it('AC6: a session with a state is never shown as idle, even when it is quiet', () => {
    const quiet = new Date(Date.now() - 3_000_000).toISOString()
    render(<LiveView />)
    stream().emit('snapshot', snapshot([
      session('aaaaaaaa-1', { last_ts: quiet, attention: [stuck] }),
      session('bbbbbbbb-2', { project: 'beta', last_ts: quiet }),
    ]))
    expect(within(card('aaaaaaaa')).queryByText(/idle/)).toBeNull()
    expect(within(card('aaaaaaaa')).getByText(/active/)).toBeInTheDocument()
    expect(card('aaaaaaaa')).not.toHaveAttribute('data-idle')
    expect(within(card('bbbbbbbb')).getByText(/idle/)).toBeInTheDocument()
  })

  it('AC7: a failed result row shows its tool name, not tool_result', () => {
    render(<LiveView />)
    stream().emit('snapshot', snapshot([session('aaaaaaaa-1')]))
    send(live('aaaaaaaa-1', { kind: 'tool_result', tool: 'Edit', is_error: true, is_sidechain: true }))
    expect(feedOf('aaaaaaaa')[0]).toMatch(/Edit.*agent.*error/)
    expect(feedOf('aaaaaaaa')[0]).not.toContain('tool_result')
  })

  it('shows no content even if the server sent extra attention fields', () => {
    render(<LiveView />)
    stream().emit('snapshot', snapshot([session('aaaaaaaa-1', { attention: [{ ...failing, input: 'SECRET-planted', text: 'SECRET-planted' }] })]))
    expect(document.body.textContent).not.toContain('SECRET')
  })
})

describe('design', () => {
  const css = readFileSync(resolve(process.cwd(), 'src/app.css'), 'utf8')
  const liveCss = css.split('\n').filter((line) => /^[^/]*\.live-/.test(line)).join('\n')

  it('Live rules in app.css use only var(--…) colors', () => {
    expect(liveCss).not.toBe('')
    expect(liveCss).not.toMatch(/#[0-9a-fA-F]{3,8}\b/)
    expect(liveCss).not.toMatch(/\b(rgb|rgba|hsl|hsla)\(/)
  })

  it('colors the pill and the row border from tokens only, one rule per state, and never by color alone', () => {
    for (const state of ['failing', 'waiting', 'needs_attention']) {
      expect(liveCss).toMatch(new RegExp(`\\.live-pill\\[data-state='${state}'\\][^\\n]*color: var\\(--color-[a-z-]+\\)`))
      expect(liveCss).toMatch(new RegExp(`\\.live-session\\[data-state='${state}'\\][^\\n]*border-color: var\\(--color-[a-z-]+\\)`))
    }
    expect(liveCss).toMatch(/\.live-pill[^\n]*font-family: var\(--font-mono\)/)
  })

  it('draws the bars from tokens: a track and an accent fill', () => {
    expect(liveCss).toMatch(/\.live-bar-track[^\n]*background: var\(--color-track\)/)
    expect(liveCss).toMatch(/\.live-bar-fill[^\n]*background: var\(--color-accent\)/)
  })

  it('stacks the three bars of a row narrower than 28rem (a phone, or a row beside the open panel)', () => {
    expect(liveCss).toMatch(/\.live-session \{[^}]*container-type: inline-size/)
    expect(liveCss).toMatch(/@container \(max-width: 28rem\) \{ \.live-bars \{[^}]*grid-template-columns: minmax\(0, 1fr\)/)
    expect(liveCss).toMatch(/\.live-bar-num \{[^}]*white-space: nowrap/) // a number never breaks across two lines
  })

  it('puts the panel right of the rows from 48rem (768 px) and below them under it', () => {
    expect(liveCss).toMatch(/\.live-body \{[^}]*grid-template-columns: minmax\(0, 1fr\);/) // one column: the panel is below the rows
    expect(liveCss).toMatch(/@media \(min-width: 48rem\) \{ \.live-body\[data-panel\] \{[^}]*grid-template-columns: minmax\(0, 1fr\) minmax\(16rem, 22rem\)/)
  })

  it('keeps every text color of the view at 4.5:1 or more on the card and page backgrounds', () => {
    const tokens = readFileSync(resolve(process.cwd(), 'src/tokens.css'), 'utf8')
    const hex = (name) => tokens.match(new RegExp(`${name}:\\s*#([0-9a-f]{6});`))[1]
    const lum = (h) => [0, 2, 4].map((i) => parseInt(h.slice(i, i + 2), 16) / 255).map((c) => (c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4)).reduce((t, c, i) => t + c * [0.2126, 0.7152, 0.0722][i], 0)
    const ratio = (a, b) => (Math.max(lum(a), lum(b)) + 0.05) / (Math.min(lum(a), lum(b)) + 0.05)
    const used = liveCss.match(/color: var\((--color-[a-z-]+)\)/g).map((m) => m.match(/\((.*)\)/)[1])
    for (const text of new Set([...used, '--color-red'])) {
      for (const ground of ['--color-bg', '--color-raised', '--color-selected']) {
        expect(ratio(hex(text), hex(ground)), `${text} on ${ground}`).toBeGreaterThanOrEqual(4.5)
      }
    }
  })

  it('shows keyboard focus on the Reconnect button with the shared focus ring, and on a row with its own ring', () => {
    expect(css).toMatch(/:focus-visible\s*\{[^}]*outline: 2px solid var\(--color-accent\)/)
    expect(liveCss).toMatch(/\.live-row:focus-visible::after \{[^}]*outline: 2px solid var\(--color-accent\)/)
  })

  it('has a main landmark and a heading per section', () => {
    render(<LiveView />)
    stream().emit('snapshot', snapshot([session('aaaaaaaa-1')]))
    expect(screen.getByRole('main')).toBeInTheDocument()
    expect(screen.getByRole('heading', { level: 1, name: 'Live' })).toBeInTheDocument()
    expect(screen.getByRole('region', { name: 'All sessions' })).toBeInTheDocument()
  })
})
