import { render, screen, waitFor, within } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import Rail from './Rail.jsx'
import { NO_SCOPE } from './Scope.jsx'
import { accountsStatus, emptySummary, fakeFetch, statusLine, warnings } from './test/fixtures.js'

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

  it('AC6: only the three existing routes are read; the scope goes to summary and warnings, not to accounts', async () => {
    const f = stub({ accounts: accountsStatus })
    render(<Rail scope={{ workspace: 'work', account: '2' }} />)
    await screen.findByRole('heading', { name: 'Next actions (3)' })
    await screen.findByText('Claude default')
    const paths = [...new Set(f.calls.map((c) => c.url.split('?')[0]))].sort()
    expect(paths).toEqual(['/api/accounts', '/api/usage/summary', '/api/warnings'])
    const summary = f.calls.find((c) => c.url.startsWith('/api/usage/summary')).url
    expect(summary).toMatch(/^\/api\/usage\/summary\?since=[^&]+&workspace=work&account=2$/)
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
})
