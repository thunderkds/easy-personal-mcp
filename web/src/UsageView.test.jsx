import { render, screen, within, waitFor, fireEvent } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import UsageView from './UsageView.jsx'
import { fakeFetch as baseFakeFetch, usageWorkspaces, usageAccounts, emptySummary, emptyBreakdown, emptyOutliers, emptySessions, breakdowns, summary } from './test/fixtures.js'

const fakeFetch = (overrides = {}) =>
  baseFakeFetch({ workspaces: usageWorkspaces, accounts: usageAccounts, ...overrides })
const usageCalls = (f) => f.calls.filter((c) => c.url.startsWith('/api/usage/'))
const fmt = (n) => n.toLocaleString('en-US')

async function renderView(overrides) {
  const fetchStub = fakeFetch(overrides)
  vi.stubGlobal('fetch', fetchStub)
  render(<UsageView />)
  await screen.findByRole('heading', { name: 'Totals' })
  await screen.findByLabelText('Workspace')
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
    expect(usageCalls(f).length).toBe(7)
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

describe('workspace and account filters (HUN-70)', () => {
  const pick = (name, value) => fireEvent.change(screen.getByLabelText(name), { target: { value } })

  it('lists workspaces without orphan and confirmed Claude accounts, All by default', async () => {
    await renderView()
    const ws = screen.getByLabelText('Workspace')
    expect(ws).toHaveValue('')
    expect(within(ws).getAllByRole('option').map((o) => o.textContent)).toEqual(['All', 'Acme', 'Empty'])
    const acc = screen.getByLabelText('Account')
    expect(within(acc).getAllByRole('option').map((o) => o.textContent)).toEqual(['All', 'Claude company'])
  })

  it('sends no filter params by default', async () => {
    const f = await renderView()
    expect(usageCalls(f).every((c) => !/workspace=|account=/.test(c.url))).toBe(true)
  })

  it('a workspace pick adds workspace= to every usage call', async () => {
    const f = await renderView()
    f.calls.length = 0
    pick('Workspace', 'Acme')
    await waitFor(() => expect(usageCalls(f).length).toBe(7))
    expect(usageCalls(f).every((c) => c.url.includes('workspace=Acme') && !c.url.includes('account='))).toBe(true)
  })

  it('workspace plus account sends both params', async () => {
    const f = await renderView()
    pick('Workspace', 'Acme')
    await waitFor(() => screen.getByRole('heading', { name: 'Totals' }))
    f.calls.length = 0
    pick('Account', '1')
    await waitFor(() => expect(usageCalls(f).length).toBe(7))
    expect(usageCalls(f).every((c) => c.url.includes('workspace=Acme') && c.url.includes('account=1'))).toBe(true)
  })

  it('shows the selected filters and period in the header', async () => {
    await renderView()
    expect(screen.getByTestId('usage-scope')).toHaveTextContent('Workspace: All · Account: All · Last 7 days')
    pick('Workspace', 'Acme')
    pick('Account', '1')
    await waitFor(() =>
      expect(screen.getByTestId('usage-scope')).toHaveTextContent('Workspace: Acme · Account: Claude company · Last 7 days'),
    )
  })

  it('keeps the not-included notice with filters', async () => {
    await renderView()
    expect(screen.getByRole('note')).toHaveTextContent(/not included/i)
  })

  it('hints when the chosen workspace has no repos', async () => {
    await renderView()
    pick('Workspace', 'Empty')
    expect(await screen.findByText(/No repos in this workspace/)).toBeInTheDocument()
  })

  it('hides the dropdowns when the options request fails, usage still loads unfiltered', async () => {
    const base = fakeFetch()
    const f = async (url, init) =>
      String(url).startsWith('/api/workspaces') || String(url).startsWith('/api/accounts')
        ? { ok: false, status: 500, json: async () => ({}) }
        : base(url, init)
    vi.stubGlobal('fetch', f)
    render(<UsageView />)
    await screen.findByRole('heading', { name: 'Totals' })
    expect(screen.queryByLabelText('Workspace')).toBeNull()
    expect(screen.queryByLabelText('Account')).toBeNull()
    expect(base.calls.every((c) => !/workspace=|account=/.test(c.url))).toBe(true)
  })
})
