import { render, screen, fireEvent, waitFor, within } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import App from './App.jsx'
import AgentsView from './AgentsView.jsx'
import { fakeFetch, accountList, accountsStatus, accountsStatusByProvider } from './test/fixtures.js'

const PLAN = { steps: [{ account_id: 2, account: 'Claude personal', action: 'add', alias: 'work-slack', connector_id: 'c1', line: 'Claude personal: add work-slack -> exec c1' }] }

/** Reads from the fixture; writes answered by `write(method, path, body)` and recorded. */
function stub({ write = () => ({}), plan = PLAN, overrides = {} } = {}) {
  const read = fakeFetch(overrides)
  const fn = vi.fn(async (url, init = {}) => {
    const u = new URL(String(url), 'http://127.0.0.1')
    if (init.method) {
      const out = write(init.method, u.pathname, JSON.parse(init.body))
      if (out instanceof Error) return { ok: false, status: 400, json: async () => ({ error: out.message }) }
      return { ok: true, status: 200, json: async () => out }
    }
    if (u.pathname === '/api/sync') return { ok: true, status: 200, json: async () => plan }
    return read(url, init)
  })
  vi.stubGlobal('fetch', fn)
  return fn
}
const writes = (fn) => fn.mock.calls.filter(([, init]) => init?.method).map(([url, init]) => [init.method, url, JSON.parse(init.body)])
const row = (name) => screen.getByRole('listitem', { name })

beforeEach(() => {
  window.location.hash = '#secret-token'
})
afterEach(() => vi.unstubAllGlobals())

