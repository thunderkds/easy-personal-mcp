import { act, fireEvent, render, screen, within } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import App from './App.jsx'
import { LiveContext } from './LiveView.jsx'
import StatusChips from './StatusChips.jsx'
import { accountsStatus, fakeFetch, firstRun } from './test/fixtures.js'

const NOW = Date.parse('2026-11-14T10:00:00Z')
const minutesAgo = (m) => new Date(NOW - m * 60_000).toISOString()

const session = (id, attention = [], minutes = 0) => ({
  session_id: id, account: 'Claude work', project: 'alpha', last_ts: minutesAgo(minutes),
  tool_calls: 0, tokens_in: 0, tokens_out: 0, cost: 0, unpriced: false, attention, feed: [],
})
const waiting = { state: 'waiting', tool: null, count: 1, is_sidechain: false }
const live = (sessions, over = {}) => ({
  state: { sessions: Object.fromEntries(sessions.map((s) => [s.session_id, s])), lastId: 0, windowS: 300 },
  link: 'live',
  loaded: true,
  reconnect() {},
  ...over,
})
const claude = (id, label, five, over = {}) => ({
  id, provider: 'claude', label, folder: `/home/o/.claude${id}`, status: 'confirmed',
  status_line: { limits: five === undefined ? null : { five_hour: five, seven_day: null, stale: false } },
  ...over,
})
const win = (used_percentage, stale = false) => ({ used_percentage, resets_at: '2026-11-14T11:00:00Z', stale })
const health = (last) => ({ sources: { claude_code: { last_ingest_utc: last, abstained: 0 }, kit_trace: { last_ingest_utc: null, abstained: 0 } } })
const timer = (done) => firstRun.steps.map((s) => ({ ...s, done: s.id === 'ingest_timer' ? done : true }))

// An option left out gets the default; `undefined` given on purpose means "still loading".
const show = (given = {}) => {
  const pick = (key, fallback) => (key in given ? given[key] : fallback)
  return render(
    <LiveContext.Provider value={pick('context', live([session('a'), session('b', [waiting])]))}>
      <StatusChips
        steps={pick('steps', timer(true))}
        health={pick('h', health(minutesAgo(4)))}
        accounts={pick('accounts', [claude(1, 'Claude work', win(42.4))])}
        onOpenLive={pick('onOpenLive', () => {})}
      />
    </LiveContext.Provider>,
  )
}
const chip = (text) => screen.getByText(text, { exact: false }).closest('.status-chip')

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date', 'setInterval', 'clearInterval'] })
  vi.setSystemTime(NOW)
})
afterEach(() => {
  vi.useRealTimers()
})

