import { render, screen, waitFor, within } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import Rail from './Rail.jsx'
import UsageView from './UsageView.jsx'
import { NO_SCOPE } from './Scope.jsx'
import { accountsStatus, breakdowns, emptySummary, fakeFetch, statusLine, summary, warnings } from './test/fixtures.js'

beforeEach(() => {
  window.location.hash = '#secret-token'
})
afterEach(() => vi.unstubAllGlobals())

const stub = (overrides) => {
  const f = fakeFetch(overrides)
  vi.stubGlobal('fetch', f)
  return f
}
const card = (name) => screen.getByRole('region', { name })
const failing = { ok: false, status: 500, json: async () => ({}) }

describe('right rail (HUN-121)', () => {
  it('AC5: renders the three cards with their headings', async () => {
    stub({ accounts: accountsStatus })
    render(<Rail scope={NO_SCOPE} />)
    expect(await screen.findByRole('heading', { name: 'Cache reuse' })).toBeInTheDocument()
    expect(await screen.findByRole('heading', { name: 'Next actions (3)' })).toBeInTheDocument()
    expect(screen.getByRole('heading', { name: 'Plan limits' })).toBeInTheDocument()
  })

  it('SC2: two warning hits give "Next actions (2)" with both titles', async () => {
    stub({ warnings: { ...warnings, hits: warnings.hits.slice(0, 2) } })
    render(<Rail scope={NO_SCOPE} />)
    const actions = await screen.findByRole('region', { name: 'Next actions (2)' })
    expect(within(actions).getAllByRole('listitem').map((li) => li.textContent)).toEqual(['Low cache reuse', 'Code index is out of date'])
  })

  it('lists at most the top 3 titles and counts all hits', async () => {
    const hit = (i) => ({ ...warnings.hits[0], rule_id: `r${i}`, title: `Title ${i}` })
    stub({ warnings: { ...warnings, hits: [1, 2, 3, 4, 5].map(hit) } })
    render(<Rail scope={NO_SCOPE} />)
    const actions = await screen.findByRole('region', { name: 'Next actions (5)' })
    expect(within(actions).getAllByRole('listitem')).toHaveLength(3)
  })

  it('no hits: says nothing to fix', async () => {
    stub({ warnings: { ...warnings, hits: [] } })
    render(<Rail scope={NO_SCOPE} />)
    expect(await screen.findByText('Nothing to fix')).toBeInTheDocument()
  })

  it('SC3: cache_hit_ratio null shows "no data" in the ring', async () => {
    stub({ summary: emptySummary })
    render(<Rail scope={NO_SCOPE} />)
    expect(await screen.findByRole('img', { name: 'Cache reuse: no data' })).toBeInTheDocument()
    expect(within(card('Cache reuse')).getByText('no data')).toBeInTheDocument()
  })

  it('AC5: the ring is an SVG with role img and an aria-label naming the ratio', async () => {
    stub()
    render(<Rail scope={NO_SCOPE} />)
    const ring = await screen.findByRole('img', { name: 'Cache reuse 91.8%, last 7 days' })
    expect(ring.tagName.toLowerCase()).toBe('svg')
  })

  it('SC4: an account at 92 % 5h uses the red tone; 42.5 % uses green', async () => {
    stub({ accounts: accountsStatus })
    render(<Rail scope={NO_SCOPE} />)
    const limits = await screen.findByRole('region', { name: 'Plan limits' })
    await within(limits).findByText('Claude personal')
    const bars = within(limits).getAllByRole('progressbar')
    const red = bars.find((b) => b.getAttribute('aria-label').startsWith('Claude personal 5h'))
    expect(red.closest('.limit')).toHaveClass('limit-red')
    const green = bars.find((b) => b.getAttribute('aria-label').startsWith('Claude default 5h'))
    expect(green.closest('.limit')).toHaveClass('limit-green')
    expect(green).toHaveAttribute('aria-valuenow', '42.5')
  })

  it('SC4: a 7d limit at 78 % uses the amber tone, 92 % the red tone, 42.5 % the green tone', async () => {
    const limits = (five, seven) => ({ five_hour: { used_percentage: five, resets_at: '2099-01-01T00:00:00Z', stale: false }, seven_day: { used_percentage: seven, resets_at: '2099-01-04T00:00:00Z', stale: false }, written_at: '2026-10-29T09:55:00Z', stale: false })
    stub({ accounts: { by: 'account', accounts: [{ id: 1, provider: 'claude', label: 'Work', status: 'confirmed', status_line: statusLine({ limits: limits(42.5, 78) }) }, { id: 2, provider: 'claude', label: 'Home', status: 'confirmed', status_line: statusLine({ limits: limits(92, 8) }) }] } })
    render(<Rail scope={NO_SCOPE} />)
    const panel = await screen.findByRole('region', { name: 'Plan limits' })
    await within(panel).findByText('Work')
    const tone = (label) => within(panel).getByRole('progressbar', { name: label }).closest('.limit')
    expect(tone('Work 7d limit 78%')).toHaveClass('limit-amber')
    expect(tone('Home 5h limit 92%')).toHaveClass('limit-red')
    expect(tone('Work 5h limit 43%')).toHaveClass('limit-green')
    expect(within(panel).getByText('Bars turn amber at 70%.')).toBeInTheDocument()
  })

  it('shows the window and the reset time on two sides of one line', async () => {
    stub({ accounts: accountsStatus })
    render(<Rail scope={NO_SCOPE} />)
    const limits = await screen.findByRole('region', { name: 'Plan limits' })
    await within(limits).findByText('Claude default')
    expect(within(limits).getByText('5h · 43%')).toBeInTheDocument()
    expect(within(limits).getAllByText(/^resets (soon|in )/).length).toBeGreaterThan(0)
  })

  it('"Open the plan →" calls onOpenPlan; it is absent when nothing handles it', async () => {
    stub()
    const open = vi.fn()
    const { unmount } = render(<Rail scope={NO_SCOPE} onOpenPlan={open} />)
    const link = await screen.findByRole('button', { name: 'Open the plan →' })
    link.click()
    expect(open).toHaveBeenCalledTimes(1)
    unmount()
    render(<Rail scope={NO_SCOPE} />)
    await screen.findByRole('heading', { name: 'Next actions (3)' })
    expect(screen.queryByRole('button', { name: 'Open the plan →' })).toBeNull()
  })

  it('shows 5h and 7d for each confirmed Claude account with data; skips codex, stale and empty ones', async () => {
    stub({ accounts: accountsStatus })
    render(<Rail scope={NO_SCOPE} />)
    const limits = await screen.findByRole('region', { name: 'Plan limits' })
    await within(limits).findByText('Claude default')
    expect(within(limits).getAllByRole('progressbar')).toHaveLength(5) // default 5h+7d, personal 5h+7d, skew 5h
    expect(within(limits).queryByText('Codex')).toBeNull()
    expect(within(limits).queryByText('Claude old')).toBeNull()
    expect(within(limits).queryByText('Claude bare')).toBeNull()
  })

  it('no account has limits: "no data"', async () => {
    stub({ accounts: { by: 'account', accounts: [{ id: 1, provider: 'claude', label: 'A', status: 'confirmed', status_line: statusLine() }] } })
    render(<Rail scope={NO_SCOPE} />)
    await waitFor(() => expect(within(card('Plan limits')).getByText('no data')).toBeInTheDocument())
  })

  it('a long account label is cut with an ellipsis and keeps its full text in title', async () => {
    const label = 'Claude account with a very long name for the company'
    const limits = { five_hour: { used_percentage: 10, resets_at: '2099-01-01T00:00:00Z', stale: false }, seven_day: null, written_at: '2026-10-29T09:55:00Z', stale: false }
    stub({ accounts: { by: 'account', accounts: [{ id: 1, provider: 'claude', label, status: 'confirmed', status_line: statusLine({ limits }) }] } })
    render(<Rail scope={NO_SCOPE} />)
    expect(await screen.findByTitle(label)).toHaveClass('rail-name')
  })

  it('edge: every request fails, each card says "no data" and nothing throws', async () => {
    vi.stubGlobal('fetch', async () => failing)
    render(<Rail scope={NO_SCOPE} />)
    await waitFor(() => {
      for (const name of ['Cache reuse', 'Next actions', 'Plan limits']) expect(within(card(name)).getByText('no data')).toBeInTheDocument()
    })
  })

  it('edge: one failing request leaves the other cards working', async () => {
    const base = fakeFetch({ accounts: accountsStatus })
    vi.stubGlobal('fetch', async (url, init) => (String(url).startsWith('/api/warnings') ? failing : base(url, init)))
    render(<Rail scope={NO_SCOPE} />)
    expect(await screen.findByRole('img', { name: 'Cache reuse 91.8%, last 7 days' })).toBeInTheDocument()
    await waitFor(() => expect(within(card('Next actions')).getByText('no data')).toBeInTheDocument())
  })

  it('AC6: only existing routes are read (summary and by-model breakdown for the card); the scope goes to summary and warnings, not to accounts', async () => {
    const f = stub({ accounts: accountsStatus })
    render(<Rail scope={{ workspace: 'work', account: '2' }} />)
    await screen.findByRole('heading', { name: 'Next actions (3)' })
    await screen.findByText('Claude default')
    const paths = [...new Set(f.calls.map((c) => c.url.split('?')[0]))].sort()
    expect(paths).toEqual(['/api/accounts', '/api/usage/breakdown', '/api/usage/summary', '/api/warnings'])
    const summaryUrl = f.calls.find((c) => c.url.startsWith('/api/usage/summary')).url
    expect(summaryUrl).toMatch(/^\/api\/usage\/summary\?since=[^&]+&workspace=work&account=2$/)
    expect(f.calls.find((c) => c.url.startsWith('/api/usage/breakdown')).url).toMatch(/^\/api\/usage\/breakdown\?by=model&since=[^&]+&workspace=work&account=2$/)
    expect(f.calls.find((c) => c.url.startsWith('/api/warnings')).url).toBe('/api/warnings?workspace=work&account=2')
    expect(f.calls.find((c) => c.url.startsWith('/api/accounts')).url).toBe('/api/accounts?by=account')
  })

  it('edge: a scope change refetches summary and warnings with the new scope', async () => {
    const f = stub()
    const { rerender } = render(<Rail scope={NO_SCOPE} />)
    await screen.findByRole('heading', { name: 'Next actions (3)' })
    rerender(<Rail scope={{ workspace: 'personal', account: '' }} />)
    await waitFor(() => expect(f.calls.some((c) => c.url === '/api/warnings?workspace=personal')).toBe(true))
    expect(f.calls.some((c) => /^\/api\/usage\/summary\?since=[^&]+&workspace=personal$/.test(c.url))).toBe(true)
  })

  it('edge: a scope change also refetches the by-model breakdown', async () => {
    const f = stub()
    const { rerender } = render(<Rail scope={NO_SCOPE} />)
    await screen.findByRole('heading', { name: 'Next actions (3)' })
    rerender(<Rail scope={{ workspace: 'personal', account: '' }} />)
    await waitFor(() => expect(f.calls.some((c) => /^\/api\/usage\/breakdown\?by=model&since=[^&]+&workspace=personal$/.test(c.url))).toBe(true))
  })
})

