import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { act, render, screen, waitFor, within } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import WatchView from './WatchView.jsx'

const T0 = 1_800_000_000
const sample = (i, over = {}) => ({
  t: T0 + i * 60,
  cpu_pct: 20 + i,
  ram_used: 6 * 2 ** 30,
  ram_total: 16 * 2 ** 30,
  disk_used: 70 * 2 ** 30,
  disk_total: 100 * 2 ** 30,
  load1: 0.5 + i / 10,
  battery_pct: 88,
  charging: true,
  claude_procs: 3,
  ...over,
})

// Five samples; the middle one has no CPU and no battery reading, the last has no battery.
const samples = [sample(0), sample(1), sample(2, { cpu_pct: null, battery_pct: null, charging: null }), sample(3), sample(4, { battery_pct: null, charging: null })]
const machine = { latest: samples.at(-1), samples, interval_s: 60 }

const limits = {
  interval_s: 60,
  accounts: [
    {
      id: 1,
      label: 'work',
      samples: [
        { t: T0, five_h_pct: 10, seven_d_pct: 20 },
        { t: T0 + 60, five_h_pct: null, seven_d_pct: null },
        { t: T0 + 120, five_h_pct: 95, seven_d_pct: 30 },
      ],
    },
    {
      id: 2,
      label: 'personal',
      samples: [{ t: T0, five_h_pct: 5, seven_d_pct: 6 }],
    },
  ],
}

const cost = (usd, unknown = false) => ({ usd, unknown_price: unknown, abstained_models: unknown ? ['x-model'] : [] })
const effectiveness = {
  oldest_stored_day: '2026-10-01',
  retention_note: 'events before 2026-10-01 are not in the store (pruned with --retention-days, or never recorded)',
  notes: ['no local git history for gone-repo (or git failed or did not answer in time): done time unknown'],
  tasks: [
    { task_id: 'T075', project: 'kit', tokens: 1_200_000, api_equivalent_cost: cost(4.1), sessions: 6, good_results: 4, tokens_per_good_result: 300_000, done_utc: '2026-11-14T10:00:00Z', done_state: 'done', tokens_per_task_done: 1_200_000 },
    { task_id: 'HUN-245', project: 'kit', tokens: 800_000, api_equivalent_cost: cost(null, true), sessions: 3, good_results: 0, tokens_per_good_result: null, done_utc: null, done_state: 'not done', tokens_per_task_done: null },
    { task_id: 'T9', project: 'gone-repo', tokens: 5_000, api_equivalent_cost: cost(1.5, true), sessions: 1, good_results: 1, tokens_per_good_result: 5_000, done_utc: null, done_state: 'unknown', tokens_per_task_done: null },
  ],
  unattributed: { tokens: 90_000, api_equivalent_cost: cost(0.5), sessions: 2, good_results: 1, tokens_per_good_result: 90_000 },
  totals: {},
}

function stub(overrides = {}) {
  const answers = { '/api/machine': machine, '/api/limits/history': limits, '/api/effectiveness': effectiveness, ...overrides }
  const f = vi.fn(async (url) => {
    const body = answers[new URL(String(url), 'http://127.0.0.1').pathname]
    if (body instanceof Error) return { ok: false, status: 500, json: async () => ({}) }
    return { ok: true, status: 200, json: async () => body }
  })
  vi.stubGlobal('fetch', f)
  return f
}

beforeEach(() => {
  window.location.hash = '#secret-token'
})
afterEach(() => {
  vi.unstubAllGlobals()
})

const tile = (name) => screen.getByRole('heading', { name, level: 3 }).closest('section')
const sparkOf = (name) => within(tile(name)).getByRole('img')
const pathOf = (el) => el.querySelector('path.watch-line').getAttribute('d')

