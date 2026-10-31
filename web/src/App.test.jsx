import { render, screen, fireEvent, within, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import App from './App.jsx'
import { fakeFetch, firstRun, workspaceOverrides } from './test/fixtures.js'

beforeEach(() => {
  window.location.hash = '#secret-token'
  window.localStorage.clear()
  vi.stubGlobal('fetch', fakeFetch())
})
afterEach(() => {
  vi.restoreAllMocks()
  vi.unstubAllGlobals()
})

const nav = () => screen.getByRole('navigation', { name: 'Views' })

it('opens on Usage and switches in-page between the views without changing the URL fragment', async () => {
  render(<App />)
  expect(await screen.findByRole('heading', { name: 'Totals' })).toBeInTheDocument()
  expect(within(nav()).getByRole('button', { name: 'Usage' })).toHaveAttribute('aria-current', 'page')
  fireEvent.click(within(nav()).getByRole('button', { name: 'Warnings & plan' }))
  expect(await screen.findByRole('heading', { name: 'Warnings and plan' })).toBeInTheDocument()
  fireEvent.click(within(nav()).getByRole('button', { name: 'Kit' }))
  expect(await screen.findByRole('heading', { name: /^Kit by project/ })).toBeInTheDocument()
  expect(window.location.hash).toBe('#secret-token')
})

it('lists the 6 views in the sidebar and marks only the active one', () => {
  render(<App />)
  const buttons = within(nav()).getAllByRole('button').filter((b) => !b.hasAttribute('aria-expanded'))
  expect(buttons.map((b) => b.textContent)).toEqual(['Usage', 'Warnings & plan', 'Kit', 'Workspaces', 'Agents', 'First run'])
  fireEvent.click(within(nav()).getByRole('button', { name: 'Kit' }))
  expect(within(nav()).getByRole('button', { name: 'Kit' })).toHaveAttribute('aria-current', 'page')
  expect(within(nav()).getByRole('button', { name: 'Usage' })).not.toHaveAttribute('aria-current')
})

describe('rail groups and First run (HUN-160)', () => {
  const groupLabels = () => within(nav()).getAllByRole('button').filter((b) => b.hasAttribute('aria-expanded')).map((b) => b.textContent)
  const allDone = { steps: firstRun.steps.map((s) => ({ ...s, done: true })) }

  it('shows the groups usage, kit, connect, system in that order, with work kept for later and hidden while empty', () => {
    render(<App />)
    expect(groupLabels()).toEqual(['// usage', '// kit', '// connect', '// system'])
    const groups = [...nav().querySelectorAll('.nav-group')]
    expect(groups.map((g) => g.dataset.group)).toEqual(['work', 'usage', 'kit', 'connect', 'system'])
    expect(groups[0]).toHaveAttribute('hidden')
    expect(groups[1]).not.toHaveAttribute('hidden')
  })

  it('puts each view in its group', () => {
    render(<App />)
    const inGroup = (id) => [...nav().querySelectorAll(`[data-group="${id}"] [aria-expanded] ~ .group-items button`)].map((b) => b.textContent)
    expect(inGroup('usage')).toEqual(['Usage', 'Warnings & plan'])
    expect(inGroup('kit')).toEqual(['Kit'])
    expect(inGroup('connect')).toEqual(['Workspaces', 'Agents'])
    expect(inGroup('system')).toEqual(['First run'])
  })

  it('shows the "N left" badge on First run', async () => {
    vi.stubGlobal('fetch', fakeFetch({ firstRun: { steps: firstRun.steps.map((s, i) => ({ ...s, done: i === 0 })) } }))
    render(<App />)
    expect(await within(nav()).findByText('3 left')).toBeInTheDocument()
  })

  it('shows 4 left on a fresh home', async () => {
    render(<App />)
    expect(await within(nav()).findByText('4 left')).toBeInTheDocument()
  })

  it('hides the badge when all four are done', async () => {
    const f = fakeFetch({ firstRun: allDone })
    vi.stubGlobal('fetch', f)
    render(<App />)
    await waitFor(() => expect(f.calls.some((c) => c.url === '/api/first-run')).toBe(true))
    await screen.findByRole('heading', { name: 'Totals' })
    expect(within(nav()).queryByText(/left$/)).not.toBeInTheDocument()
    expect(within(nav()).getByRole('button', { name: 'First run' })).toBeInTheDocument()
  })

  it('hides the badge when the checklist cannot be read', async () => {
    const f = fakeFetch()
    vi.stubGlobal('fetch', async (url, init) => (String(url) === '/api/first-run' ? { ok: false, status: 500, json: async () => ({}) } : f(url, init)))
    render(<App />)
    await screen.findByRole('heading', { name: 'Totals' })
    expect(within(nav()).queryByText(/left$/)).not.toBeInTheDocument()
  })

  it('opens the checklist, whose buttons switch to the Agents and Workspaces views', async () => {
    render(<App />)
    fireEvent.click(within(nav()).getByRole('button', { name: /^First run/ }))
    expect(within(nav()).getByRole('button', { name: /^First run/ })).toHaveAttribute('aria-current', 'page')
    expect(await screen.findByText('easy-personal ingest')).toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: 'Open Agents' }))
    expect(within(nav()).getByRole('button', { name: 'Agents' })).toHaveAttribute('aria-current', 'page')
    fireEvent.click(within(nav()).getByRole('button', { name: /^First run/ }))
    fireEvent.click(await screen.findByRole('button', { name: 'Open Workspaces' }))
    expect(within(nav()).getByRole('button', { name: 'Workspaces' })).toHaveAttribute('aria-current', 'page')
    expect(window.location.hash).toBe('#secret-token')
  })

  it('reads the checklist again when you come back to it', async () => {
    const f = fakeFetch()
    vi.stubGlobal('fetch', f)
    render(<App />)
    await within(nav()).findByText('4 left')
    const before = f.calls.filter((c) => c.url === '/api/first-run').length
    fireEvent.click(within(nav()).getByRole('button', { name: /^First run/ }))
    await waitFor(() => expect(f.calls.filter((c) => c.url === '/api/first-run').length).toBeGreaterThan(before))
  })

  it('only ever reads /api/first-run with GET', async () => {
    const f = fakeFetch()
    vi.stubGlobal('fetch', f)
    render(<App />)
    await within(nav()).findByText('4 left')
    expect(f.calls.filter((c) => c.url === '/api/first-run').every((c) => !c.init.method)).toBe(true)
  })
})