describe('status chips (T068)', () => {
  it('shows live, waiting, last ingest and 5h limit from the data', () => {
    show()
    expect(chip('2 active')).toHaveAttribute('title', 'Live sessions')
    expect(chip('1 waiting')).toHaveAttribute('title', 'Sessions waiting for you')
    expect(chip('ingest 4m ago')).toHaveAttribute('title', expect.stringContaining('Last ingest'))
    expect(chip('5h 42%')).toHaveAttribute('title', expect.stringContaining('5-hour plan limit'))
    expect(chip('5h 42%').querySelector('.status-bar-fill')).toHaveStyle({ width: '42%' })
  })

  it('counts a session that has an attention state as waiting, and never as idle', () => {
    const failing = { state: 'failing', tool: 'Bash', count: 3, is_sidechain: false }
    show({ context: live([session('a', [failing], 60), session('b', [], 60)]) })
    expect(chip('1 active')).toBeInTheDocument()
    expect(chip('1 waiting')).toBeInTheDocument()
  })

  it('shows a grey 0 active and no waiting chip when no session is live', () => {
    show({ context: live([]) })
    expect(chip('0 active')).toHaveAttribute('data-on', 'false')
    expect(screen.queryByText(/waiting/)).not.toBeInTheDocument()
  })

  it('does not count a session past the active window as active', () => {
    show({ context: live([session('a', [], 1), session('b', [], 6)]) })
    expect(chip('1 active')).toHaveAttribute('data-on', 'true')
  })

  it('keeps the last counts while the stream reconnects', () => {
    show({ context: live([session('a'), session('b')], { link: 'reconnecting' }) })
    expect(chip('2 active')).toBeInTheDocument()
  })

  it('shows no live chips before the first snapshot', () => {
    show({ context: live([], { loaded: false, link: 'connecting' }) })
    expect(screen.queryByText(/active/)).not.toBeInTheDocument()
  })

  it('opens the Live view from the live and waiting chips', () => {
    const onOpenLive = vi.fn()
    show({ onOpenLive })
    fireEvent.click(chip('2 active'))
    fireEvent.click(chip('1 waiting'))
    expect(onOpenLive).toHaveBeenCalledTimes(2)
  })

  it('turns the ingest chip amber with "no timer" when the timer step is not done', () => {
    show({ steps: timer(false) })
    const c = chip('ingest 4m ago')
    expect(c).toHaveTextContent('ingest 4m ago · no timer')
    expect(c).toHaveClass('tone-amber')
    expect(c.querySelector('svg')).not.toBeNull()
  })

  it('keeps the ingest chip muted when the timer is installed or the checklist is unknown', () => {
    const { unmount } = show()
    expect(chip('ingest 4m ago')).not.toHaveClass('tone-amber')
    unmount()
    show({ steps: null })
    expect(chip('ingest 4m ago')).not.toHaveClass('tone-amber')
  })

  it('takes the newest ingest time over the sources and writes it short', () => {
    const h = { sources: { a: { last_ingest_utc: minutesAgo(200) }, b: { last_ingest_utc: minutesAgo(90) }, c: { last_ingest_utc: null } } }
    show({ h })
    expect(chip('ingest 1h ago')).toBeInTheDocument()
  })

  it.each([
    [0, 'ingest just now'],
    [59, 'ingest 59m ago'],
    [60 * 5, 'ingest 5h ago'],
    [60 * 24 * 3, 'ingest 3d ago'],
  ])('writes an ingest %i minutes old as "%s"', (m, text) => {
    show({ h: health(minutesAgo(m)) })
    expect(chip(text)).toBeInTheDocument()
  })

  it('says "ingest no data" (faint), never 0m ago, when the health read failed or has no time', () => {
    const { unmount } = show({ h: null })
    expect(chip('ingest no data')).toHaveClass('tone-faint')
    unmount()
    show({ h: { sources: null } })
    expect(chip('ingest no data')).toBeInTheDocument()
    expect(screen.queryByText(/ago/)).not.toBeInTheDocument()
  })

  it('draws no ingest or limit chip while their data is loading', () => {
    show({ h: undefined, accounts: undefined })
    expect(screen.queryByText(/ingest/)).not.toBeInTheDocument()
    expect(screen.queryByText(/^5h/)).not.toBeInTheDocument()
  })

  it('shows 5h no data (faint) when no account has the value', () => {
    show({ accounts: [claude(1, 'Claude work', undefined), claude(2, 'Claude old', win(88, true))] })
    expect(chip('5h no data')).toHaveClass('tone-faint')
    expect(chip('5h no data').querySelector('.status-bar')).toBeNull()
  })

  it('shows 5h no data when the accounts could not be read', () => {
    show({ accounts: null })
    expect(chip('5h no data')).toBeInTheDocument()
  })

  it('goes amber from 70 % and red from 90 %', () => {
    const { unmount } = show({ accounts: [claude(1, 'a', win(69.6))] })
    expect(chip('5h 70%')).toHaveClass('tone-amber') // rounds like the rail
    unmount()
    const second = show({ accounts: [claude(1, 'a', win(72))] })
    expect(chip('5h 72%')).toHaveClass('tone-amber')
    second.unmount()
    const third = show({ accounts: [claude(1, 'a', win(91))] })
    expect(chip('5h 91%')).toHaveClass('tone-red')
    third.unmount()
    show({ accounts: [claude(1, 'a', win(69))] })
    expect(chip('5h 69%')).not.toHaveClass('tone-amber')
  })

  it('takes the highest percentage over confirmed Claude accounts and names it in the title', () => {
    show({
      accounts: [
        claude(1, 'Claude work', win(30)),
        claude(2, 'Claude personal', win(55)),
        claude(3, 'Claude old', win(99, true)),
        claude(4, 'Claude gone', win(95), { status: 'missing' }),
        claude(5, 'Codex', win(97), { provider: 'codex' }),
      ],
    })
    expect(chip('5h 55%')).toHaveAttribute('title', expect.stringContaining('Claude personal'))
  })

  it('reads the limits of the shared account fixture', () => {
    show({ accounts: accountsStatus.accounts })
    expect(chip('5h 92%')).toHaveClass('tone-red')
  })
})

