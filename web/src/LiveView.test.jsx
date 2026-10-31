import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { act, fireEvent, render, screen, within } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import LiveView, { FEED_MAX } from './LiveView.jsx'

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
const snapshot = (sessions) => ({ active_window_s: 300, sessions })
let counter = 0
const live = (sessionId, over = {}) => ({
  id: ++counter, session_id: sessionId, account: 'Claude work', project: 'alpha', ts: new Date().toISOString(),
  kind: 'tool_use', tool: 'Bash', is_error: false, is_sidechain: false, tokens_in: null, tokens_out: null, cost_usd: null, ...over,
})
const send = (event) => stream().emit('live', event, String(event.id))
const card = (idPrefix) => screen.getByRole('heading', { level: 4, name: idPrefix }).closest('li')
const feedOf = (idPrefix) => within(card(idPrefix)).getAllByRole('listitem').map((li) => li.textContent)

beforeEach(() => {
  counter = 0
  FakeEventSource.instances = []
  window.location.hash = '#secret-token'
  vi.stubGlobal('EventSource', FakeEventSource)
})
afterEach(() => {
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
    expect(screen.getByRole('heading', { level: 4, name: 'aaaaaaaa' })).toBeInTheDocument()
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
  })

  it('groups sessions by account, then project', () => {
    render(<LiveView />)
    stream().emit('snapshot', snapshot([
      session('aaaaaaaa-1', { account: 'Claude work', project: 'alpha' }),
      session('bbbbbbbb-2', { account: 'Claude work', project: 'beta' }),
      session('cccccccc-3', { account: 'Claude home', project: 'alpha' }),
    ]))
    const accounts = screen.getAllByRole('heading', { level: 2 }).map((h) => h.textContent).filter((t) => t.startsWith('Claude'))
    expect(accounts).toEqual(['Claude home', 'Claude work'])
    const work = screen.getByRole('heading', { level: 2, name: 'Claude work' }).closest('section')
    expect(within(work).getAllByRole('heading', { level: 3 }).map((h) => h.textContent)).toEqual(['alpha', 'beta'])
    expect(within(work).getAllByRole('heading', { level: 4 })).toHaveLength(2)
  })

  it('a live event for an unknown session adds it under its own account and project', () => {
    render(<LiveView />)
    stream().emit('snapshot', snapshot([]))
    send(live('dddddddd-4', { account: 'Claude new', project: 'gamma' }))
    const section = screen.getByRole('heading', { level: 2, name: 'Claude new' }).closest('section')
    expect(within(section).getByRole('heading', { level: 3 })).toHaveTextContent('gamma')
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
    const heading = screen.getByRole('heading', { level: 3 })
    expect(heading).toHaveTextContent(long)
    expect(heading).toHaveAttribute('title', long)
    expect(heading).toHaveClass('live-name')
  })

  it('scrolls 60 sessions inside their own box', () => {
    render(<LiveView />)
    stream().emit('snapshot', snapshot(Array.from({ length: 60 }, (_, i) => session(`s${String(i).padStart(7, '0')}-x`, { project: `p${i}` }))))
    expect(screen.getAllByRole('heading', { level: 4 })).toHaveLength(60)
    const box = screen.getByRole('heading', { level: 4, name: 's0000000' }).closest('.live-sessions')
    expect(box).not.toBeNull()
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
    expect(within(card('aaaaaaaa')).getByText('Tool calls').nextSibling).toHaveTextContent('1')
  })

  it('announces new items politely and labels the feed', () => {
    render(<LiveView />)
    stream().emit('snapshot', snapshot([session('aaaaaaaa-1')]))
    const feed = within(card('aaaaaaaa')).getByRole('list', { name: /Tool calls, newest first/ })
    expect(feed).toHaveAttribute('aria-live', 'polite')
  })

  it('ignores an event it has already seen (same id)', () => {
    render(<LiveView />)
    stream().emit('snapshot', snapshot([session('aaaaaaaa-1')]))
    const event = live('aaaaaaaa-1', { tokens_in: 10, tokens_out: 0, cost_usd: 1 })
    send(event)
    send(event)
    expect(feedOf('aaaaaaaa')).toHaveLength(1)
    expect(within(card('aaaaaaaa')).getByText('Tokens in').nextSibling).toHaveTextContent('10')
  })

  it('a malformed message is ignored', () => {
    render(<LiveView />)
    stream().emit('snapshot', snapshot([session('aaaaaaaa-1')]))
    act(() => stream().listeners.live.forEach((fn) => fn({ data: '{not json', lastEventId: '9' })))
    expect(screen.getByRole('heading', { level: 4, name: 'aaaaaaaa' })).toBeInTheDocument()
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
    const a = card('aaaaaaaa')
    expect(within(a).getByText('Tool calls').nextSibling).toHaveTextContent('4')
    expect(within(a).getByText('Tokens in').nextSibling).toHaveTextContent('1,100')
    expect(within(a).getByText('Tokens out').nextSibling).toHaveTextContent('250')
    expect(within(a).getByText('API-equivalent cost').nextSibling).toHaveTextContent('$0.60')
    expect(within(card('bbbbbbbb')).getByText('API-equivalent cost').nextSibling).toHaveTextContent('$0.25')
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
    const text = within(card('aaaaaaaa')).getByText('API-equivalent cost').nextSibling.textContent
    expect(text).toContain('abstained')
    expect(text).not.toContain('$0')
    send(live('aaaaaaaa-1', { kind: 'usage', tool: null, tokens_in: 1, tokens_out: 1, cost_usd: 0.4 }))
    expect(within(card('aaaaaaaa')).getByText('API-equivalent cost').nextSibling.textContent).toMatch(/^\$0\.40 \+ abstained/)
  })

  it('a fresh snapshot replaces the totals and keeps the feed of a session it still lists', () => {
    render(<LiveView />)
    stream().emit('snapshot', snapshot([session('aaaaaaaa-1')]))
    send(live('aaaaaaaa-1', { tool: 'Read', tokens_in: 5, tokens_out: 5, cost_usd: 0.05 }))
    stream().emit('snapshot', snapshot([session('aaaaaaaa-1', { tool_calls: 9, tokens_in: 900, tokens_out: 100, cost_usd: 2 })]))
    expect(feedOf('aaaaaaaa')).toHaveLength(1)
    expect(within(card('aaaaaaaa')).getByText('Tool calls').nextSibling).toHaveTextContent('9')
    expect(within(card('aaaaaaaa')).getByText('API-equivalent cost').nextSibling).toHaveTextContent('$2.00')
    send(live('aaaaaaaa-1', { tool: 'Edit' })) // ids start over after a snapshot
    expect(feedOf('aaaaaaaa')).toHaveLength(2)
  })
})