it('shows the active view title in the top bar', async () => {
  render(<App />)
  const bar = screen.getByRole('banner')
  expect(within(bar).getByText('Usage')).toBeInTheDocument()
  fireEvent.click(within(nav()).getByRole('button', { name: 'Kit' }))
  expect(within(bar).getByText('Kit')).toBeInTheDocument()
  expect(await screen.findByRole('heading', { name: /^Kit by project/ })).toBeInTheDocument()
})

it('has a menu button that toggles aria-expanded and closes after choosing a view', () => {
  render(<App />)
  const menu = screen.getByRole('button', { name: 'Menu' })
  expect(menu).toHaveAttribute('aria-expanded', 'false')
  fireEvent.click(menu)
  expect(menu).toHaveAttribute('aria-expanded', 'true')
  fireEvent.click(within(nav()).getByRole('button', { name: 'Agents' }))
  expect(menu).toHaveAttribute('aria-expanded', 'false')
})

describe('global scope selector (HUN-118)', () => {
  const scopeWorkspaces = {
    workspaces: [
      { id: 'o', name: 'orphan', is_orphan: true, repos: [] },
      { id: 'w1', name: 'work', is_orphan: false, repos: ['/w'] },
      { id: 'w2', name: 'personal', is_orphan: false, repos: ['/p'] },
    ],
  }
  const scopeAccounts = {
    accounts: [
      { id: 1, provider: 'claude', label: 'Claude default', status: 'confirmed' },
      { id: 2, provider: 'claude', label: 'Claude personal', status: 'confirmed' },
      { id: 3, provider: 'claude', label: 'Claude detected', status: 'detected' },
      { id: 4, provider: 'codex', label: 'Codex', status: 'confirmed' },
    ],
  }
  beforeEach(() => stub())
  const scope = () => screen.getByRole('region', { name: 'Scope' })
  const select = async (name) => within(await screen.findByRole('region', { name: 'Scope' })).findByLabelText(name)
  const pick = async (name, value) => fireEvent.change(await select(name), { target: { value } })
  const go = (tab) => fireEvent.click(within(nav()).getByRole('button', { name: tab }))
  const stub = (overrides) => {
    const f = fakeFetch({ workspaces: scopeWorkspaces, accounts: scopeAccounts, ...overrides })
    vi.stubGlobal('fetch', f)
    return f
  }

  it('AC1: the sidebar has a Scope block with the selects labelled workspace and account on every tab', async () => {
    stub(workspaceOverrides())
    render(<App />)
    expect(await within(screen.getByRole('complementary')).findByRole('region', { name: 'Scope' })).toBeInTheDocument()
    for (const tab of ['Usage', 'Warnings & plan', 'Kit', 'Workspaces', 'Agents']) {
      go(tab)
      expect(await select('workspace')).toBeInTheDocument()
      expect(within(scope()).getByLabelText('account')).toBeInTheDocument()
    }
  })

  it('lists All plus non-orphan workspaces and confirmed Claude accounts', async () => {
    render(<App />)
    expect(within(await select('workspace')).getAllByRole('option').map((o) => o.textContent)).toEqual(['All workspaces', 'work', 'personal'])
    expect(within(await select('account')).getAllByRole('option').map((o) => o.textContent)).toEqual(['All accounts', 'Claude default', 'Claude personal'])
  })

  it('AC2/AC3: a choice made on Usage is kept on Kit, and Kit sends it', async () => {
    const f = stub()
    render(<App />)
    await screen.findByRole('heading', { name: 'Totals' })
    await pick('workspace', 'personal')
    go('Kit')
    expect(await screen.findByRole('heading', { name: /^Kit by project/ })).toBeInTheDocument()
    expect(await select('workspace')).toHaveValue('personal')
    const kitCalls = f.calls.filter((c) => c.url.startsWith('/api/kit'))
    expect(kitCalls.at(-1).url).toBe('/api/kit?workspace=personal')
  })

  it('AC3: Warnings & plan sends workspace and account, and refetches when they change', async () => {
    const f = stub()
    render(<App />)
    go('Warnings & plan')
    await screen.findByTestId('headline')
    await pick('workspace', 'work')
    await waitFor(() => expect(f.calls.some((c) => c.url === '/api/warnings?workspace=work')).toBe(true))
    expect(f.calls.some((c) => c.url.startsWith('/api/plan'))).toBe(false) // the cards come from warnings
    await pick('account', '2')
    await waitFor(() => expect(f.calls.some((c) => c.url === '/api/warnings?workspace=work&account=2')).toBe(true))
  })

  it('AC3: Usage requests include workspace= and account=', async () => {
    const f = stub()
    render(<App />)
    await screen.findByRole('heading', { name: 'Totals' })
    await pick('workspace', 'work')
    await pick('account', '1')
    await waitFor(() =>
      expect(f.calls.filter((c) => c.url.startsWith('/api/usage/') && c.url.includes('workspace=work') && c.url.includes('account=1')).length).toBeGreaterThanOrEqual(7),
    )
  })

  it('AC4: the header of each of the three views names the scope, All workspaces / All accounts when empty', async () => {
    render(<App />)
    await screen.findByRole('heading', { name: 'Totals' })
    expect(await within(screen.getByRole('banner')).findByText('scope: All workspaces · All accounts')).toBeInTheDocument()
    await pick('workspace', 'work')
    await pick('account', '2')
    for (const [tab, heading] of [['Usage', 'Totals'], ['Warnings & plan', 'Warnings and plan'], ['Kit', /^Kit by project/]]) {
      go(tab)
      await screen.findByRole('heading', { name: heading })
      expect(within(screen.getByRole('banner')).getByText('scope: work · Claude personal')).toBeInTheDocument()
    }
  })

  it('AC5: remembers the choice in localStorage and restores it on the next load', async () => {
    const first = render(<App />)
    await pick('workspace', 'personal')
    await pick('account', '1')
    expect(JSON.parse(window.localStorage.getItem('easy-personal-scope'))).toEqual({ workspace: 'personal', account: '1' })
    first.unmount()
    const f = stub()
    render(<App />)
    expect(await select('workspace')).toHaveValue('personal')
    expect(await select('account')).toHaveValue('1')
    await screen.findByRole('heading', { name: 'Totals' })
    expect(f.calls.filter((c) => c.url.startsWith('/api/usage/')).every((c) => c.url.includes('workspace=personal') && c.url.includes('account=1'))).toBe(true)
  })

  it('AC5: a stored workspace or account that no longer exists falls back to All', async () => {
    window.localStorage.setItem('easy-personal-scope', JSON.stringify({ workspace: 'gone', account: '99' }))
    const f = stub()
    render(<App />)
    expect(await select('workspace')).toHaveValue('')
    expect(await select('account')).toHaveValue('')
    await screen.findByRole('heading', { name: 'Totals' })
    expect(within(screen.getByRole('banner')).getByText('scope: All workspaces · All accounts')).toBeInTheDocument()
    expect(f.calls.every((c) => !/workspace=|account=/.test(c.url.replace('/api/workspaces', '').replace('/api/accounts', '')))).toBe(true)
  })

  it('works when localStorage throws', async () => {
    vi.spyOn(Storage.prototype, 'getItem').mockImplementation(() => {
      throw new Error('denied')
    })
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
      throw new Error('denied')
    })
    render(<App />)
    await screen.findByRole('heading', { name: 'Totals' })
    await pick('workspace', 'work')
    expect(await select('workspace')).toHaveValue('work')
  })

  it('AC6: the header shows the scope as text, and the block lives in the sidebar menu', async () => {
    render(<App />)
    await screen.findByRole('heading', { name: 'Totals' })
    await pick('workspace', 'work')
    expect(within(screen.getByRole('banner')).getByText('scope: work · All accounts')).toBeInTheDocument()
    expect(screen.getByRole('complementary')).toContainElement(await screen.findByRole('region', { name: 'Scope' }))
    fireEvent.click(screen.getByRole('button', { name: 'Menu' }))
    expect(screen.getByRole('complementary')).toHaveAttribute('data-open', 'true')
  })

  it('AC7: no select lives in the top bar any more', async () => {
    render(<App />)
    await screen.findByRole('heading', { name: 'Totals' })
    expect(within(screen.getByRole('banner')).queryByLabelText('workspace')).toBeNull()
    expect(screen.getAllByLabelText('workspace')).toHaveLength(1)
  })

  it('options request fails: the selects are hidden and the views load unscoped', async () => {
    const base = fakeFetch()
    const f = async (url, init) =>
      String(url).startsWith('/api/workspaces') || String(url).startsWith('/api/accounts') ? { ok: false, status: 500, json: async () => ({}) } : base(url, init)
    vi.stubGlobal('fetch', f)
    window.localStorage.setItem('easy-personal-scope', JSON.stringify({ workspace: 'work', account: '1' }))
    render(<App />)
    await screen.findByRole('heading', { name: 'Totals' })
    expect(screen.queryByLabelText('workspace')).toBeNull()
    expect(base.calls.every((c) => !/workspace=|account=/.test(c.url))).toBe(true)
  })

  it('keeps the ignored-accounts note working (HUN-114)', async () => {
    stub({ accounts: { accounts: [{ id: 1, provider: 'claude', label: 'A', status: 'confirmed' }, { id: 7, provider: 'claude', label: 'Z', status: 'ignored' }] } })
    render(<App />)
    expect(await screen.findByText('1 agent account is ignored, so its usage is not counted.')).toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: 'Open Agents' }))
    expect(within(nav()).getByRole('button', { name: 'Agents' })).toHaveAttribute('aria-current', 'page')
  })
})