describe('Agents view', () => {
  it('is reachable from the nav as an Agents tab', async () => {
    stub()
    render(<App />)
    fireEvent.click(within(screen.getByRole('navigation', { name: 'Views' })).getByRole('button', { name: 'Agents' }))
    expect(await screen.findByRole('heading', { name: 'Agents' })).toBeInTheDocument()
  })

  it('AC1: toggle switches between account and provider grouping', async () => {
    const fn = stub()
    render(<AgentsView />)
    await screen.findByRole('listitem', { name: 'Claude default' })
    expect(screen.queryByRole('heading', { name: 'claude' })).toBeNull()
    fireEvent.click(screen.getByRole('button', { name: 'By provider' }))
    expect(await screen.findByRole('heading', { name: 'claude' })).toBeInTheDocument()
    expect(screen.getByRole('heading', { name: 'codex' })).toBeInTheDocument()
    expect(fn.mock.calls.some(([u]) => String(u).includes('by=provider'))).toBe(true)
    expect(screen.getByRole('button', { name: 'By provider' })).toHaveAttribute('aria-pressed', 'true')
  })

  it('AC2: a detected folder offers Confirm & name and Ignore', async () => {
    const fn = stub()
    render(<AgentsView />)
    const r = await screen.findByRole('listitem', { name: '.claude-new' })
    fireEvent.change(within(r).getByLabelText('Name'), { target: { value: 'Claude side' } })
    fireEvent.click(within(r).getByRole('button', { name: 'Confirm & name' }))
    await waitFor(() => expect(writes(fn)).toContainEqual(['POST', '/api/accounts/5/confirm', { label: 'Claude side' }]))
  })

  it('AC2: Ignore posts an empty body', async () => {
    const fn = stub()
    render(<AgentsView />)
    const r = await screen.findByRole('listitem', { name: '.claude-new' })
    fireEvent.click(within(r).getByRole('button', { name: 'Ignore' }))
    await waitFor(() => expect(writes(fn)).toContainEqual(['POST', '/api/accounts/5/ignore', {}]))
  })

  it('unnamed row heading is the last folder segment; the full path shows once', async () => {
    stub()
    render(<AgentsView />)
    const r = await screen.findByRole('listitem', { name: '.claude-new' })
    expect(within(r).getByText('.claude-new')).toBeInTheDocument()
    expect(within(r).getAllByText('/home/o/.claude-new')).toHaveLength(1)
  })

  it('AC2: Confirm & name needs a label', async () => {
    const fn = stub()
    render(<AgentsView />)
    const r = await screen.findByRole('listitem', { name: '.claude-new' })
    expect(within(r).getByRole('button', { name: 'Confirm & name' })).toBeDisabled()
    expect(writes(fn)).toHaveLength(0)
  })

  it('AC3: confirmed Claude row has a workspaces picker without orphan; Codex says listed only', async () => {
    stub()
    render(<AgentsView />)
    const claude = await screen.findByRole('listitem', { name: 'Claude default' })
    const group = within(claude).getByRole('group', { name: 'Workspaces' })
    expect(within(group).getByRole('checkbox', { name: 'work' })).toBeChecked()
    expect(within(group).getByRole('checkbox', { name: 'personal' })).not.toBeChecked()
    expect(within(group).queryByRole('checkbox', { name: 'orphan' })).toBeNull()
    const codex = screen.getByRole('listitem', { name: 'Codex' })
    expect(within(codex).getByText('listed only')).toBeInTheDocument()
    expect(within(codex).queryByRole('group', { name: 'Workspaces' })).toBeNull()
  })

  it('AC5: a missing folder is marked missing and has no picker', async () => {
    stub()
    render(<AgentsView />)
    const r = await screen.findByRole('listitem', { name: 'Claude gone' })
    expect(within(r).getByText('missing')).toBeInTheDocument()
    expect(within(r).queryByRole('group', { name: 'Workspaces' })).toBeNull()
  })

  it('edge: no workspaces yet shows a create-a-workspace hint', async () => {
    stub({ overrides: { workspaces: { workspaces: [{ id: 1, name: 'orphan', is_orphan: true }] } } })
    render(<AgentsView />)
    const r = await screen.findByRole('listitem', { name: 'Claude default' })
    expect(within(r).getByText(/create a workspace first/i)).toBeInTheDocument()
  })

  it('AC4 + AC6: saving picks shows the plan and writes only after Confirm', async () => {
    const fn = stub()
    render(<AgentsView />)
    const r = await screen.findByRole('listitem', { name: 'Claude personal' })
    fireEvent.click(within(r).getByRole('checkbox', { name: 'work' }))
    fireEvent.click(within(r).getByRole('button', { name: 'Review changes' }))
    expect(await within(r).findByText(/add work-slack -> exec c1/)).toBeInTheDocument()
    // picks stored, plan read, nothing applied yet
    expect(writes(fn)).toEqual([['PUT', '/api/accounts/2/workspaces', { workspaces: ['work'] }]])
    expect(fn.mock.calls.some(([u]) => String(u) === '/api/sync?account=2')).toBe(true)
    const putCall = fn.mock.calls.find(([, init]) => init?.method === 'PUT')
    expect(putCall[1].headers['X-Dashboard-Token']).toBe('secret-token')
    fireEvent.click(within(r).getByRole('button', { name: 'Confirm' }))
    await waitFor(() => expect(writes(fn)).toContainEqual(['POST', '/api/sync', { account: 2, confirm: true }]))
  })

  it('Cancel puts the previous picks back and applies nothing', async () => {
    const fn = stub()
    render(<AgentsView />)
    const r = await screen.findByRole('listitem', { name: 'Claude default' })
    fireEvent.click(within(r).getByRole('checkbox', { name: 'personal' }))
    fireEvent.click(within(r).getByRole('button', { name: 'Review changes' }))
    await within(r).findByRole('button', { name: 'Cancel' })
    fireEvent.click(within(r).getByRole('button', { name: 'Cancel' }))
    await waitFor(() => expect(writes(fn)).toHaveLength(2))
    expect(writes(fn)[1]).toEqual(['PUT', '/api/accounts/1/workspaces', { workspaces: ['work'] }])
    expect(within(r).getByRole('checkbox', { name: 'personal' })).not.toBeChecked()
  })

  it('double click on Confirm sends one apply', async () => {
    const fn = stub()
    render(<AgentsView />)
    const r = await screen.findByRole('listitem', { name: 'Claude personal' })
    fireEvent.click(within(r).getByRole('checkbox', { name: 'work' }))
    fireEvent.click(within(r).getByRole('button', { name: 'Review changes' }))
    const confirm = await within(r).findByRole('button', { name: 'Confirm' })
    fireEvent.click(confirm)
    fireEvent.click(confirm)
    await waitFor(() => expect(writes(fn).filter((w) => w[1] === '/api/sync')).toHaveLength(1))
  })

  it('apply error is shown and picks revert to server state', async () => {
    const fn = stub({ write: (m, p) => (p === '/api/sync' ? new Error('claude failed') : {}) })
    render(<AgentsView />)
    const r = await screen.findByRole('listitem', { name: 'Claude personal' })
    fireEvent.click(within(r).getByRole('checkbox', { name: 'work' }))
    fireEvent.click(within(r).getByRole('button', { name: 'Review changes' }))
    fireEvent.click(await within(r).findByRole('button', { name: 'Confirm' }))
    expect(await within(r).findByRole('alert')).toHaveTextContent('claude failed')
    await waitFor(() => expect(within(r).getByRole('checkbox', { name: 'work' })).not.toBeChecked())
    expect(writes(fn).filter((w) => w[0] === 'PUT').pop()).toEqual(['PUT', '/api/accounts/2/workspaces', { workspaces: [] }])
  })

  it('nothing to change: no Confirm, picks are kept', async () => {
    const fn = stub({ plan: { steps: [] } })
    render(<AgentsView />)
    const r = await screen.findByRole('listitem', { name: 'Claude personal' })
    fireEvent.click(within(r).getByRole('checkbox', { name: 'work' }))
    fireEvent.click(within(r).getByRole('button', { name: 'Review changes' }))
    expect(await within(r).findByText(/nothing to change/i)).toBeInTheDocument()
    expect(within(r).queryByRole('button', { name: 'Confirm' })).toBeNull()
    expect(writes(fn)).toHaveLength(1)
  })
})