describe('machine tiles (AC1)', () => {
  it('reads only the three read-only routes, with the token', async () => {
    const f = stub()
    render(<WatchView />)
    await screen.findByText('Task')
    expect(f.mock.calls.map(([u]) => u).sort()).toEqual(['/api/effectiveness', '/api/limits/history', '/api/machine'])
    for (const [, init] of f.mock.calls) expect(init.headers['X-Dashboard-Token']).toBe('secret-token')
  })

  it('shows six tiles with the latest value', async () => {
    stub()
    render(<WatchView />)
    await screen.findByRole('heading', { name: 'CPU', level: 3 })
    for (const name of ['CPU', 'RAM', 'Disk', 'Load (1 min)', 'Battery', 'Claude processes']) expect(tile(name)).toBeInTheDocument()
    expect(tile('RAM')).toHaveTextContent('6.0 / 16.0 GB')
    expect(tile('Disk')).toHaveTextContent('70 %')
    expect(tile('Load (1 min)')).toHaveTextContent('0.9')
    expect(tile('Claude processes')).toHaveTextContent('3')
    expect(tile('CPU')).toHaveTextContent('24 %')
  })

  it('draws a null sample as a gap: the path breaks, no point is drawn at 0', async () => {
    stub()
    render(<WatchView />)
    await screen.findByRole('heading', { name: 'CPU', level: 3 })
    const d = pathOf(sparkOf('CPU'))
    // samples 0-1, a gap at 2, then samples 3-4: two pieces, so two "M"
    expect(d.match(/M/g)).toHaveLength(2)
    // CPU 20..24 % on a 0..100 scale never reaches the bottom line (y = 36): nothing is drawn at 0
    for (const [, y] of d.matchAll(/[ML]\s*[\d.]+ ([\d.]+)/g)) expect(Number(y)).toBeLessThan(36)
  })

  it('says "no data" for a measure whose latest value is null', async () => {
    stub()
    render(<WatchView />)
    await screen.findByRole('heading', { name: 'Battery', level: 3 })
    expect(tile('Battery')).toHaveTextContent('no data')
    expect(pathOf(sparkOf('Battery'))).toMatch(/^M/)
  })

  it('draws no line for a measure with no data at all', async () => {
    const none = [sample(0, { battery_pct: null, charging: null }), sample(1, { battery_pct: null, charging: null })]
    stub({ '/api/machine': { latest: none[1], samples: none, interval_s: 60 } })
    render(<WatchView />)
    await screen.findByRole('heading', { name: 'Battery', level: 3 })
    expect(tile('Battery')).toHaveTextContent('no data')
    expect(sparkOf('Battery').querySelector('path')).toBeNull()
  })

  it('breaks the line where the sampler paused (more than two intervals between samples)', async () => {
    const paused = [sample(0), sample(1), sample(30), sample(31)]
    stub({ '/api/machine': { latest: paused.at(-1), samples: paused, interval_s: 60 } })
    render(<WatchView />)
    await screen.findByRole('heading', { name: 'CPU', level: 3 })
    expect(pathOf(sparkOf('CPU')).match(/M/g)).toHaveLength(2)
  })

  it('shows a lone reading as a dot, not nothing', async () => {
    const lone = [sample(0, { cpu_pct: null }), sample(1), sample(2, { cpu_pct: null })]
    stub({ '/api/machine': { latest: lone.at(-1), samples: lone, interval_s: 60 } })
    render(<WatchView />)
    await screen.findByRole('heading', { name: 'CPU', level: 3 })
    expect(pathOf(sparkOf('CPU'))).toMatch(/^M[\d.]+ [\d.]+ L[\d.]+ [\d.]+$/)
  })

  it('names each graph for a screen reader, with the latest value', async () => {
    stub()
    render(<WatchView />)
    await screen.findByRole('heading', { name: 'CPU', level: 3 })
    expect(sparkOf('CPU')).toHaveAccessibleName('CPU, last 24 hours, latest 24 %')
    expect(sparkOf('Battery')).toHaveAccessibleName('Battery, last 24 hours, latest no data')
  })

  it('says there are no samples yet right after start', async () => {
    stub({ '/api/machine': { latest: null, samples: [], interval_s: 60 } })
    render(<WatchView />)
    expect(await screen.findByText(/No samples yet/)).toBeInTheDocument()
    expect(screen.queryByRole('heading', { name: 'CPU', level: 3 })).toBeNull()
  })

  it('shows a charging battery', async () => {
    const ok = [sample(0), sample(1)]
    stub({ '/api/machine': { latest: ok[1], samples: ok, interval_s: 60 } })
    render(<WatchView />)
    await screen.findByRole('heading', { name: 'Battery', level: 3 })
    expect(tile('Battery')).toHaveTextContent('88 %')
    expect(tile('Battery')).toHaveTextContent('charging')
  })
})

