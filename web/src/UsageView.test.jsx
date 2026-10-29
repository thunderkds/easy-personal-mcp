import { render, screen, within, waitFor, fireEvent } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import UsageView from './UsageView.jsx'
import { fakeFetch as baseFakeFetch, usageWorkspaces, usageAccounts, emptySummary, emptyBreakdown, emptyOutliers, emptySessions, breakdowns, summary, previousDay } from './test/fixtures.js'

const fakeFetch = (overrides = {}) =>
  baseFakeFetch({ workspaces: usageWorkspaces, accounts: usageAccounts, ...overrides })
const usageCalls = (f) => f.calls.filter((c) => c.url.startsWith('/api/usage/'))
const fmt = (n) => n.toLocaleString('en-US')

const options = {
  workspaces: usageWorkspaces.workspaces.filter((w) => !w.is_orphan),
  accounts: usageAccounts.accounts.filter((a) => a.provider === 'claude' && a.status === 'confirmed'),
  ignored: 0,
}
const NONE = { workspace: '', account: '' }

/** The scope now lives in App (HUN-118): the view takes `scope` + `options` and refetches when they change. */
async function renderView(overrides, scope = NONE) {
  const fetchStub = fakeFetch(overrides)
  vi.stubGlobal('fetch', fetchStub)
  const view = render(<UsageView scope={scope} options={options} />)
  await screen.findByRole('heading', { name: 'Totals' })
  fetchStub.rerender = (next) => view.rerender(<UsageView scope={next} options={options} />)
  return fetchStub
}

beforeEach(() => {
  window.location.hash = '#secret-token'
})
afterEach(() => {
  vi.unstubAllGlobals()
})

describe('totals (AC1)', () => {
  it('shows API numbers verbatim, tokens first', async () => {
    await renderView()
    const totals = screen.getByRole('region', { name: 'Totals' })
    for (const n of [summary.sessions, summary.messages, summary.input_tokens, summary.output_tokens, summary.cache_creation_input_tokens, summary.cache_read_input_tokens]) {
      expect(within(totals).getAllByText(fmt(n)).length).toBeGreaterThan(0)
    }
    expect(within(totals).getByText('91.8%')).toBeInTheDocument()
    // total tokens = sum of the four counters
    expect(within(totals).getByText(fmt(1200 + 3400 + 5600 + 78900))).toBeInTheDocument()
  })

  it('sends the dashboard token from the URL fragment on every request', async () => {
    const f = await renderView()
    expect(usageCalls(f).length).toBe(8)
    for (const c of f.calls) {
      expect(c.init.headers['X-Dashboard-Token']).toBe('secret-token')
      expect(c.url.startsWith('/api/')).toBe(true)
    }
  })

  it('shows the abstained record count, never as zero', async () => {
    await renderView()
    expect(screen.getByText(/4 records could not be read/)).toBeInTheDocument()
  })
})

describe('breakdowns (AC1, AC6)', () => {
  it('renders one section per dimension with tokens before cost', async () => {
    await renderView()
    for (const name of ['By day', 'By model', 'By project', 'By tool']) {
      const sec = screen.getByRole('region', { name })
      const headers = within(sec).getAllByRole('columnheader').map((h) => h.textContent)
      expect(headers[0]).toMatch(/^(Day|Model|Project|Tool)$/)
      expect(headers[1]).toBe('Total tokens')
      expect(headers.indexOf('API-equivalent cost')).toBe(headers.length - 1)
    }
  })

  it('day rows show the API total tokens', async () => {
    await renderView()
    const sec = screen.getByRole('region', { name: 'By day' })
    const row = within(sec).getByRole('row', { name: /2026-10-20/ })
    expect(within(row).getByText(fmt(1100 + 3200 + 5300 + 78500))).toBeInTheDocument()
    expect(within(sec).getByRole('img', { name: /Tokens per day/ })).toBeInTheDocument()
  })

  it('renders both project rows when two projects share a folder name', async () => {
    await renderView()
    const sec = screen.getByRole('region', { name: 'By project' })
    const rows = within(sec).getAllByRole('row').slice(1)
    expect(rows).toHaveLength(2)
    expect(within(rows[0]).getByText('app')).toBeInTheDocument()
    expect(within(rows[1]).getByText('app')).toBeInTheDocument()
    expect(within(rows[0]).getByText(fmt(600 + 1700 + 2800 + 39000))).toBeInTheDocument()
    expect(within(rows[1]).getByText(fmt(600 + 1700 + 2800 + 39900))).toBeInTheDocument()
  })

  it('notes that tool groups overlap', async () => {
    await renderView()
    expect(screen.getByText(/appears in each tool/)).toBeInTheDocument()
  })
})