// One EventSource for the whole page (T068 design constraint): the top bar reads the stream App already holds.
describe('one live stream', () => {
  class FakeEventSource {
    static instances = []
    constructor(url) {
      this.url = url
      this.listeners = {}
      this.readyState = 0
      FakeEventSource.instances.push(this)
    }
    addEventListener(type, fn) {
      ;(this.listeners[type] ??= []).push(fn)
    }
    close() {
      this.readyState = 2
    }
    emit(type, data) {
      act(() => (this.listeners[type] ?? []).forEach((fn) => fn({ type, data: JSON.stringify(data) })))
    }
  }
  const nav = () => screen.getByRole('navigation', { name: 'Views' })
  const bar = () => within(screen.getByRole('banner'))

  beforeEach(() => {
    vi.useRealTimers()
    FakeEventSource.instances = []
    window.location.hash = '#secret-token'
    window.localStorage.clear()
    vi.stubGlobal('EventSource', FakeEventSource)
    vi.stubGlobal('fetch', fakeFetch({ accounts: accountsStatus }))
  })
  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('opens one EventSource while the Live view and the top bar are shown, and after switching views', async () => {
    render(<App />)
    fireEvent.click(within(nav()).getByRole('button', { name: 'Live' }))
    expect(await screen.findByRole('heading', { name: 'Live' })).toBeInTheDocument()
    const [source] = FakeEventSource.instances
    source.emit('snapshot', {
      active_window_s: 300, last_id: 1,
      sessions: [
        { session_id: 'aaaaaaaa1', account: 'Claude work', project: 'alpha', last_ts: new Date().toISOString(), tool_calls: 0, tokens_in: 0, tokens_out: 0, cost_usd: 0 },
        { session_id: 'bbbbbbbb2', account: 'Claude work', project: 'beta', last_ts: new Date().toISOString(), tool_calls: 0, tokens_in: 0, tokens_out: 0, cost_usd: 0, attention: [waiting] },
      ],
    })
    expect(bar().getByText('2 active')).toBeInTheDocument() // the bar and the Live rows read the same stream
    expect(bar().getByText('1 waiting')).toBeInTheDocument()
    expect(screen.getAllByText(/^active ·/)).toHaveLength(2)
    for (const view of ['Usage', 'Live', 'Kit', 'Live']) {
      fireEvent.click(within(nav()).getByRole('button', { name: view }))
    }
    await screen.findByRole('heading', { name: 'Live' })
    expect(FakeEventSource.instances).toHaveLength(1)
    expect(source.readyState).not.toBe(2)
    expect(bar().getByText('2 active')).toBeInTheDocument()
  })

  it('draws the chips from the page data on a view without the Live view', async () => {
    render(<App />)
    expect(await bar().findByText('5h 92%')).toBeInTheDocument()
    expect(await bar().findByText(/ingest no data/)).toBeInTheDocument() // the fixture has no /api/store/health route
    expect(FakeEventSource.instances).toHaveLength(1)
    expect(FakeEventSource.instances[0].url).toBe('/api/live?token=secret-token')
  })

  it('opens the Live view from the Waiting chip', async () => {
    render(<App />)
    FakeEventSource.instances[0].emit('snapshot', {
      active_window_s: 300, last_id: 1,
      sessions: [{ session_id: 'bbbbbbbb2', account: 'Claude work', project: 'beta', last_ts: new Date().toISOString(), tool_calls: 0, tokens_in: 0, tokens_out: 0, cost_usd: 0, attention: [waiting] }],
    })
    fireEvent.click(await bar().findByText('1 waiting'))
    expect(await screen.findByRole('heading', { name: 'Live' })).toBeInTheDocument()
    expect(within(nav()).getByRole('button', { name: 'Live' })).toHaveAttribute('aria-current', 'page')
  })
})