describe('plan limits (AC1)', () => {
  it('shows one card per account with the latest 5 h and 7 d values; 90 % and more is red', async () => {
    stub()
    render(<WatchView />)
    const work = (await screen.findByRole('heading', { name: 'work', level: 3 })).closest('section')
    expect(within(work).getByText('95 %')).toHaveClass('limit-red')
    expect(within(work).getByText('30 %')).not.toHaveClass('limit-red')
    const personal = screen.getByRole('heading', { name: 'personal', level: 3 }).closest('section')
    expect(personal).toHaveTextContent('5 h')
    expect(personal).toHaveTextContent('5 %')
    expect(personal).toHaveTextContent('6 %')
  })

  it('draws the 5 h and the 7 d line; a null round is a gap', async () => {
    stub()
    render(<WatchView />)
    const work = (await screen.findByRole('heading', { name: 'work', level: 3 })).closest('section')
    const svg = within(work).getByRole('img')
    expect(svg).toHaveAccessibleName('work plan limits, last 24 hours, 5 h 95 %, 7 d 30 %')
    const five = svg.querySelector('path.watch-line')
    const seven = svg.querySelector('path.watch-line-7d')
    // samples 0 and 2 are single readings around the gap: each is its own piece
    expect(five.getAttribute('d').match(/M/g)).toHaveLength(2)
    expect(seven.getAttribute('d').match(/M/g)).toHaveLength(2)
    expect(svg.querySelector('line.watch-ref')).not.toBeNull()
  })

  it('says "no data" when an account has only null readings', async () => {
    const nulls = { interval_s: 60, accounts: [{ id: 1, label: 'work', samples: [{ t: T0, five_h_pct: null, seven_d_pct: null }] }] }
    stub({ '/api/limits/history': nulls })
    render(<WatchView />)
    const work = (await screen.findByRole('heading', { name: 'work', level: 3 })).closest('section')
    expect(work).toHaveTextContent('no data')
    expect(work.querySelector('path.watch-line')).toBeNull()
  })

  it('says so when there is no Claude account', async () => {
    stub({ '/api/limits/history': { interval_s: 60, accounts: [] } })
    render(<WatchView />)
    expect(await screen.findByText('No Claude account yet.')).toBeInTheDocument()
  })
})