describe('grouped sidebar and right rail (HUN-121, HUN-126)', () => {
  const groupButton = (name) => within(nav()).getByRole('button', { name })

  it('AC3: "// usage" and "// connect" are aria-expanded toggles that hold their views', () => {
    render(<App />)
    expect(groupButton('// usage')).toHaveAttribute('aria-expanded', 'true')
    expect(groupButton('// connect')).toHaveAttribute('aria-expanded', 'true')
    const usage = document.getElementById(groupButton('// usage').getAttribute('aria-controls'))
    const connect = document.getElementById(groupButton('// connect').getAttribute('aria-controls'))
    expect([...usage.querySelectorAll('button')].map((b) => b.textContent)).toEqual(['Usage', 'Warnings & plan'])
    expect([...connect.querySelectorAll('button')].map((b) => b.textContent)).toEqual(['Workspaces', 'Agents'])
  })

  it('SC2: collapsing "// connect" hides Workspaces and Agents; opening it shows them again', () => {
    render(<App />)
    fireEvent.click(groupButton('// connect'))
    expect(groupButton('// connect')).toHaveAttribute('aria-expanded', 'false')
    expect(within(nav()).queryByRole('button', { name: 'Workspaces' })).toBeNull()
    expect(within(nav()).queryByRole('button', { name: 'Agents' })).toBeNull()
    expect(within(nav()).getByRole('button', { name: 'Usage' })).toBeInTheDocument()
    fireEvent.click(groupButton('// connect'))
    expect(within(nav()).getByRole('button', { name: 'Agents' })).toBeInTheDocument()
  })

  it('SC1: on default load the header reads "scope: All workspaces · All accounts"', async () => {
    render(<App />)
    expect(await within(screen.getByRole('banner')).findByText('scope: All workspaces · All accounts')).toBeInTheDocument()
  })

  it('AC3: the sidebar has the brand, the scope selects labelled in lowercase, and the "// family" links', async () => {
    render(<App />)
    const side = screen.getByRole('complementary')
    expect(within(side).getByText('Easy Personal')).toBeInTheDocument()
    const scope = await within(side).findByRole('region', { name: 'Scope' })
    expect(within(scope).getByLabelText('workspace')).toBeInTheDocument()
    expect(within(scope).getByLabelText('account')).toBeInTheDocument()
    expect(within(side).getByText('// family')).toBeInTheDocument()
    const links = within(side).getAllByRole('link')
    expect(links.map((a) => [a.textContent, a.getAttribute('href')])).toEqual([
      ['Easy Kit', 'https://personal-agentic-claude.vercel.app/'],
      ['Easy Memory', 'https://easy-memory.vercel.app/'],
      ['Easy Verifier', 'https://easy-verifier.vercel.app/'],
    ])
    for (const a of links) expect(a).toHaveAttribute('rel', 'noopener noreferrer')
  })

  it('AC3: the header holds the menu button, the view title and the scope line, and has no view h1 hidden away', async () => {
    render(<App />)
    const bar = screen.getByRole('banner')
    expect(within(bar).getByRole('button', { name: 'Menu' })).toBeInTheDocument()
    expect(within(bar).getByText('Usage')).toBeInTheDocument()
    expect(await within(bar).findByText(/^scope: /)).toBeInTheDocument()
    fireEvent.click(within(nav()).getByRole('button', { name: 'Workspaces' }))
    expect(within(bar).queryByText(/^scope: /)).toBeNull() // Workspaces and Agents ignore the scope
  })

  it('AC3: "Open the plan →" in the rail opens Warnings & plan', async () => {
    render(<App />)
    fireEvent.click(await screen.findByRole('button', { name: 'Open the plan →' }))
    expect(within(nav()).getByRole('button', { name: 'Warnings & plan' })).toHaveAttribute('aria-current', 'page')
  })

  it('AC3: each view item has a colored dot, hidden from screen readers', () => {
    render(<App />)
    for (const name of ['Usage', 'Warnings & plan', 'Kit', 'Workspaces', 'Agents']) {
      const dot = within(nav()).getByRole('button', { name }).querySelector('.dot')
      expect(dot).not.toBeNull()
      expect(dot).toHaveAttribute('aria-hidden', 'true')
    }
  })

  it('AC3: the Scope block sits above the navigation', async () => {
    render(<App />)
    const scope = await screen.findByRole('region', { name: 'Scope' })
    expect(scope.compareDocumentPosition(nav()) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
  })

  it('AC5: the rail shows its three cards beside every view', async () => {
    render(<App />)
    expect(await screen.findByRole('heading', { name: 'Cache reuse' })).toBeInTheDocument()
    expect(await screen.findByRole('heading', { name: 'Next actions (3)' })).toBeInTheDocument()
    expect(screen.getByRole('heading', { name: 'Plan limits' })).toBeInTheDocument()
    fireEvent.click(within(nav()).getByRole('button', { name: 'Agents' }))
    expect(await screen.findByRole('heading', { name: 'Cache reuse' })).toBeInTheDocument()
  })

  it('AC6: the rail asks for the chosen scope', async () => {
    const f = fakeFetch({ workspaces: { workspaces: [{ id: 'w1', name: 'Acme', is_orphan: false, repos: ['/a'] }] } })
    vi.stubGlobal('fetch', f)
    window.localStorage.setItem('easy-personal-scope', JSON.stringify({ workspace: 'Acme', account: '' }))
    render(<App />)
    await screen.findByRole('heading', { name: 'Next actions (3)' })
    expect(f.calls.some((c) => c.url === '/api/warnings?workspace=Acme')).toBe(true)
  })

  describe('AC9: Agents changes reload the scope options', () => {
    const accountsBy = (list) => ({ by: 'account', accounts: list })
    const acc = (id, label, status) => ({ id, provider: 'claude', label, folder: `/h/.c${id}`, status, workspaces: [], status_line: null })

    it('confirming a detected account adds it to the sidebar Account list', async () => {
      let list = [acc(1, 'Claude default', 'confirmed'), acc(5, '', 'detected')]
      const read = fakeFetch()
      const f = vi.fn(async (url, init = {}) => {
        if (init.method === 'POST') {
          list = [acc(1, 'Claude default', 'confirmed'), acc(5, 'Claude side', 'confirmed')]
          return { ok: true, status: 200, json: async () => ({}) }
        }
        if (String(url).startsWith('/api/accounts')) return { ok: true, status: 200, json: async () => accountsBy(list) }
        return read(url, init)
      })
      vi.stubGlobal('fetch', f)
      render(<App />)
      fireEvent.click(within(nav()).getByRole('button', { name: 'Agents' }))
      const row = await screen.findByRole('listitem', { name: '.c5' })
      const accountSelect = await within(await screen.findByRole('region', { name: 'Scope' })).findByLabelText('account')
      expect(within(accountSelect).getAllByRole('option').map((o) => o.textContent)).toEqual(['All accounts', 'Claude default'])
      fireEvent.change(within(row).getByLabelText('Name'), { target: { value: 'Claude side' } })
      fireEvent.click(within(row).getByRole('button', { name: 'Confirm & name' }))
      await waitFor(() => expect(within(accountSelect).getAllByRole('option').map((o) => o.textContent)).toEqual(['All accounts', 'Claude default', 'Claude side']))
    })

    it('ignoring an account updates the sidebar list and the ignored-accounts note without a reload', async () => {
      let list = [acc(1, 'Claude default', 'confirmed'), acc(2, 'Claude personal', 'confirmed')]
      const read = fakeFetch()
      vi.stubGlobal(
        'fetch',
        vi.fn(async (url, init = {}) => {
          if (init.method === 'POST') {
            list = [acc(1, 'Claude default', 'confirmed'), acc(2, 'Claude personal', 'ignored')]
            return { ok: true, status: 200, json: async () => ({}) }
          }
          if (String(url).startsWith('/api/accounts')) return { ok: true, status: 200, json: async () => accountsBy(list) }
          return read(url, init)
        }),
      )
      render(<App />)
      expect(await screen.findByText('Claude personal', { selector: 'option' })).toBeInTheDocument()
      fireEvent.click(within(nav()).getByRole('button', { name: 'Agents' }))
      const row = await screen.findByRole('listitem', { name: 'Claude personal' })
      fireEvent.click(within(row).getByRole('button', { name: 'Ignore' }))
      await waitFor(() => expect(screen.queryByText('Claude personal', { selector: 'option' })).toBeNull())
      fireEvent.click(within(nav()).getByRole('button', { name: 'Usage' }))
      expect(await screen.findByText('1 agent account is ignored, so its usage is not counted.')).toBeInTheDocument()
    })
  })
})