describe('cost label (AC3)', () => {
  it('always says API-equivalent cost and dates the price table', async () => {
    await renderView()
    expect(screen.queryByText(/^cost$/i)).toBeNull()
    expect(screen.getAllByText('API-equivalent cost').length).toBeGreaterThanOrEqual(5)
    const totals = screen.getByRole('region', { name: 'Totals' })
    expect(within(totals).getByText(/\$12\.50/)).toBeInTheDocument()
    expect(within(totals).getByText(/price table 2026-10-01/)).toBeInTheDocument()
  })

  it('shows abstained cost as abstained, not $0', async () => {
    await renderView()
    const sec = screen.getByRole('region', { name: 'By model' })
    const row = within(sec).getByRole('row', { name: /mystery-model/ })
    expect(within(row).getByText(/abstained: mystery-model/)).toBeInTheDocument()
    expect(within(row).queryByText(/\$0/)).toBeNull()
  })
})

describe('notice (AC4)', () => {
  it('says cloud sessions and other machines are not included', async () => {
    await renderView()
    const note = screen.getByRole('note')
    expect(note).toHaveTextContent(/cloud sessions/i)
    expect(note).toHaveTextContent(/other machines/i)
    expect(note).toHaveTextContent(/not included/i)
  })
})

describe('outliers', () => {
  it('lists outlier sessions with reasons', async () => {
    await renderView()
    const sec = screen.getByRole('region', { name: 'Outlier sessions' })
    expect(within(sec).getByText('sess-outlier-1')).toBeInTheDocument()
    expect(within(sec).getByText('140')).toBeInTheDocument()
    expect(within(sec).getByText(fmt(180000))).toBeInTheDocument()
  })
})

describe('empty and error states', () => {
  it('shows a clear empty state for an empty store', async () => {
    await renderView({
      summary: emptySummary,
      breakdowns: { day: emptyBreakdown('day'), model: emptyBreakdown('model'), project: emptyBreakdown('project'), tool: emptyBreakdown('tool') },
      outliers: emptyOutliers,
      sessions: emptySessions,
    })
    expect(screen.getAllByText(/No usage recorded in this period/).length).toBeGreaterThan(0)
    expect(screen.getByText(/No sessions in this period/)).toBeInTheDocument()
    expect(screen.getByRole('note')).toBeInTheDocument()
  })

  it('shows an error when the API refuses', async () => {
    vi.stubGlobal('fetch', fakeFetch({ status: 403 }))
    render(<UsageView />)
    expect(await screen.findByRole('alert')).toHaveTextContent(/403/)
  })
})

describe('period', () => {
  it('refetches with since for a preset, and without it for All', async () => {
    const f = await renderView()
    expect(usageCalls(f).every((c) => c.url.includes('since='))).toBe(true)
    f.calls.length = 0
    screen.getByRole('button', { name: 'All time' }).click()
    await waitFor(() => expect(usageCalls(f).length).toBe(7))
    expect(usageCalls(f).every((c) => !c.url.includes('since='))).toBe(true)
  })
})

describe('workspace and account scope (HUN-70, HUN-118)', () => {
  it('sends no scope params by default', async () => {
    const f = await renderView()
    expect(usageCalls(f).every((c) => !/workspace=|account=/.test(c.url))).toBe(true)
  })

  it('a workspace in the scope adds workspace= to every usage call', async () => {
    const f = await renderView(undefined, { workspace: 'Acme', account: '' })
    await waitFor(() => expect(usageCalls(f).filter((c) => c.url.includes('workspace=Acme')).length).toBe(8))
    expect(usageCalls(f).every((c) => c.url.includes('workspace=Acme') && !c.url.includes('account='))).toBe(true)
  })

  it('refetches with both params when the scope changes', async () => {
    const f = await renderView(undefined, { workspace: 'Acme', account: '' })
    f.calls.length = 0
    f.rerender({ workspace: 'Acme', account: '1' })
    await waitFor(() => expect(usageCalls(f).length).toBe(8))
    expect(usageCalls(f).every((c) => c.url.includes('workspace=Acme') && c.url.includes('account=1'))).toBe(true)
  })

  it('shows what it shows, All when empty (AC4)', async () => {
    await renderView()
    expect(screen.getByTestId('scope-line')).toHaveTextContent('Showing: All · All')
  })

  it('names the chosen workspace and account label', async () => {
    await renderView(undefined, { workspace: 'Acme', account: '1' })
    expect(screen.getByTestId('scope-line')).toHaveTextContent('Showing: Acme · Claude company')
  })

  it('has no filter selects of its own any more (AC7)', async () => {
    await renderView()
    expect(screen.queryByLabelText('Workspace')).toBeNull()
    expect(screen.queryByLabelText('Account')).toBeNull()
  })

  it('keeps the not-included notice', async () => {
    await renderView()
    expect(screen.getByRole('note')).toHaveTextContent(/not included/i)
  })

  it('hints when the chosen workspace has no repos', async () => {
    await renderView(undefined, { workspace: 'Empty', account: '' })
    expect(await screen.findByText(/No repos in this workspace/)).toBeInTheDocument()
  })

  it('without options the view still loads unscoped', async () => {
    const f = fakeFetch()
    vi.stubGlobal('fetch', f)
    render(<UsageView />)
    await screen.findByRole('heading', { name: 'Totals' })
    expect(f.calls.every((c) => !/workspace=|account=/.test(c.url))).toBe(true)
  })
})