describe('effectiveness table (AC1)', () => {
  const rows = async () => {
    stub()
    render(<WatchView />)
    await screen.findByText('Task')
    return within(screen.getByRole('table')).getAllByRole('row').slice(1)
  }

  it('has the eight columns', async () => {
    stub()
    render(<WatchView />)
    await screen.findByText('Task')
    const heads = within(screen.getByRole('table')).getAllByRole('columnheader').map((h) => h.textContent)
    expect(heads).toEqual(['Task', 'Tokens', 'Cost', 'Sessions', 'Good results', 'Tokens per good result', 'Done', 'Tokens per task done'])
  })

  it('lists the tasks in the order given and the unattributed line last', async () => {
    const r = await rows()
    expect(r).toHaveLength(4)
    expect(r.map((row) => row.cells[0].textContent)).toEqual(['T075', 'HUN-245', 'T9', 'Unattributed'])
  })

  it('shows the figures of a done task', async () => {
    const [row] = await rows()
    expect([...row.cells].map((c) => c.textContent)).toEqual(['T075', '1.2 M', '$4.10', '6', '4', '300 K', '2026-11-14', '1.2 M'])
  })

  it('shows no data, not done, unknown and never $0 for what is missing', async () => {
    const r = await rows()
    const texts = (row) => [...row.cells].map((c) => c.textContent)
    expect(texts(r[1])).toEqual(['HUN-245', '800 K', 'unknown price', '3', '0', 'no data', 'not done', 'no data'])
    expect(texts(r[2])[2]).toBe('$1.50 + unknown price')
    expect(texts(r[2])[6]).toBe('unknown')
    expect(texts(r[3])).toEqual(['Unattributed', '90 K', '$0.50', '2', '1', '90 K', '—', '—'])
    expect(screen.getByRole('table')).not.toHaveTextContent('$0.00')
  })

  it('names the project of a task in its title', async () => {
    const r = await rows()
    expect(r[2].cells[0]).toHaveAttribute('title', 'gone-repo')
  })

  it('shows the oldest stored day note when the API gives it, and the notes', async () => {
    stub()
    render(<WatchView />)
    expect(await screen.findByText(/events before 2026-10-01 are not in the store/)).toBeInTheDocument()
    expect(screen.getByText(/no local git history for gone-repo/)).toBeInTheDocument()
  })

  it('shows no note when the API gives none', async () => {
    stub({ '/api/effectiveness': { ...effectiveness, oldest_stored_day: null, retention_note: null, notes: [] } })
    render(<WatchView />)
    await screen.findByText('Task')
    expect(screen.queryByText(/not in the store/)).toBeNull()
  })

  it('says there are no tasks when there is nothing to show', async () => {
    stub({ '/api/effectiveness': { ...effectiveness, tasks: [], unattributed: { ...effectiveness.unattributed, tokens: 0, sessions: 0, good_results: 0, tokens_per_good_result: null } } })
    render(<WatchView />)
    expect(await screen.findByText('No tasks yet.')).toBeInTheDocument()
    expect(screen.queryByRole('table')).toBeNull()
  })

  it('scrolls inside its own frame: the table sits in .table-wrap', async () => {
    stub()
    render(<WatchView />)
    await screen.findByText('Task')
    expect(screen.getByRole('table').closest('.table-wrap')).not.toBeNull()
  })
})

describe('parts load and fail on their own', () => {
  it('shows an error line for the failed part and still renders the others', async () => {
    stub({ '/api/machine': new Error('boom') })
    render(<WatchView />)
    expect(await screen.findByRole('alert')).toHaveTextContent('Could not load machine status')
    expect(await screen.findByRole('heading', { name: 'work', level: 3 })).toBeInTheDocument()
    expect(screen.getByRole('table')).toBeInTheDocument()
  })

  it('shows a loading line per part until its answer arrives', () => {
    vi.stubGlobal('fetch', () => new Promise(() => {}))
    render(<WatchView />)
    expect(screen.getAllByRole('status')).toHaveLength(3)
  })

  it('has the page heading and the 24 h note', async () => {
    stub()
    render(<WatchView />)
    expect(screen.getByRole('heading', { name: 'Watch', level: 1 })).toBeInTheDocument()
    expect(screen.getByText(/the graphs start empty after a restart/)).toBeInTheDocument()
    await screen.findByText('Task')
  })
})