// HUN-114: re-choose ignored accounts, rename / ignore confirmed ones.
const withExtra = [
  { ...accountList[0], is_default: true },
  { ...accountList[1], is_default: false },
  { ...accountList[2], is_default: false },
  { id: 6, provider: 'claude', label: 'Claude wife', folder: '/home/o/.claude-wife', status: 'ignored', config_dir_env: null, workspaces: [], is_default: false },
  { id: 7, provider: 'codex', label: null, folder: '/home/o/.codex-old', status: 'ignored', config_dir_env: null, workspaces: [], is_default: false },
]
const accountOverrides = { accounts: { by: 'account', accounts: withExtra } }
// by=provider: the fake fetch serves one override for both groupings, so serve each shape by query
function providerStub(write) {
  const base = stub({ write })
  const inner = base.getMockImplementation()
  base.mockImplementation(async (url, init = {}) => {
    const u = new URL(String(url), 'http://127.0.0.1')
    if (u.pathname === '/api/accounts' && !init.method) {
      const by = u.searchParams.get('by')
      const body =
        by === 'provider'
          ? { by, providers: { claude: withExtra.filter((a) => a.provider === 'claude'), codex: withExtra.filter((a) => a.provider === 'codex') } }
          : accountOverrides.accounts
      return { ok: true, status: 200, json: async () => body }
    }
    return inner(url, init)
  })
  return base
}

