import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
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
      // By model shows short headers; its full names are the aria-label (asserted in its own test)
      const headers = within(sec).getAllByRole('columnheader').map((h) => h.getAttribute('aria-label') ?? h.textContent)
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

describe('layout (HUN-126)', () => {
  it('By day: one bar per day, the latest day is the full-color bar, with day labels', async () => {
    await renderView()
    const chart = within(screen.getByRole('region', { name: 'By day' })).getByRole('img', { name: /Tokens per day/ })
    const bars = chart.querySelectorAll('.bar')
    expect(bars).toHaveLength(breakdowns.day.groups.length)
    expect(bars[bars.length - 1]).toHaveClass('bar-latest')
    expect(chart.querySelectorAll('.bar-latest')).toHaveLength(1)
    expect(bars[0]).toHaveAttribute('title', expect.stringMatching(/^2026-10-\d\d: [\d,]+ tokens$/))
  })

  it('By model: short visible headers, full names for assistive tech, in a compact table', async () => {
    await renderView()
    const sec = screen.getByRole('region', { name: 'By model' })
    const headers = within(sec).getAllByRole('columnheader')
    expect(headers.map((h) => h.textContent)).toEqual(['Model', 'Total', 'Output', 'Cost'])
    expect(headers.map((h) => h.getAttribute('aria-label') ?? h.textContent)).toEqual(['Model', 'Total tokens', 'Output tokens', 'API-equivalent cost'])
    expect(within(sec).getByRole('columnheader', { name: 'API-equivalent cost' })).toHaveTextContent('Cost')
    expect(within(sec).getByRole('columnheader', { name: 'Total tokens' })).toHaveTextContent('Total')
    expect(within(sec).getByRole('columnheader', { name: 'Output tokens' })).toHaveTextContent('Output')
  })

  it('By project: one bar row per project (same names allowed), the top project in full color', async () => {
    await renderView()
    const rows = within(screen.getByRole('region', { name: 'By project' })).getAllByRole('listitem')
    expect(rows).toHaveLength(2)
    expect(rows[0]).toHaveClass('bar-row-top')
    expect(rows[1]).not.toHaveClass('bar-row-top')
    expect(within(rows[0]).getByText('app')).toBeInTheDocument()
    expect(within(rows[0]).getByText(fmt(600 + 1700 + 2800 + 39000))).toBeInTheDocument()
  })

  it('the full tables stay available behind a disclosure for By day and By project', async () => {
    await renderView()
    for (const name of ['By day', 'By project']) {
      const sec = screen.getByRole('region', { name })
      expect(within(sec).getByText('Show table').closest('details')).not.toHaveAttribute('open')
      expect(within(sec).getAllByRole('columnheader').map((h) => h.textContent)).toContain('Total tokens')
    }
  })

  it('Outlier sessions: a count and a raised row with the context value in amber', async () => {
    await renderView()
    const sec = screen.getByRole('region', { name: 'Outlier sessions' })
    expect(within(sec).getByTestId('outlier-count')).toHaveTextContent('1')
    expect(within(sec).getByText(fmt(180000))).toHaveClass('outlier-context')
  })

  it('the session timeline sits below the grids', async () => {
    await renderView()
    const timeline = screen.getByRole('region', { name: /^Session timeline/ })
    const outliers = screen.getByRole('region', { name: 'Outlier sessions' })
    expect(outliers.compareDocumentPosition(timeline) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
  })
})

describe('cost label (AC3)', () => {
  it('always says API-equivalent cost and dates the price table', async () => {
    await renderView()
    // only the By model column header may read "Cost" (its aria-label is the full name)
    expect(screen.queryAllByText(/^cost$/i).filter((el) => el.tagName !== 'TH')).toEqual([])
    expect(screen.getAllByText('API-equivalent cost').length).toBeGreaterThanOrEqual(5)
    const totals = screen.getByRole('region', { name: 'Totals' })
    expect(within(totals).getByText(/\$12\.50/)).toBeInTheDocument()
    const footnote = screen.getByTestId('cost-footnote')
    expect(footnote).toHaveTextContent(/price table 2026-10-01/)
    expect(footnote).toHaveTextContent('It is not what a subscription pays.')
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

  it('SC3: cache_hit_ratio 0.79 shows 79.0% with a bar at 79 and the cache read count', async () => {
    await renderView({ summary: { ...summary, cache_hit_ratio: 0.79 } })
    const card = kpi('Cache hit ratio')
    expect(within(card).getByText('79.0%')).toBeInTheDocument()
    expect(within(card).getByRole('progressbar', { name: 'Cache hit ratio' })).toHaveAttribute('aria-valuenow', '79')
    expect(within(card).getByText(`cache read ${fmt(summary.cache_read_input_tokens)}`)).toBeInTheDocument()
  })

  it('the KPI strip has 4 cells; the ring gauge is only in the right rail', async () => {
    await renderView()
    const strip = screen.getByTestId('kpi-strip')
    expect(within(strip).getAllByRole('region').map((r) => r.getAttribute('aria-labelledby'))).toHaveLength(4)
    expect(screen.queryByRole('region', { name: 'Cache hit gauge' })).toBeNull()
  })

  it('null cache hit ratio says no data, never 0%', async () => {
    await renderView({ summary: { ...summary, cache_hit_ratio: null } })
    expect(within(kpi('Cache hit ratio')).getByText('no data')).toBeInTheDocument()
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

describe('You / Agents series (HUN-161)', () => {
  const section = () => screen.getByRole('region', { name: 'You and agents' })
  const withUnknown = {
    ...breakdowns,
    day: {
      ...breakdowns.day,
      groups: [
        { ...breakdowns.day.groups[0], split: { typed: 500, agent: 300, unknown: 200 } },
        breakdowns.day.groups[1],
      ],
      total: { ...breakdowns.day.total, split: { typed: 50500, agent: 38400, unknown: 200 } },
    },
  }

  it('AC5: one stacked column per day with a You and an Agents segment sized by tokens', async () => {
    await renderView()
    const chart = within(section()).getByRole('img', { name: /You and agents tokens per day, 2 days/ })
    const columns = chart.querySelectorAll('.split-col')
    expect(columns).toHaveLength(breakdowns.day.groups.length)
    expect(columns[0]).toHaveAttribute('title', '2026-10-19: You 600, Agents 400')
    const [you, agents] = columns[1].querySelectorAll('.split-seg')
    expect(you).toHaveClass('split-typed')
    expect(agents).toHaveClass('split-agent')
    expect(you.style.height).toBe(`${(50000 / 88100) * 100}%`)
    expect(agents.style.height).toBe(`${(38100 / 88100) * 100}%`)
    expect(chart.getAttribute('aria-label')).toContain(`You ${fmt(50600)}`)
    expect(chart.getAttribute('aria-label')).toContain(`Agents ${fmt(38500)}`)
  })

  it('AC5: a legend names each series with its total and share; Unknown is left out while it is 0', async () => {
    await renderView()
    const legend = within(section()).getByRole('list', { name: 'Legend' })
    const items = within(legend).getAllByRole('listitem').map((li) => li.textContent)
    expect(items).toEqual([`You ${fmt(50600)} (56.8%)`, `Agents ${fmt(38500)} (43.2%)`])
    expect(within(section()).queryByText(/Unknown/)).toBeNull()
  })

  it('AC5: Unknown joins the chart, legend and table when the period has unknown tokens', async () => {
    await renderView({ breakdowns: withUnknown })
    const sec = section()
    const items = within(within(sec).getByRole('list', { name: 'Legend' })).getAllByRole('listitem')
    expect(items.map((li) => li.textContent)).toEqual([
      `You ${fmt(50500)} (56.7%)`,
      `Agents ${fmt(38400)} (43.1%)`,
      `Unknown ${fmt(200)} (0.2%)`,
    ])
    const first = within(sec).getByRole('img').querySelectorAll('.split-col')[0]
    expect(first.querySelectorAll('.split-seg')).toHaveLength(3)
    expect(first.querySelector('.split-unknown')).not.toBeNull()
    expect(within(sec).getByText(/not yet backfilled/)).toBeInTheDocument()
  })

  it('AC5: the table behind the chart lists the three numbers of each day', async () => {
    await renderView({ breakdowns: withUnknown })
    const table = within(section()).getByRole('table')
    expect(within(table).getAllByRole('columnheader').map((h) => h.textContent)).toEqual(['Day', 'You', 'Agents', 'Unknown'])
    const row = within(table).getByRole('row', { name: /2026-10-19/ })
    expect(within(row).getAllByRole('cell').map((c) => c.textContent)).toEqual(['500', '300', '200'])
  })

  it('AC5: workspace and account scope go to the same day breakdown the series reads', async () => {
    const f = await renderView({}, { workspace: 'Acme', account: '2' })
    const day = usageCalls(f).find((c) => c.url.includes('/breakdown') && c.url.includes('by=day') && !c.url.includes('until'))
    expect(day.url).toContain('workspace=Acme')
    expect(day.url).toContain('account=2')
    expect(within(section()).getByRole('img')).toBeInTheDocument()
  })

  it('shows no series when the data has no split (nothing to compare) or no days', async () => {
    const plain = { ...breakdowns, day: { ...breakdowns.day, groups: breakdowns.day.groups.map(({ split: _s, ...g }) => g), total: { ...breakdowns.day.total, split: undefined } } }
    await renderView({ breakdowns: plain })
    expect(screen.queryByRole('region', { name: 'You and agents' })).toBeNull()
  })

  it('colors come from design tokens only (no raw literal in the split rules)', () => {
    const css = readFileSync(resolve(process.cwd(), 'src/app.css'), 'utf8')
    const rules = css.split('\n').filter((line) => /\.split-/.test(line))
    expect(rules.length).toBeGreaterThan(3)
    for (const line of rules) expect(line).not.toMatch(/#[0-9a-fA-F]{3,8}\b|rgba?\(|hsla?\(/)
    expect(rules.join('\n')).toContain('var(--color-accent)')
    expect(rules.join('\n')).toContain('var(--color-amber)')
  })
})

describe('one-line figures, cost note, model names (HUN-188 AC1-AC3)', () => {
  const kpi = (name) => screen.getByRole('region', { name })
  const big = { ...breakdowns.day.total, input_tokens: 3_000_000_000, output_tokens: 100_000_000, cache_creation_input_tokens: 7_421_801 }
  const bigSummary = { ...summary, input_tokens: 3_000_000_000, output_tokens: 100_000_000, cache_creation_input_tokens: 7_421_801, cache_read_input_tokens: 0, messages: 21286 }

  it('shows a big figure compact, with the full value as title and accessible text', async () => {
    await renderView({ summary: bigSummary })
    const value = within(kpi('Total tokens')).getByText('3,107,421,801').closest('.kpi-value')
    expect(value).toHaveAttribute('title', '3,107,421,801')
    expect(value).toHaveTextContent('3.11 B3,107,421,801')
    expect(value.querySelector('[aria-hidden="true"]')).toHaveTextContent('3.11 B')
    expect(within(kpi('Messages')).getByText('21.3 K')).toBeInTheDocument()
    expect(within(kpi('Messages')).getByText('21,286').closest('.kpi-value')).toHaveAttribute('title', '21,286')
  })

  it('keeps small figures exact, with no title', async () => {
    await renderView()
    const value = within(kpi('Messages')).getByText('20')
    expect(value).toHaveClass('kpi-value')
    expect(value).not.toHaveAttribute('title')
  })

  it('cost figure is the dollar amount; abstained models go in a muted line under it', async () => {
    const cost = { usd: 2138.61, price_table_date: '2026-10-01', abstained_models: ['(unknown)', 'claude-opus-5'] }
    await renderView({ breakdowns: { ...breakdowns, day: { ...breakdowns.day, total: { ...breakdowns.day.total, api_equivalent_cost: cost } } } })
    const card = kpi('API-equivalent cost')
    expect(card.querySelector('.kpi-value')).toHaveTextContent(/^\$2138\.61$/)
    const note = within(card).getByText('abstained: (unknown), claude-opus-5')
    expect(note).toHaveClass('kpi-note')
    expect(card.querySelector('.kpi-value')).not.toContainElement(note)
  })

  it('has no muted line when no model abstained', async () => {
    await renderView()
    expect(kpi('API-equivalent cost').querySelector('.kpi-note')).toBeNull()
  })

  it('By model rows for two opus ids show different text and carry the full id as title', async () => {
    const mk = (key) => ({ ...breakdowns.model.groups[0], key })
    await renderView({ breakdowns: { ...breakdowns, model: { ...breakdowns.model, groups: [mk('claude-opus-4-1-20250805'), mk('claude-opus-5')] } } })
    const sec = screen.getByRole('region', { name: 'By model' })
    const heads = within(sec).getAllByRole('rowheader')
    expect(heads.map((h) => h.textContent)).toEqual(['claude-opus-4-1-20250805', 'claude-opus-5'])
    expect(heads.map((h) => h.getAttribute('title'))).toEqual(['claude-opus-4-1-20250805', 'claude-opus-5'])
  })

  it('CSS: the model column wraps instead of clipping, figures never wrap, colours stay tokens', () => {
    const css = readFileSync(resolve(process.cwd(), 'src/app.css'), 'utf8')
    const block = css.slice(css.indexOf('/* Usage, Memory, Kit UX */'))
    expect(block).toMatch(/\.model-table th:first-child \{[^}]*overflow-wrap: anywhere/)
    expect(block).not.toMatch(/\.model-table th:first-child \{[^}]*text-overflow: ellipsis/)
    expect(block).toMatch(/\.kpi-value \{[^}]*white-space: nowrap/)
    expect(block).not.toMatch(/#[0-9a-fA-F]{3,8}\b|rgb\(|hsl\(/)
  })
})

describe('top 10 lists (HUN-190, FR-058)', () => {
  const rows = (n, prefix) =>
    Array.from({ length: n }, (_, i) => ({
      key: `${prefix}-${i + 1}`,
      messages: 1,
      input_tokens: 100,
      output_tokens: 100,
      cache_creation_input_tokens: 100,
      cache_read_input_tokens: 100,
      api_equivalent_cost: { usd: 1, price_table_date: '2026-10-01', abstained_models: [] },
    }))
  const bd = (by, n) => ({ ...breakdowns[by], groups: rows(n, by) })
  const sessionsOf = (n) => ({
    ...emptyOutliers,
    sessions: Array.from({ length: n }, (_, i) => ({ session_id: `sess-${i + 1}`, project: 'app', tool_calls: 200 - i, max_context_tokens: 1000, reasons: ['tool_calls'] })),
  })
  const many = (n) => ({
    breakdowns: { ...breakdowns, model: bd('model', n), project: bd('project', n), tool: bd('tool', n), day: bd('day', n) },
    outliers: sessionsOf(n),
  })
  const region = (name) => screen.getByRole('region', { name })
  const bodyRows = (sec) => within(sec).getAllByRole('row').filter((r) => within(r).queryAllByRole('columnheader').length === 0)

  it('By project: 10 bar rows + "Show 15 more"; click shows 25 and "Show fewer"; click folds again (AC1)', async () => {
    await renderView(many(25))
    const sec = region('By project')
    expect(within(sec).getAllByRole('listitem')).toHaveLength(10)
    const btn = within(sec).getAllByRole('button', { name: 'Show 15 more' })[0]
    expect(btn).toHaveAttribute('aria-expanded', 'false')
    fireEvent.click(btn)
    expect(within(sec).getAllByRole('listitem')).toHaveLength(25)
    const open = within(sec).getByRole('button', { name: 'Show fewer' })
    expect(open).toHaveAttribute('aria-expanded', 'true')
    fireEvent.click(open)
    expect(within(sec).getAllByRole('listitem')).toHaveLength(10)
  })

  it('By project "Show table": 10 rows and its own button, independent of the bars (AC2)', async () => {
    await renderView(many(25))
    const sec = region('By project')
    const table = within(sec).getByRole('table')
    expect(within(table).getAllByRole('row')).toHaveLength(11)
    const buttons = within(sec).getAllByRole('button', { name: 'Show 15 more' })
    expect(buttons).toHaveLength(2)
    fireEvent.click(buttons[1])
    expect(within(table).getAllByRole('row')).toHaveLength(26)
    expect(within(sec).getAllByRole('listitem')).toHaveLength(10)
    expect(within(sec).getByRole('button', { name: 'Show fewer' })).toBeInTheDocument()
  })

  it('By model, By tool and Outlier sessions: 10 rows + "Show 15 more", then all 25 (AC2)', async () => {
    await renderView(many(25))
    for (const [name, count] of [['By model', () => bodyRows(region('By model')).length], ['By tool', () => bodyRows(region('By tool')).length], ['Outlier sessions', () => within(region('Outlier sessions')).getAllByRole('listitem').length]]) {
      expect(count()).toBe(10)
      fireEvent.click(within(region(name)).getByRole('button', { name: 'Show 15 more' }))
      expect(count()).toBe(25)
      expect(within(region(name)).getByRole('button', { name: 'Show fewer' })).toHaveAttribute('aria-expanded', 'true')
    }
  })

  it('the Outlier sessions count stays the true total while folded', async () => {
    await renderView(many(25))
    expect(screen.getByTestId('outlier-count')).toHaveTextContent('25')
  })

  it('opening one list leaves the others folded (independent)', async () => {
    await renderView(many(25))
    fireEvent.click(within(region('By model')).getByRole('button', { name: 'Show 15 more' }))
    expect(bodyRows(region('By model'))).toHaveLength(25)
    expect(bodyRows(region('By tool'))).toHaveLength(10)
    expect(within(region('Outlier sessions')).getAllByRole('listitem')).toHaveLength(10)
  })

  it('exactly 10 rows: all shown, no button; 11 rows: "Show 1 more" (AC3)', async () => {
    await renderView(many(10))
    expect(screen.queryByRole('button', { name: /Show \d+ more|Show fewer/ })).toBeNull()
    expect(within(region('By project')).getAllByRole('listitem')).toHaveLength(10)
    expect(bodyRows(region('By tool'))).toHaveLength(10)
  })
  it('11 rows: one hidden', async () => {
    await renderView(many(11))
    expect(within(region('By tool')).getByRole('button', { name: 'Show 1 more' })).toBeInTheDocument()
    expect(within(region('Outlier sessions')).getByRole('button', { name: 'Show 1 more' })).toBeInTheDocument()
  })

  it('0 rows: the existing empty text and no button (AC3)', async () => {
    await renderView({
      breakdowns: { day: emptyBreakdown('day'), model: emptyBreakdown('model'), project: emptyBreakdown('project'), tool: emptyBreakdown('tool') },
      outliers: emptyOutliers,
    })
    expect(screen.getAllByText(/No usage recorded in this period/).length).toBeGreaterThan(0)
    expect(screen.getByText(/No outlier sessions in this period/)).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: /Show \d+ more/ })).toBeNull()
  })

  it('bar widths use the total of every row, folded or not (AC4)', async () => {
    await renderView(many(25))
    const sec = region('By project')
    const width = () => within(sec).getAllByRole('listitem')[0].querySelector('.bar-fill').style.width
    expect(width()).toBe('4%')
    fireEvent.click(within(sec).getAllByRole('button', { name: 'Show 15 more' })[0])
    expect(width()).toBe('4%')
  })

  it('By day shows every day and has no button (AC5)', async () => {
    await renderView(many(25))
    const sec = region('By day')
    expect(sec.querySelectorAll('.bar')).toHaveLength(25)
    expect(within(sec).getByRole('table').querySelectorAll('tbody tr')).toHaveLength(25)
    expect(within(sec).queryByRole('button')).toBeNull()
  })

  it('changing the period folds the lists again (AC6)', async () => {
    const f = await renderView(many(25))
    fireEvent.click(within(region('By model')).getByRole('button', { name: 'Show 15 more' }))
    expect(bodyRows(region('By model'))).toHaveLength(25)
    f.calls.length = 0
    fireEvent.click(screen.getByRole('button', { name: 'All time' }))
    await waitFor(() => expect(usageCalls(f).length).toBe(7))
    await screen.findByRole('heading', { name: 'Totals' })
    expect(bodyRows(region('By model'))).toHaveLength(10)
    expect(within(region('By model')).getByRole('button', { name: 'Show 15 more' })).toHaveAttribute('aria-expanded', 'false')
  })

  it('changing the workspace folds the lists again', async () => {
    const f = await renderView(many(25))
    fireEvent.click(within(region('By tool')).getByRole('button', { name: 'Show 15 more' }))
    expect(bodyRows(region('By tool'))).toHaveLength(25)
    f.rerender({ workspace: 'Acme', account: '' })
    await waitFor(() => expect(bodyRows(region('By tool'))).toHaveLength(10))
  })

  it('the button is a focusable native <button> with the count in its text (AC7)', async () => {
    await renderView(many(25))
    const btn = within(region('By tool')).getByRole('button', { name: 'Show 15 more' })
    expect(btn.tagName).toBe('BUTTON')
    expect(btn).toHaveClass('details-toggle')
    btn.focus()
    expect(btn).toHaveFocus()
  })
})