describe('design tokens (AC3)', () => {
  it('watch.css holds no colour literal and no raw px font size', () => {
    const css = readFileSync(resolve(process.cwd(), 'src/watch.css'), 'utf8')
    expect(css).not.toMatch(/#[0-9a-fA-F]{3,8}\b|rgba?\(|hsla?\(/)
    expect(css).not.toMatch(/font-size:\s*[\d.]+px/)
  })

  it('WatchView.jsx holds no colour literal', () => {
    const src = readFileSync(resolve(process.cwd(), 'src/WatchView.jsx'), 'utf8')
    expect(src).not.toMatch(/#[0-9a-fA-F]{3,8}\b|rgba?\(|hsla?\(/)
  })
})

describe('refresh every 60 s (T083 AC1, AC2)', () => {
  beforeEach(() => {
    // Only the interval is faked: promises and the waiting of findBy / waitFor stay real.
    vi.useFakeTimers({ toFake: ['setInterval', 'clearInterval'] })
  })
  afterEach(() => {
    vi.useRealTimers()
  })
  const tick = (ms) => act(() => vi.advanceTimersByTime(ms))
  const urls = (f) => f.mock.calls.map(([u]) => u).sort()

  it('reads each of the three routes again after 60 s, and stops when the view closes', async () => {
    const f = stub()
    const { unmount } = render(<WatchView />)
    await screen.findByText('Task')
    expect(f).toHaveBeenCalledTimes(3)
    tick(59_000)
    expect(f).toHaveBeenCalledTimes(3)
    tick(1_000)
    await waitFor(() => expect(f).toHaveBeenCalledTimes(6))
    expect(urls(f)).toEqual(['/api/effectiveness', '/api/effectiveness', '/api/limits/history', '/api/limits/history', '/api/machine', '/api/machine'])
    unmount()
    tick(300_000)
    expect(f).toHaveBeenCalledTimes(6)
  })

  it('skips a tick while the earlier read of that route is still in flight', async () => {
    const f = stub()
    render(<WatchView />)
    await screen.findByText('Task')
    f.mockImplementation(() => new Promise(() => {}))
    tick(60_000)
    expect(f).toHaveBeenCalledTimes(6)
    tick(180_000)
    expect(f).toHaveBeenCalledTimes(6)
  })

  it('keeps the last good data and shows the error line when a refresh fails; the next good read clears it', async () => {
    const f = stub()
    render(<WatchView />)
    await screen.findByRole('heading', { name: 'CPU', level: 3 })
    stub({ '/api/machine': new Error('boom') })
    tick(60_000)
    expect(await screen.findByRole('alert')).toHaveTextContent('Could not load machine status')
    expect(tile('CPU')).toHaveTextContent('24 %')
    expect(screen.getByRole('table')).toBeInTheDocument()
    expect(f).toHaveBeenCalled()
    stub()
    tick(60_000)
    await waitFor(() => expect(screen.queryByRole('alert')).toBeNull())
    expect(tile('CPU')).toHaveTextContent('24 %')
  })

  it('shows newer rows after a refresh', async () => {
    stub()
    render(<WatchView />)
    await screen.findByRole('heading', { name: 'CPU', level: 3 })
    const next = [...samples, sample(5, { cpu_pct: 77 })]
    stub({ '/api/machine': { latest: next.at(-1), samples: next, interval_s: 60 } })
    tick(60_000)
    await waitFor(() => expect(tile('CPU')).toHaveTextContent('77 %'))
  })
})

describe('graphs share one "now" (T083 AC3)', () => {
  const lastX = (d) => Number(d.match(/([\d.]+) [\d.]+$/)[1])
  const pair = (endT) => [
    { t: endT - 3600, five_h_pct: 10, seven_d_pct: 20 },
    { t: endT, five_h_pct: 30, seven_d_pct: 40 },
  ]

  it('a limits series that stopped 1 h earlier ends before the right edge', async () => {
    const lim = {
      interval_s: 60,
      accounts: [
        { id: 1, label: 'live', samples: pair(T0 + 7200) },
        { id: 2, label: 'stale', samples: pair(T0 + 3600) },
      ],
    }
    stub({ '/api/limits/history': lim })
    render(<WatchView />)
    const card = async (name) => (await screen.findByRole('heading', { name, level: 3 })).closest('section')
    const live = within(await card('live')).getByRole('img').querySelector('path.watch-line').getAttribute('d')
    const stale = within(await card('stale')).getByRole('img').querySelector('path.watch-line').getAttribute('d')
    expect(lastX(live)).toBeCloseTo(200, 1)
    // 1 h of 24 h is 8.33 of 200
    expect(lastX(stale)).toBeCloseTo(200 - 200 / 24, 1)
  })

  it('a machine series ends at the newest sample of the limits, not at its own last sample', async () => {
    const lim = { interval_s: 60, accounts: [{ id: 1, label: 'live', samples: pair(T0 + 4 * 60 + 3600) }] }
    stub({ '/api/limits/history': lim })
    render(<WatchView />)
    await screen.findByRole('heading', { name: 'live', level: 3 })
    expect(lastX(pathOf(sparkOf('CPU')))).toBeCloseTo(200 - 200 / 24, 1)
  })
})