describe('re-choose and rename (HUN-114)', () => {
  it('AC1: an ignored row offers Use this account, then the Name form, then confirms with the label', async () => {
    const fn = providerStub()
    render(<AgentsView />)
    const r = await screen.findByRole('listitem', { name: 'Claude wife' })
    expect(within(r).queryByLabelText('Name')).toBeNull()
    fireEvent.click(within(r).getByRole('button', { name: 'Use this account' }))
    const name = within(r).getByLabelText('Name')
    fireEvent.change(name, { target: { value: 'Wife Claude' } })
    fireEvent.click(within(r).getByRole('button', { name: 'Confirm & name' }))
    await waitFor(() => expect(writes(fn)).toContainEqual(['POST', '/api/accounts/6/confirm', { label: 'Wife Claude' }]))
  })

  it('AC5: after a confirm the row says usage appears after the next ingest, with the command', async () => {
    providerStub((m, p) => ({}))
    // after the write the reload returns the account as confirmed
    const fn = vi.mocked(fetch)
    const inner = fn.getMockImplementation()
    let confirmed = false
    fn.mockImplementation(async (url, init = {}) => {
      if (init.method === 'POST') confirmed = true
      const u = new URL(String(url), 'http://127.0.0.1')
      if (confirmed && u.pathname === '/api/accounts' && !init.method) {
        const list = withExtra.map((a) => (a.id === 6 ? { ...a, status: 'confirmed', label: 'Wife Claude' } : a))
        return { ok: true, status: 200, json: async () => ({ by: 'account', accounts: list }) }
      }
      return inner(url, init)
    })
    render(<AgentsView />)
    const r = await screen.findByRole('listitem', { name: 'Claude wife' })
    fireEvent.click(within(r).getByRole('button', { name: 'Use this account' }))
    fireEvent.change(within(r).getByLabelText('Name'), { target: { value: 'Wife Claude' } })
    fireEvent.click(within(r).getByRole('button', { name: 'Confirm & name' }))
    const row6 = await screen.findByRole('listitem', { name: 'Wife Claude' })
    expect(within(row6).getByRole('status')).toHaveTextContent(/appears after the next ingest/i)
    expect(within(row6).getByText('easy-personal ingest')).toBeInTheDocument()
  })

  it('AC2: a confirmed non-default row has Rename and Ignore; Rename confirms with the new label', async () => {
    const fn = providerStub()
    render(<AgentsView />)
    const r = await screen.findByRole('listitem', { name: 'Claude personal' })
    expect(within(r).getByRole('button', { name: 'Ignore' })).toBeInTheDocument()
    fireEvent.click(within(r).getByRole('button', { name: 'Rename' }))
    const name = within(r).getByLabelText('Name')
    expect(name).toHaveValue('Claude personal')
    fireEvent.change(name, { target: { value: 'Personal' } })
    fireEvent.click(within(r).getByRole('button', { name: 'Save name' }))
    await waitFor(() => expect(writes(fn)).toContainEqual(['POST', '/api/accounts/2/confirm', { label: 'Personal' }]))
  })

  it('AC2: Ignore on a confirmed row posts to ignore', async () => {
    const fn = providerStub()
    render(<AgentsView />)
    const r = await screen.findByRole('listitem', { name: 'Claude personal' })
    fireEvent.click(within(r).getByRole('button', { name: 'Ignore' }))
    await waitFor(() => expect(writes(fn)).toContainEqual(['POST', '/api/accounts/2/ignore', {}]))
  })

  it('edge: renaming to an empty label is disabled; Cancel closes the form without a request', async () => {
    const fn = providerStub()
    render(<AgentsView />)
    const r = await screen.findByRole('listitem', { name: 'Claude personal' })
    fireEvent.click(within(r).getByRole('button', { name: 'Rename' }))
    fireEvent.change(within(r).getByLabelText('Name'), { target: { value: '  ' } })
    expect(within(r).getByRole('button', { name: 'Save name' })).toBeDisabled()
    fireEvent.click(within(r).getByRole('button', { name: 'Cancel' }))
    expect(within(r).queryByLabelText('Name')).toBeNull()
    expect(writes(fn)).toHaveLength(0)
  })

  it('AC3 (default): the default account can be renamed but has no Ignore button', async () => {
    providerStub()
    render(<AgentsView />)
    const r = await screen.findByRole('listitem', { name: 'Claude default' })
    expect(within(r).getByRole('button', { name: 'Rename' })).toBeInTheDocument()
    expect(within(r).queryByRole('button', { name: 'Ignore' })).toBeNull()
  })

  it('AC3 (provider): By provider shows the same actions', async () => {
    const fn = providerStub()
    render(<AgentsView />)
    fireEvent.click(screen.getByRole('button', { name: 'By provider' }))
    const wife = await screen.findByRole('listitem', { name: 'Claude wife' })
    expect(within(wife).getByRole('button', { name: 'Use this account' })).toBeInTheDocument()
    const personal = screen.getByRole('listitem', { name: 'Claude personal' })
    expect(within(personal).getByRole('button', { name: 'Rename' })).toBeInTheDocument()
    expect(within(personal).getByRole('button', { name: 'Ignore' })).toBeInTheDocument()
    expect(within(screen.getByRole('listitem', { name: 'Claude default' })).queryByRole('button', { name: 'Ignore' })).toBeNull()
    fireEvent.click(within(screen.getByRole('listitem', { name: '.codex-old' })).getByRole('button', { name: 'Use this account' }))
    fireEvent.change(screen.getByLabelText('Name'), { target: { value: 'Codex old' } })
    fireEvent.click(screen.getByRole('button', { name: 'Confirm & name' }))
    await waitFor(() => expect(writes(fn)).toContainEqual(['POST', '/api/accounts/7/confirm', { label: 'Codex old' }]))
  })

  it('edge: a missing folder has no Use / Rename / Ignore actions', async () => {
    providerStub()
    render(<AgentsView />)
    const r = await screen.findByRole('listitem', { name: 'Claude gone' })
    for (const name of ['Use this account', 'Rename', 'Ignore']) expect(within(r).queryByRole('button', { name })).toBeNull()
  })

  it('AC7: a refused action shows the server text in an alert on that row only', async () => {
    providerStub((m, p) => (p.endsWith('/ignore') ? new Error('the default account cannot be ignored') : {}))
    render(<AgentsView />)
    const r = await screen.findByRole('listitem', { name: 'Claude personal' })
    fireEvent.click(within(r).getByRole('button', { name: 'Ignore' }))
    expect(await within(r).findByRole('alert')).toHaveTextContent('the default account cannot be ignored')
    expect(within(screen.getByRole('listitem', { name: 'Claude wife' })).queryByRole('alert')).toBeNull()
  })
})