describe('widgets (HUN-83)', () => {
  const kpi = (name) => screen.getByRole('region', { name })
  const day = (total, groups = []) => ({ ...breakdowns, day: { ...breakdowns.day, groups, total } })
  const sevenDays = Array.from({ length: 7 }, (_, i) => ({
    ...breakdowns.day.groups[0],
    key: `2026-10-${String(20 + i)}`,
    input_tokens: 100 * (i + 1),
  }))
  const tok = (n) => ({ input_tokens: n, output_tokens: 0, cache_creation_input_tokens: 0, cache_read_input_tokens: 0 })

  it('shows 3 KPI cards with the period value from the existing responses', async () => {
    await renderView()
    expect(within(kpi('Total tokens')).getByText(fmt(89100))).toBeInTheDocument()
    expect(within(kpi('Messages')).getByText('20')).toBeInTheDocument()
    expect(within(kpi('API-equivalent cost')).getByText('$12.50')).toBeInTheDocument()
  })

  it('badges the change against the previous equal period', async () => {
    await renderView()
    expect(within(kpi('Total tokens')).getByText('↑ 100.0%')).toBeInTheDocument() // 89,100 vs 44,550
    expect(within(kpi('Messages')).getByText('↑ 25.0%')).toBeInTheDocument() // 20 vs 16
    expect(within(kpi('API-equivalent cost')).getByText('↓ 37.5%')).toBeInTheDocument() // 12.5 vs 20
  })

  it('Last 7 days: total tokens 1,500 vs 1,000 gives up 50.0%', async () => {
    await renderView({
      summary: { ...summary, ...tok(1500) },
      previousDay: { ...previousDay, total: { ...previousDay.total, ...tok(1000) } },
    })
    expect(within(kpi('Total tokens')).getByText('↑ 50.0%')).toBeInTheDocument()
  })

  it('previous period with 0 tokens says no previous data, no Infinity or NaN', async () => {
    await renderView({ previousDay: emptyBreakdown('day') })
    for (const name of ['Total tokens', 'Messages', 'API-equivalent cost']) {
      expect(within(kpi(name)).getByText('no previous data')).toBeInTheDocument()
    }
    expect(document.body.textContent).not.toMatch(/Infinity|NaN/)
  })

  it('All time sends no previous-period request and shows no previous data', async () => {
    const f = await renderView()
    f.calls.length = 0
    screen.getByRole('button', { name: 'All time' }).click()
    await waitFor(() => expect(usageCalls(f).length).toBe(7))
    expect(usageCalls(f).every((c) => !c.url.includes('until='))).toBe(true)
    expect(within(kpi('Total tokens')).getByText('no previous data')).toBeInTheDocument()
  })

  it('previous period is [now - 2 days, now - days) with the same workspace and account', async () => {
    const f = await renderView(undefined, { workspace: 'Acme', account: '' })
    f.calls.length = 0
    f.rerender({ workspace: 'Acme', account: '1' })
    await waitFor(() => expect(usageCalls(f).filter((c) => c.url.includes('until=')).length).toBe(1))
    const prev = usageCalls(f).filter((c) => c.url.includes('until=')).at(-1)
    const params = new URL(prev.url, 'http://x').searchParams
    expect(prev.url).toMatch(/^\/api\/usage\/breakdown\?/)
    expect(params.get('by')).toBe('day')
    expect(params.get('workspace')).toBe('Acme')
    expect(params.get('account')).toBe('1')
    const week = 7 * 86400000
    expect(Math.abs(Date.parse(params.get('since')) - (Date.now() - 2 * week))).toBeLessThan(5000)
    expect(Math.abs(Date.parse(params.get('until')) - (Date.now() - week))).toBeLessThan(5000)
    // the current period's own requests keep their `since` equal to the previous `until`
    const cur = usageCalls(f).filter((c) => !c.url.includes('until=')).at(-1)
    expect(Math.abs(Date.parse(new URL(cur.url, 'http://x').searchParams.get('since')) - Date.parse(params.get('until')))).toBeLessThan(5000)
  })

  it('a failing previous-period request still renders the view with no previous data', async () => {
    const base = fakeFetch()
    vi.stubGlobal('fetch', async (url, init) =>
      String(url).includes('until=') ? { ok: false, status: 500, json: async () => ({}) } : base(url, init),
    )
    render(<UsageView />)
    await screen.findByRole('heading', { name: 'Totals' })
    expect(screen.queryByRole('alert')).toBeNull()
    expect(within(kpi('Total tokens')).getByText('no previous data')).toBeInTheDocument()
  })

  it('sparkline names the days and the peak', async () => {
    await renderView({ breakdowns: day(breakdowns.day.total, sevenDays) })
    const svg = within(kpi('Total tokens')).getByRole('img')
    expect(svg).toHaveAttribute('aria-label', expect.stringMatching(/7 days/))
    expect(svg.getAttribute('aria-label')).toContain('peak 1,600') // day 7: 700 + 200 + 300 + 400
  })

  it('no day groups: values shown, no sparkline', async () => {
    await renderView({ breakdowns: day(breakdowns.day.total, []) })
    expect(within(kpi('Total tokens')).queryByRole('img')).toBeNull()
    expect(within(kpi('Total tokens')).getByText(fmt(89100))).toBeInTheDocument()
  })

  it('cache hit ratio: progress and ring show the same ratio', async () => {
    await renderView()
    const card = kpi('Cache hit ratio')
    expect(within(card).getByRole('progressbar')).toHaveAttribute('aria-valuenow', '91.84')
    const gauge = kpi('Cache hit gauge')
    expect(within(gauge).getByRole('img', { name: /91\.8%/ })).toBeInTheDocument()
  })

  it('null cache hit ratio says no data in both widgets, never 0%', async () => {
    await renderView({ summary: { ...summary, cache_hit_ratio: null } })
    expect(within(kpi('Cache hit ratio')).getByText('no data')).toBeInTheDocument()
    expect(within(kpi('Cache hit gauge')).getByText('no data')).toBeInTheDocument()
    expect(within(kpi('Cache hit ratio')).queryByRole('progressbar')).toBeNull()
  })

  it('cost KPI names abstained models and never shows $0', async () => {
    const abstainedCost = { usd: null, price_table_date: '2026-10-01', abstained_models: ['mystery-model'] }
    await renderView({
      breakdowns: day({ ...breakdowns.day.total, api_equivalent_cost: abstainedCost }, [
        { ...breakdowns.day.groups[0], api_equivalent_cost: abstainedCost },
        breakdowns.day.groups[1],
      ]),
    })
    const card = kpi('API-equivalent cost')
    expect(within(card).getByText(/abstained: mystery-model/)).toBeInTheDocument()
    expect(card.textContent).not.toMatch(/\$0/)
    expect(within(card).getByText('no previous data')).toBeInTheDocument()
    expect(within(card).getByRole('img').getAttribute('aria-label')).toContain('mystery-model')
  })
})

describe('ignored accounts note (HUN-114)', () => {
  it('AC6: says how many Claude accounts are ignored and opens the Agents view from a button', async () => {
    vi.stubGlobal('fetch', fakeFetch())
    const onOpenAgents = vi.fn()
    render(<UsageView options={{ ...options, ignored: 2 }} onOpenAgents={onOpenAgents} />)
    await screen.findByRole('heading', { name: 'Totals' })
    expect(await screen.findByText('2 agent accounts are ignored, so their usage is not counted.')).toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: 'Open Agents' }))
    expect(onOpenAgents).toHaveBeenCalledTimes(1)
  })

  it('one ignored account reads in the singular', async () => {
    vi.stubGlobal('fetch', fakeFetch())
    render(<UsageView options={{ ...options, ignored: 1 }} onOpenAgents={() => {}} />)
    expect(await screen.findByText('1 agent account is ignored, so its usage is not counted.')).toBeInTheDocument()
  })

  it('no note when no Claude account is ignored (App counts only Claude accounts, see App.test.jsx)', async () => {
    await renderView()
    expect(screen.queryByText(/ignored, so/)).toBeNull()
  })
})