describe('cache reuse follows the app-wide period (HUN-196, FR-064)', () => {
  const DAY = 86400000
  const cacheCalls = (f) => f.calls.filter((c) => /^\/api\/usage\/(summary|breakdown)/.test(c.url))
  const sinceOf = (url) => {
    const raw = new URL(url, 'http://x').searchParams.get('since')
    return raw === null ? null : new Date(raw).getTime()
  }
  const model = (key, messages, input, cc, cr) => ({ ...breakdowns.model.groups[0], key, messages, input_tokens: input, cache_creation_input_tokens: cc, cache_read_input_tokens: cr })
  const withModels = (...groups) => ({ breakdowns: { ...breakdowns, model: { ...breakdowns.model, groups } } })
  // 21 / 1000 prompt tokens = 2.1 %
  const smallSummary = { ...summary, input_tokens: 79, cache_creation_input_tokens: 21, cache_read_input_tokens: 900, cache_hit_ratio: 0.9 }
  const card = () => screen.getByRole('region', { name: 'Cache reuse' })

  it('AC1: 7 days is the default; 30 days sends the 30-day since to both requests and names the period', async () => {
    const f = stub()
    const { rerender } = render(<Rail scope={NO_SCOPE} period="7" />)
    await screen.findByRole('img', { name: 'Cache reuse 91.8%, last 7 days' })
    expect(Math.abs(Date.now() - 7 * DAY - sinceOf(cacheCalls(f)[0].url))).toBeLessThan(60000)
    f.calls.length = 0
    rerender(<Rail scope={NO_SCOPE} period="30" />)
    await screen.findByRole('img', { name: 'Cache reuse 91.8%, last 30 days' })
    expect(within(card()).getByText('Share of prompt tokens read from cache, last 30 days.')).toBeInTheDocument()
    const calls = cacheCalls(f)
    expect(calls.map((c) => c.url.split('?')[0]).sort()).toEqual(['/api/usage/breakdown', '/api/usage/summary'])
    for (const c of calls) expect(Math.abs(Date.now() - 30 * DAY - sinceOf(c.url))).toBeLessThan(60000)
  })

  it('AC1: All time sends no since in either request', async () => {
    const f = stub()
    render(<Rail scope={NO_SCOPE} period="all" />)
    await screen.findByRole('img', { name: 'Cache reuse 91.8%, all time' })
    const calls = cacheCalls(f)
    expect(calls).toHaveLength(2)
    expect(calls.map((c) => c.url).sort()).toEqual(['/api/usage/breakdown?by=model', '/api/usage/summary'])
  })

  it('AC3: "Written to cache" is cache creation over prompt tokens, one decimal (21 / 1000 = 2.1%)', async () => {
    stub({ summary: smallSummary })
    render(<Rail scope={NO_SCOPE} />)
    expect(await within(await screen.findByRole('region', { name: 'Cache reuse' })).findByText('Written to cache 2.1%')).toBeInTheDocument()
  })

  it('AC4: "Lowest" names the lowest-ratio model among groups with 20 or more messages', async () => {
    stub({ summary: smallSummary, ...withModels(model('claude-opus-5-5', 40, 10, 10, 980), model('claude-haiku-5-5', 20, 175, 25, 800), model('claude-sonnet-5-5', 30, 10, 20, 970)) })
    render(<Rail scope={NO_SCOPE} />)
    expect(await within(await screen.findByRole('region', { name: 'Cache reuse' })).findByText('Lowest: claude-haiku-5-5 80.0%')).toBeInTheDocument()
  })

  it('AC4: a group with 19 messages is ignored, even when it has the lowest ratio', async () => {
    stub({ summary: smallSummary, ...withModels(model('claude-opus-5-5', 40, 10, 10, 980), model('tiny', 19, 900, 0, 100), model('claude-sonnet-5-5', 30, 40, 20, 940)) })
    render(<Rail scope={NO_SCOPE} />)
    expect(await within(await screen.findByRole('region', { name: 'Cache reuse' })).findByText('Lowest: claude-sonnet-5-5 94.0%')).toBeInTheDocument()
    expect(screen.queryByText(/tiny/)).toBeNull()
  })

  it('AC4: fewer than 2 qualifying groups leaves the "Lowest" line out (the written line stays)', async () => {
    stub({ summary: smallSummary, ...withModels(model('claude-opus-5-5', 40, 10, 10, 980), model('tiny', 19, 900, 0, 100)) })
    render(<Rail scope={NO_SCOPE} />)
    await within(await screen.findByRole('region', { name: 'Cache reuse' })).findByText('Written to cache 2.1%')
    expect(screen.queryByText(/Lowest/)).toBeNull()
  })

  it('edge: an (unknown) model group is skipped for "Lowest" and does not count as a qualifying group', async () => {
    stub({ summary: smallSummary, ...withModels(model('claude-opus-5-5', 40, 10, 10, 980), model('(unknown)', 50, 900, 0, 100)) })
    render(<Rail scope={NO_SCOPE} />)
    await within(await screen.findByRole('region', { name: 'Cache reuse' })).findByText('Written to cache 2.1%')
    expect(screen.queryByText(/Lowest/)).toBeNull()
  })

  it('AC5: no prompt tokens gives "no data" in the ring and no extra lines, no NaN or Infinity', async () => {
    stub({ summary: emptySummary, ...withModels(model('claude-opus-5-5', 40, 0, 0, 0), model('claude-haiku-5-5', 40, 0, 0, 0)) })
    render(<Rail scope={NO_SCOPE} />)
    await screen.findByRole('img', { name: 'Cache reuse: no data' })
    expect(screen.queryByText(/Written to cache/)).toBeNull()
    expect(screen.queryByText(/Lowest/)).toBeNull()
    expect(card().textContent).not.toMatch(/NaN|Infinity/)
  })

  it('edge: a model group with no prompt tokens is skipped, never NaN', async () => {
    stub({ summary: smallSummary, ...withModels(model('claude-opus-5-5', 40, 10, 10, 980), model('empty', 40, 0, 0, 0), model('claude-sonnet-5-5', 30, 40, 20, 940)) })
    render(<Rail scope={NO_SCOPE} />)
    expect(await within(await screen.findByRole('region', { name: 'Cache reuse' })).findByText('Lowest: claude-sonnet-5-5 94.0%')).toBeInTheDocument()
    expect(card().textContent).not.toMatch(/NaN|Infinity/)
  })

  it('edge: a failing breakdown keeps the ring and the written line and leaves "Lowest" out', async () => {
    const base = fakeFetch({ summary: smallSummary })
    vi.stubGlobal('fetch', async (url, init) => (String(url).startsWith('/api/usage/breakdown') ? failing : base(url, init)))
    render(<Rail scope={NO_SCOPE} />)
    expect(await screen.findByRole('img', { name: 'Cache reuse 90.0%, last 7 days' })).toBeInTheDocument()
    expect(within(card()).getByText('Written to cache 2.1%')).toBeInTheDocument()
    expect(screen.queryByText(/Lowest/)).toBeNull()
  })

  it('edge: a failing summary shows "no data" even when the breakdown works', async () => {
    const base = fakeFetch()
    vi.stubGlobal('fetch', async (url, init) => (String(url).startsWith('/api/usage/summary') ? failing : base(url, init)))
    render(<Rail scope={NO_SCOPE} />)
    await waitFor(() => expect(within(card()).getByText('no data')).toBeInTheDocument())
    expect(screen.queryByText(/Lowest/)).toBeNull()
  })
})