describe('Agents status card (HUN-116)', () => {
  const NOW = '2026-10-29T10:00:00Z'
  beforeEach(() => vi.useFakeTimers({ toFake: ['Date'], now: new Date(NOW) }))
  afterEach(() => vi.useRealTimers())
  async function open(by = 'account') {
    stub({ overrides: { accounts: accountsStatus, accountsByProvider: accountsStatusByProvider } })
    render(<AgentsView />)
    await screen.findByRole('listitem', { name: 'Claude default' })
    if (by === 'provider') {
      fireEvent.click(screen.getByRole('button', { name: 'By provider' }))
      await screen.findByRole('heading', { name: 'claude' })
    }
  }

  it('AC1: shows email, readable plan and organization; omits the line when all are null', async () => {
    await open()
    expect(within(row('Claude default')).getByText('👤 a@b.c · subscription · Acme Org')).toBeInTheDocument()
    expect(within(row('Claude personal')).getByText('👤 me@home.test')).toBeInTheDocument()
    expect(within(row('Claude bare')).queryByText(/👤/)).toBeNull()
  })

  it('AC2: shows last model and relative last-active time, "never" when null', async () => {
    await open()
    expect(within(row('Claude default')).getByText('claude-opus-5-5 · last active 2 h ago')).toBeInTheDocument()
    expect(within(row('Claude personal')).getByText('claude-sonnet-5-5 · last active just now')).toBeInTheDocument()
    expect(within(row('Claude bare')).getByText('last active never')).toBeInTheDocument()
  })

  it('AC3: shows tokens today and last 7 days with thousands separators', async () => {
    await open()
    expect(within(row('Claude default')).getByText('Tokens: 1,234 today · 56,789 last 7 days')).toBeInTheDocument()
  })

  it('AC4: limit bars use Progress, tone by threshold, text with reset countdown, aria-label with numbers', async () => {
    await open()
    const r = row('Claude default')
    expect(within(r).getByText('5h 43% · resets in 1h 20m')).toBeInTheDocument()
    expect(within(r).getByText('7d 75% · resets in 3d 4h')).toBeInTheDocument()
    const five = within(r).getByRole('progressbar', { name: '5h limit 43%, resets in 1h 20m' })
    expect(five).toHaveAttribute('aria-valuenow', '42.5')
    expect(five.closest('.limit')).toHaveClass('limit-green')
    expect(within(r).getByRole('progressbar', { name: '7d limit 75%, resets in 3d 4h' }).closest('.limit')).toHaveClass('limit-amber')
    const p = row('Claude personal')
    expect(within(p).getByText('5h 92% · resets in 30m')).toBeInTheDocument()
    expect(within(p).getByRole('progressbar', { name: '5h limit 92%, resets in 30m' }).closest('.limit')).toHaveClass('limit-red')
    expect(within(p).getByRole('progressbar', { name: /^7d limit 10%/ }).closest('.limit')).toHaveClass('limit-green')
  })

  it('AC4: boundaries are 70 and 90', async () => {
    const mk = (pct) => ({ ...accountsStatus.accounts[0], limits: undefined, status_line: { ...accountsStatus.accounts[0].status_line, limits: { five_hour: { used_percentage: pct, resets_at: '2026-10-29T11:00:00Z', stale: false }, seven_day: null, written_at: null, stale: false } } })
    const tone = async (pct) => {
      stub({ overrides: { accounts: { by: 'account', accounts: [mk(pct)] } } })
      const { unmount } = render(<AgentsView />)
      const bar = await screen.findByRole('progressbar')
      const cls = bar.closest('.limit').className
      unmount()
      return cls
    }
    expect(await tone(69)).toContain('limit-green')
    expect(await tone(70)).toContain('limit-amber')
    expect(await tone(89)).toContain('limit-amber')
    expect(await tone(90)).toContain('limit-red')
  })

  it('AC5: a stale snapshot says "as of <time>, reset since" and draws no bar for that window', async () => {
    await open()
    const r = row('Claude old')
    expect(within(r).getByText(`5h · as of ${new Date('2026-10-29T08:00:00Z').toLocaleString()}, reset since`)).toBeInTheDocument()
    expect(within(r).queryByRole('progressbar')).toBeNull()
  })

  it('AC5: no snapshot shows the muted README hint (Claude accounts only)', async () => {
    await open()
    expect(within(row('Claude bare')).getByText('Plan limits: add the status-line snippet (README)')).toHaveClass('muted')
    expect(within(row('Codex')).queryByText(/Plan limits/)).toBeNull()
  })

  it('edge: a window without a percentage is hidden; a reset already passed says "resets soon"', async () => {
    await open()
    expect(within(row('Claude old')).queryByText(/^7d/)).toBeNull()
    expect(within(row('Claude skew')).getByText('5h 30% · resets soon')).toBeInTheDocument()
    expect(within(row('Claude skew')).queryByText(/^7d/)).toBeNull()
  })

  it('AC6: works in By provider', async () => {
    await open('provider')
    expect(within(row('Claude personal')).getByText('5h 92% · resets in 30m')).toBeInTheDocument()
    expect(within(row('Codex')).getByText('Tokens: 5 today · 6 last 7 days')).toBeInTheDocument()
  })

  it('actions and workspace picker stay on the card', async () => {
    await open()
    const r = row('Claude default')
    expect(within(r).getByRole('button', { name: 'Rename' })).toBeInTheDocument()
    expect(within(r).getByRole('group', { name: 'Workspaces' })).toBeInTheDocument()
  })
})