describe('no content', () => {
  it('shows only metadata even if the server sent extra fields', () => {
    render(<LiveView />)
    stream().emit('snapshot', snapshot([session('aaaaaaaa-1')]))
    send({ ...live('aaaaaaaa-1', { tool: 'Bash' }), input: { command: 'cat SECRET-planted' }, text: 'SECRET-planted body' })
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

  it('keeps every text color of the view at 4.5:1 or more on the card and page backgrounds', () => {
    const tokens = readFileSync(resolve(process.cwd(), 'src/tokens.css'), 'utf8')
    const hex = (name) => tokens.match(new RegExp(`${name}:\\s*#([0-9a-f]{6});`))[1]
    const lum = (h) => [0, 2, 4].map((i) => parseInt(h.slice(i, i + 2), 16) / 255).map((c) => (c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4)).reduce((t, c, i) => t + c * [0.2126, 0.7152, 0.0722][i], 0)
    const ratio = (a, b) => (Math.max(lum(a), lum(b)) + 0.05) / (Math.min(lum(a), lum(b)) + 0.05)
    const used = liveCss.match(/color: var\((--color-[a-z-]+)\)/g).map((m) => m.match(/\((.*)\)/)[1])
    for (const text of new Set([...used, '--color-red'])) {
      for (const ground of ['--color-bg', '--color-raised']) {
        expect(ratio(hex(text), hex(ground)), `${text} on ${ground}`).toBeGreaterThanOrEqual(4.5)
      }
    }
  })

  it('shows keyboard focus on the Reconnect button with the shared focus ring', () => {
    expect(css).toMatch(/:focus-visible\s*\{[^}]*outline: 2px solid var\(--color-accent\)/)
  })

  it('has a main landmark and a heading per section', () => {
    render(<LiveView />)
    stream().emit('snapshot', snapshot([session('aaaaaaaa-1')]))
    expect(screen.getByRole('main')).toBeInTheDocument()
    expect(screen.getByRole('heading', { level: 1, name: 'Live' })).toBeInTheDocument()
    expect(screen.getByRole('region', { name: 'All sessions' })).toBeInTheDocument()
  })
})
