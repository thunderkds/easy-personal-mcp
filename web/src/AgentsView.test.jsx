import { render as rtlRender, screen, fireEvent, waitFor, within } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import App from './App.jsx'
import AgentsView from './AgentsView.jsx'
import { ToastProvider } from './Toast.jsx'
import { fakeFetch, accountList, accountsStatus, accountsStatusByProvider } from './test/fixtures.js'

// A view rendered alone needs the toast host App provides; App brings its own.
const render = (ui, options) => rtlRender(ui, ui.type === App ? options : { wrapper: ToastProvider, ...options })
const toasts = () => within(screen.getByTestId('toasts'))
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
const openRemoved = () => fireEvent.click(screen.getByRole('button', { name: /^Removed \(\d+\)$/ }))

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

  it('T070: a missing folder says the folder is gone and its stored usage stays, with no usage lines', async () => {
    stub()
    render(<AgentsView />)
    const r = await screen.findByRole('listitem', { name: 'Claude gone' })
    expect(within(r).getByText('The folder is gone. Its stored usage stays in reports.')).toBeInTheDocument()
    expect(within(r).queryByText(/last active/)).toBeNull()
    expect(within(r).queryByText(/Plan limits/)).toBeNull()
    expect(within(r).queryByText(/Tokens:/)).toBeNull()
  })

  it('T070: each card carries its status so a detected account can be lit', async () => {
    stub()
    render(<AgentsView />)
    expect(await screen.findByRole('listitem', { name: /claude-new/ })).toHaveAttribute('data-status', 'detected')
    expect(screen.getByRole('listitem', { name: 'Claude gone' })).toHaveAttribute('data-status', 'missing')
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
    expect(await toasts().findByRole('alert')).toHaveTextContent('claude failed')
    expect(within(r).queryByRole('alert')).toBeNull()
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
    fireEvent.click(await screen.findByRole('button', { name: 'Removed (2)' }))
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
    fireEvent.click(await screen.findByRole('button', { name: 'Removed (2)' }))
    const r = await screen.findByRole('listitem', { name: 'Claude wife' })
    fireEvent.click(within(r).getByRole('button', { name: 'Use this account' }))
    fireEvent.change(within(r).getByLabelText('Name'), { target: { value: 'Wife Claude' } })
    fireEvent.click(within(r).getByRole('button', { name: 'Confirm & name' }))
    const row6 = await screen.findByRole('listitem', { name: 'Wife Claude' })
    expect(within(row6).getByRole('status')).toHaveTextContent(/appears after the next ingest/i)
    expect(within(row6).getByText('easy-personal ingest')).toBeInTheDocument()
  })

  it('AC2: a confirmed non-default row has Rename and Remove; Rename confirms with the new label', async () => {
    const fn = providerStub()
    render(<AgentsView />)
    const r = await screen.findByRole('listitem', { name: 'Claude personal' })
    expect(within(r).getByRole('button', { name: 'Remove' })).toBeInTheDocument()
    expect(within(r).queryByRole('button', { name: 'Ignore' })).toBeNull()
    fireEvent.click(within(r).getByRole('button', { name: 'Rename' }))
    const name = within(r).getByLabelText('Name')
    expect(name).toHaveValue('Claude personal')
    fireEvent.change(name, { target: { value: 'Personal' } })
    fireEvent.click(within(r).getByRole('button', { name: 'Save name' }))
    await waitFor(() => expect(writes(fn)).toContainEqual(['POST', '/api/accounts/2/confirm', { label: 'Personal' }]))
  })

  it('T132 AC7: Remove on a confirmed row posts to ignore', async () => {
    const fn = providerStub()
    render(<AgentsView />)
    const r = await screen.findByRole('listitem', { name: 'Claude personal' })
    fireEvent.click(within(r).getByRole('button', { name: 'Remove' }))
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

  it('AC3 (default): the default account can be renamed but has no Ignore or Remove button', async () => {
    providerStub()
    render(<AgentsView />)
    const r = await screen.findByRole('listitem', { name: 'Claude default' })
    expect(within(r).getByRole('button', { name: 'Rename' })).toBeInTheDocument()
    expect(within(r).queryByRole('button', { name: 'Ignore' })).toBeNull()
    expect(within(r).queryByRole('button', { name: 'Remove' })).toBeNull()
  })

  it('AC3 (provider): By provider shows the same actions', async () => {
    const fn = providerStub()
    render(<AgentsView />)
    fireEvent.click(screen.getByRole('button', { name: 'By provider' }))
    fireEvent.click(await screen.findByRole('button', { name: 'Removed (2)' }))
    const wife = await screen.findByRole('listitem', { name: 'Claude wife' })
    expect(within(wife).getByRole('button', { name: 'Use this account' })).toBeInTheDocument()
    const personal = screen.getByRole('listitem', { name: 'Claude personal' })
    expect(within(personal).getByRole('button', { name: 'Rename' })).toBeInTheDocument()
    expect(within(personal).getByRole('button', { name: 'Remove' })).toBeInTheDocument()
    expect(within(screen.getByRole('listitem', { name: 'Claude default' })).queryByRole('button', { name: 'Remove' })).toBeNull()
    fireEvent.click(within(screen.getByRole('listitem', { name: '.codex-old' })).getByRole('button', { name: 'Use this account' }))
    fireEvent.change(screen.getByLabelText('Name'), { target: { value: 'Codex old' } })
    fireEvent.click(screen.getByRole('button', { name: 'Confirm & name' }))
    await waitFor(() => expect(writes(fn)).toContainEqual(['POST', '/api/accounts/7/confirm', { label: 'Codex old' }]))
  })

  it('edge: a missing folder has no Use / Rename / Ignore actions', async () => {
    providerStub()
    render(<AgentsView />)
    const r = await screen.findByRole('listitem', { name: 'Claude gone' })
    for (const name of ['Use this account', 'Rename', 'Ignore', 'Remove']) expect(within(r).queryByRole('button', { name })).toBeNull()
  })

  it('AC7: a refused action shows the server text in an error toast, not inline in the row', async () => {
    providerStub((m, p) => (p.endsWith('/ignore') ? new Error('the default account cannot be ignored') : {}))
    render(<AgentsView />)
    const r = await screen.findByRole('listitem', { name: 'Claude personal' })
    fireEvent.click(within(r).getByRole('button', { name: 'Remove' }))
    expect(await toasts().findByRole('alert')).toHaveTextContent('the default account cannot be ignored')
    expect(within(r).queryByRole('alert')).toBeNull()
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
    const who = within(row('Claude default')).getByText('a@b.c · subscription · Acme Org')
    expect(who.closest('p')).toHaveClass('who')
    expect(who.closest('p').querySelector('svg')).not.toBeNull() // the person mark is an icon, not an emoji
    expect(within(row('Claude personal')).getByText('me@home.test').closest('p')).toHaveClass('who')
    expect(row('Claude bare').querySelector('.who')).toBeNull()
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

describe('Agents toasts and form hints (HUN-200)', () => {
  it('shows the apply result as a status toast, not inline', async () => {
    stub({ write: (m, p) => (p === '/api/sync' ? { status: 0, lines: ['Claude personal: added work-slack'] } : {}) })
    render(<AgentsView />)
    const r = await screen.findByRole('listitem', { name: 'Claude personal' })
    fireEvent.click(within(r).getByRole('checkbox', { name: 'work' }))
    fireEvent.click(within(r).getByRole('button', { name: 'Review changes' }))
    fireEvent.click(await within(r).findByRole('button', { name: 'Confirm' }))
    expect(await toasts().findByRole('status')).toHaveTextContent('added work-slack')
    expect(within(r).queryByText(/added work-slack/)).toBeNull()
  })

  it('a failed apply result (status not 0) is an error toast', async () => {
    stub({ write: (m, p) => (p === '/api/sync' ? { status: 1, lines: ['claude refused'] } : {}) })
    render(<AgentsView />)
    const r = await screen.findByRole('listitem', { name: 'Claude personal' })
    fireEvent.click(within(r).getByRole('checkbox', { name: 'work' }))
    fireEvent.click(within(r).getByRole('button', { name: 'Review changes' }))
    fireEvent.click(await within(r).findByRole('button', { name: 'Confirm' }))
    expect(await toasts().findByRole('alert')).toHaveTextContent('claude refused')
  })

  it('the account name field is required and has a hint', async () => {
    stub()
    render(<AgentsView />)
    const r = await screen.findByRole('listitem', { name: 'Claude personal' })
    fireEvent.click(within(r).getByRole('button', { name: 'Rename' }))
    const name = within(r).getByLabelText('Name')
    expect(name).toHaveAttribute('aria-required', 'true')
    expect(name).toHaveAccessibleDescription(/shown on the account card/i)
  })

  it('shows "Reviewing…" on the picker button while the plan is asked for', async () => {
    let release
    const hold = new Promise((r) => (release = r))
    const read = fakeFetch()
    vi.stubGlobal('fetch', async (url, init = {}) => {
      if (init.method) {
        await hold
        return { ok: true, status: 200, json: async () => ({}) }
      }
      if (String(url).startsWith('/api/sync')) return { ok: true, status: 200, json: async () => PLAN }
      return read(url, init)
    })
    render(<AgentsView />)
    const r = await screen.findByRole('listitem', { name: 'Claude personal' })
    fireEvent.click(within(r).getByRole('checkbox', { name: 'work' }))
    fireEvent.click(within(r).getByRole('button', { name: 'Review changes' }))
    expect(await within(r).findByRole('button', { name: 'Reviewing…' })).toBeDisabled()
    release()
    expect(await within(r).findByRole('button', { name: 'Confirm' })).toBeInTheDocument()
  })
})

// T132: create a new account, remove (= ignore) an account, restore it from "Removed (N)".
describe('new account and remove (T132)', () => {
  const NEW_FOLDER = '/home/o/.claude-side'
  const created = { id: 8, provider: 'claude', label: '', folder: NEW_FOLDER, status: 'detected', config_dir_env: null, workspaces: [], is_default: false }

  /** One mutable account list behind both groupings; `create` answers POST /api/accounts/create (a row, or an Error for 400/409). */
  function listStub({ start = withExtra, create = () => created } = {}) {
    let list = start
    const fn = stub({
      write: (method, path, body) => {
        if (path === '/api/accounts/create') {
          const out = create(body)
          if (!(out instanceof Error)) list = [...list, out]
          return out
        }
        const m = path.match(/^\/api\/accounts\/(\d+)\/ignore$/)
        if (m) list = list.map((a) => (a.id === Number(m[1]) ? { ...a, status: 'ignored' } : a))
        return {}
      },
    })
    const inner = fn.getMockImplementation()
    fn.mockImplementation(async (url, init = {}) => {
      const u = new URL(String(url), 'http://127.0.0.1')
      if (u.pathname === '/api/accounts' && !init.method) {
        const by = u.searchParams.get('by')
        const body = by === 'provider' ? { by, providers: Object.groupBy(list, (a) => a.provider) } : { by, accounts: list }
        return { ok: true, status: 200, json: async () => body }
      }
      return inner(url, init)
    })
    return fn
  }
  const nameField = () => screen.getByLabelText('New account name')
  const createButton = () => screen.getByRole('button', { name: 'Create' })

  it('AC6: the form sits above the list, with a hint, and Create is off until the name is valid', async () => {
    listStub()
    render(<AgentsView />)
    await screen.findByRole('listitem', { name: 'Claude default' })
    expect(within(screen.getByRole('region', { name: 'New account' })).getByText(/lower-case letters, digits and dashes/i)).toBeInTheDocument()
    expect(createButton()).toBeDisabled()
    fireEvent.change(nameField(), { target: { value: 'side' } })
    expect(createButton()).toBeEnabled()
    expect(nameField().compareDocumentPosition(screen.getByRole('listitem', { name: 'Claude default' })) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
  })

  it.each(['Side', 'a b', '-side', 'a/b', '..', 'x'.repeat(31), 'é'])('AC6: %j is refused inline and nothing is sent', async (value) => {
    const fn = listStub()
    render(<AgentsView />)
    await screen.findByRole('listitem', { name: 'Claude default' })
    fireEvent.change(nameField(), { target: { value } })
    expect(nameField()).toHaveAttribute('aria-invalid', 'true')
    expect(screen.getByText(/use only lower-case letters, digits and dashes/i)).toBeInTheDocument()
    expect(createButton()).toBeDisabled()
    fireEvent.submit(nameField().closest('form'))
    expect(writes(fn)).toHaveLength(0)
  })

  it.each(['a', 'side', 'my-acct-2', '0', 'x'.repeat(30)])('AC6: %j is accepted', async (value) => {
    listStub()
    render(<AgentsView />)
    await screen.findByRole('listitem', { name: 'Claude default' })
    fireEvent.change(nameField(), { target: { value } })
    expect(nameField()).not.toHaveAttribute('aria-invalid', 'true')
    expect(createButton()).toBeEnabled()
  })

  it('AC6: Create posts the name, shows the sign-in command, and the new account shows its Name field', async () => {
    const fn = listStub()
    render(<AgentsView />)
    await screen.findByRole('listitem', { name: 'Claude default' })
    fireEvent.change(nameField(), { target: { value: 'side' } })
    fireEvent.click(createButton())
    await waitFor(() => expect(writes(fn)).toContainEqual(['POST', '/api/accounts/create', { name: 'side' }]))
    const signIn = await screen.findByTestId('signin-command')
    expect(within(signIn).getByText(`CLAUDE_CONFIG_DIR=${NEW_FOLDER} claude`)).toBeInTheDocument()
    expect(signIn).toHaveTextContent('/login')
    const r = await screen.findByRole('listitem', { name: '.claude-side' })
    expect(r).toHaveAttribute('data-status', 'detected')
    expect(within(r).getByLabelText('Name')).toBeInTheDocument()
    expect(nameField()).toHaveValue('')
  })

  it('AC6: the sign-in command copies as one line; a folder with a space or quote is single-quoted', async () => {
    const writeText = vi.fn(async () => {})
    Object.defineProperty(navigator, 'clipboard', { value: { writeText }, configurable: true })
    listStub({ create: () => ({ ...created, folder: "/home/o x/it's/.claude-side" }) })
    render(<AgentsView />)
    await screen.findByRole('listitem', { name: 'Claude default' })
    fireEvent.change(nameField(), { target: { value: 'side' } })
    fireEvent.click(createButton())
    const signIn = await screen.findByTestId('signin-command')
    fireEvent.click(within(signIn).getByRole('button', { name: 'Copy' }))
    await waitFor(() => expect(writeText).toHaveBeenCalledWith("CLAUDE_CONFIG_DIR='/home/o x/it'\\''s/.claude-side' claude"))
    expect(await within(signIn).findByText('Copied')).toBeInTheDocument()
  })

  it('AC6: a folder with a hidden character gets no command and no Copy button', async () => {
    listStub({ create: () => ({ ...created, folder: '/home/o/.claude-side\u202e' }) })
    render(<AgentsView />)
    await screen.findByRole('listitem', { name: 'Claude default' })
    fireEvent.change(nameField(), { target: { value: 'side' } })
    fireEvent.click(createButton())
    const signIn = await screen.findByTestId('signin-command')
    expect(within(signIn).queryByRole('button', { name: 'Copy' })).toBeNull()
    expect(signIn).toHaveTextContent(/copy is off/i)
  })

  it.each([
    ['409', 'a folder or file named .claude-side already exists'],
    ['400', 'name must be lower-case letters, digits and dashes'],
  ])('AC6: a %s answer shows the server text in an error toast, with no sign-in block and no reload', async (_code, message) => {
    const fn = listStub({ create: () => new Error(message) })
    render(<AgentsView />)
    await screen.findByRole('listitem', { name: 'Claude default' })
    fireEvent.change(nameField(), { target: { value: 'side' } })
    fireEvent.click(createButton())
    expect(await toasts().findByRole('alert')).toHaveTextContent(message)
    expect(screen.queryByTestId('signin-command')).toBeNull()
    expect(nameField()).toHaveValue('side')
    expect(createButton()).toBeEnabled()
    expect(fn.mock.calls.filter(([u]) => new URL(String(u), 'http://x').pathname === '/api/accounts')).toHaveLength(1)
  })

  it('AC6: a double click sends one request', async () => {
    const fn = listStub()
    render(<AgentsView />)
    await screen.findByRole('listitem', { name: 'Claude default' })
    fireEvent.change(nameField(), { target: { value: 'side' } })
    const button = createButton()
    fireEvent.click(button)
    fireEvent.click(button)
    await screen.findByTestId('signin-command')
    expect(writes(fn).filter(([, u]) => u === '/api/accounts/create')).toHaveLength(1)
  })

  it('AC7: a detected account keeps Ignore; a confirmed non-default one shows Remove, not Ignore; the default shows neither', async () => {
    listStub({ start: [...withExtra, created] })
    render(<AgentsView />)
    const detected = await screen.findByRole('listitem', { name: '.claude-side' })
    expect(within(detected).getByRole('button', { name: 'Ignore' })).toBeInTheDocument()
    expect(within(detected).queryByRole('button', { name: 'Remove' })).toBeNull()
    const confirmed = screen.getByRole('listitem', { name: 'Claude personal' })
    expect(within(confirmed).getByRole('button', { name: 'Remove' })).toBeInTheDocument()
    expect(within(confirmed).queryByRole('button', { name: 'Ignore' })).toBeNull()
    const def = screen.getByRole('listitem', { name: 'Claude default' })
    expect(within(def).queryByRole('button', { name: /^(Remove|Ignore)$/ })).toBeNull()
  })

  it('AC8: ignored accounts are out of the main list; "Removed (N)" is collapsed and lists them with Use this account', async () => {
    listStub()
    render(<AgentsView />)
    await screen.findByRole('listitem', { name: 'Claude default' })
    expect(screen.queryByRole('listitem', { name: 'Claude wife' })).toBeNull()
    expect(screen.queryByRole('listitem', { name: '.codex-old' })).toBeNull()
    const toggle = screen.getByRole('button', { name: 'Removed (2)' })
    expect(toggle).toHaveAttribute('aria-expanded', 'false')
    openRemoved()
    expect(toggle).toHaveAttribute('aria-expanded', 'true')
    const list = screen.getByRole('list', { name: 'Removed accounts' })
    for (const name of ['Claude wife', '.codex-old']) {
      expect(within(within(list).getByRole('listitem', { name })).getByRole('button', { name: 'Use this account' })).toBeInTheDocument()
    }
    expect(screen.getAllByRole('listitem', { name: 'Claude wife' })).toHaveLength(1)
    fireEvent.click(toggle)
    expect(screen.queryByRole('listitem', { name: 'Claude wife' })).toBeNull()
  })

  it('AC8: no ignored accounts means no toggle', async () => {
    listStub({ start: accountList })
    render(<AgentsView />)
    await screen.findByRole('listitem', { name: 'Claude default' })
    expect(screen.queryByRole('button', { name: /^Removed/ })).toBeNull()
  })

  it('AC8: By provider lists the ignored accounts of every provider in one Removed section, not under their provider', async () => {
    listStub()
    render(<AgentsView />)
    fireEvent.click(screen.getByRole('button', { name: 'By provider' }))
    await screen.findByRole('heading', { name: 'claude' })
    expect(screen.queryByRole('listitem', { name: 'Claude wife' })).toBeNull()
    openRemoved()
    expect(screen.getByRole('button', { name: 'Removed (2)' })).toBeInTheDocument()
    expect(within(screen.getByRole('list', { name: 'Removed accounts' })).getAllByRole('listitem')).toHaveLength(2)
    expect(screen.getAllByRole('listitem', { name: 'Claude wife' })).toHaveLength(1)
  })

  it('AC7 + AC8: Remove moves the row into Removed (N), and Use this account brings it back through the confirm flow', async () => {
    const fn = listStub({ start: accountList.map((a) => ({ ...a, is_default: a.id === 1 })) })
    render(<AgentsView />)
    fireEvent.click(within(await screen.findByRole('listitem', { name: 'Claude personal' })).getByRole('button', { name: 'Remove' }))
    await waitFor(() => expect(writes(fn)).toContainEqual(['POST', '/api/accounts/2/ignore', {}]))
    const toggle = await screen.findByRole('button', { name: 'Removed (1)' })
    expect(screen.queryByRole('listitem', { name: 'Claude personal' })).toBeNull()
    fireEvent.click(toggle)
    const r = within(screen.getByRole('list', { name: 'Removed accounts' })).getByRole('listitem', { name: 'Claude personal' })
    fireEvent.click(within(r).getByRole('button', { name: 'Use this account' }))
    fireEvent.change(within(r).getByLabelText('Name'), { target: { value: 'Claude personal' } })
    fireEvent.click(within(r).getByRole('button', { name: 'Confirm & name' }))
    await waitFor(() => expect(writes(fn)).toContainEqual(['POST', '/api/accounts/2/confirm', { label: 'Claude personal' }]))
  })
})