describe('Rail and Usage ask the same question (HUN-228, NFR-013)', () => {
  const pathOf = (c) => c.url.split('?')[0]
  const urlsOf = (f, path, by) =>
    f.calls.filter((c) => pathOf(c) === path && !c.url.includes('until=') && (by === undefined || c.url.includes(`by=${by}`))).map((c) => c.url)

  it('AC5: the Usage summary and model breakdown URLs equal the rail ones; since is a whole minute', async () => {
    vi.useFakeTimers({ toFake: ['Date'] })
    vi.setSystemTime(new Date('2026-11-05T10:20:33.456Z'))
    const f = stub()
    render(
      <>
        <UsageView scope={NO_SCOPE} period="7" />
        <Rail scope={NO_SCOPE} period="7" />
      </>,
    )
    await screen.findByRole('heading', { name: 'Totals' })
    await screen.findByRole('img', { name: 'Cache reuse 91.8%, last 7 days' })
    vi.useRealTimers()
    const summaryUrls = urlsOf(f, '/api/usage/summary')
    expect(summaryUrls).toHaveLength(2) // one from Usage, one from the rail
    expect(summaryUrls[0]).toBe(summaryUrls[1])
    expect(summaryUrls[0]).toBe('/api/usage/summary?since=2026-10-29T10%3A20%3A00.000Z')
    const modelUrls = urlsOf(f, '/api/usage/breakdown', 'model')
    expect(modelUrls).toHaveLength(2)
    expect(modelUrls[0]).toBe(modelUrls[1])
    expect(modelUrls[0]).toMatch(/since=[^&]+T\d\d%3A\d\d%3A00\.000Z$/)
  })
})
