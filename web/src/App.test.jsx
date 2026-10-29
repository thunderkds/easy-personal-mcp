import { render, screen, fireEvent, within, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import App from './App.jsx'
import { fakeFetch, workspaceOverrides } from './test/fixtures.js'

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
  expect(await screen.findByRole('heading', { name: 'Warnings' })).toBeInTheDocument()
  fireEvent.click(within(nav()).getByRole('button', { name: 'Kit' }))
  expect(await screen.findByRole('heading', { name: 'Kit by project' })).toBeInTheDocument()
  expect(window.location.hash).toBe('#secret-token')
})

it('lists the 5 views in the sidebar and marks only the active one', () => {
  render(<App />)
  const buttons = within(nav()).getAllByRole('button')
  expect(buttons.map((b) => b.textContent)).toEqual(['Usage', 'Warnings & plan', 'Kit', 'Workspaces', 'Agents'])
  fireEvent.click(within(nav()).getByRole('button', { name: 'Kit' }))
  expect(within(nav()).getByRole('button', { name: 'Kit' })).toHaveAttribute('aria-current', 'page')
  expect(within(nav()).getByRole('button', { name: 'Usage' })).not.toHaveAttribute('aria-current')
})

it('shows the active view title in the top bar', async () => {
  render(<App />)
  const bar = screen.getByRole('banner')
  expect(within(bar).getByText('Usage')).toBeInTheDocument()
  fireEvent.click(within(nav()).getByRole('button', { name: 'Kit' }))
  expect(within(bar).getByText('Kit')).toBeInTheDocument()
  expect(await screen.findByRole('heading', { name: 'Kit by project' })).toBeInTheDocument()
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

  it('AC1: the sidebar has a Scope block with labelled Workspace and Account selects on every tab', async () => {
    stub(workspaceOverrides())
    render(<App />)
    expect(await within(screen.getByRole('complementary')).findByRole('region', { name: 'Scope' })).toBeInTheDocument()
    for (const tab of ['Usage', 'Warnings & plan', 'Kit', 'Workspaces', 'Agents']) {
      go(tab)
      expect(await select('Workspace')).toBeInTheDocument()
      expect(within(scope()).getByLabelText('Account')).toBeInTheDocument()
    }
  })

  it('lists All plus non-orphan workspaces and confirmed Claude accounts', async () => {
    render(<App />)
    expect(within(await select('Workspace')).getAllByRole('option').map((o) => o.textContent)).toEqual(['All', 'work', 'personal'])
    expect(within(await select('Account')).getAllByRole('option').map((o) => o.textContent)).toEqual(['All', 'Claude default', 'Claude personal'])
  })

  it('AC2/AC3: a choice made on Usage is kept on Kit, and Kit sends it', async () => {
    const f = stub()
    render(<App />)
    await screen.findByRole('heading', { name: 'Totals' })
    await pick('Workspace', 'personal')
    go('Kit')
    expect(await screen.findByRole('heading', { name: 'Kit by project' })).toBeInTheDocument()
    expect(await select('Workspace')).toHaveValue('personal')
    const kitCalls = f.calls.filter((c) => c.url.startsWith('/api/kit'))
    expect(kitCalls.at(-1).url).toBe('/api/kit?workspace=personal')
  })

  it('AC3: Warnings & plan sends workspace and account, and refetches when they change', async () => {
    const f = stub()
    render(<App />)
    go('Warnings & plan')
    await screen.findByRole('heading', { name: 'Warnings' })
    await pick('Workspace', 'work')
    await waitFor(() => expect(f.calls.some((c) => c.url === '/api/warnings?workspace=work')).toBe(true))
    expect(f.calls.some((c) => c.url === '/api/plan?workspace=work')).toBe(true)
    await pick('Account', '2')
    await waitFor(() => expect(f.calls.some((c) => c.url === '/api/warnings?workspace=work&account=2')).toBe(true))
    expect(f.calls.some((c) => c.url === '/api/plan?workspace=work&account=2')).toBe(true)
  })

  it('AC3: Usage requests include workspace= and account=', async () => {
    const f = stub()
    render(<App />)
    await screen.findByRole('heading', { name: 'Totals' })
    await pick('Workspace', 'work')
    await pick('Account', '1')
    await waitFor(() =>
      expect(f.calls.filter((c) => c.url.startsWith('/api/usage/') && c.url.includes('workspace=work') && c.url.includes('account=1')).length).toBeGreaterThanOrEqual(7),
    )
  })

  it('AC4: each of the three views says what it shows, All when empty', async () => {
    render(<App />)
    await screen.findByRole('heading', { name: 'Totals' })
    expect(screen.getByTestId('scope-line')).toHaveTextContent('Showing: All · All')
    await pick('Workspace', 'work')
    await pick('Account', '2')
    for (const [tab, heading] of [['Usage', 'Totals'], ['Warnings & plan', 'Warnings'], ['Kit', 'Kit by project']]) {
      go(tab)
      await screen.findByRole('heading', { name: heading })
      expect(screen.getByTestId('scope-line')).toHaveTextContent('Showing: work · Claude personal')
    }
  })

  it('AC5: remembers the choice in localStorage and restores it on the next load', async () => {
    const first = render(<App />)
    await pick('Workspace', 'personal')
    await pick('Account', '1')
    expect(JSON.parse(window.localStorage.getItem('easy-personal-scope'))).toEqual({ workspace: 'personal', account: '1' })
    first.unmount()
    const f = stub()
    render(<App />)
    expect(await select('Workspace')).toHaveValue('personal')
    expect(await select('Account')).toHaveValue('1')
    await screen.findByRole('heading', { name: 'Totals' })
    expect(f.calls.filter((c) => c.url.startsWith('/api/usage/')).every((c) => c.url.includes('workspace=personal') && c.url.includes('account=1'))).toBe(true)
  })

  it('AC5: a stored workspace or account that no longer exists falls back to All', async () => {
    window.localStorage.setItem('easy-personal-scope', JSON.stringify({ workspace: 'gone', account: '99' }))
    const f = stub()
    render(<App />)
    expect(await select('Workspace')).toHaveValue('')
    expect(await select('Account')).toHaveValue('')
    await screen.findByRole('heading', { name: 'Totals' })
    expect(screen.getByTestId('scope-line')).toHaveTextContent('Showing: All · All')
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
    await pick('Workspace', 'work')
    expect(await select('Workspace')).toHaveValue('work')
  })

  it('AC6: the top bar shows the scope as text, and the block lives in the sidebar menu', async () => {
    render(<App />)
    await screen.findByRole('heading', { name: 'Totals' })
    await pick('Workspace', 'work')
    expect(within(screen.getByRole('banner')).getByText('Scope: work · All')).toBeInTheDocument()
    expect(screen.getByRole('complementary')).toContainElement(await screen.findByRole('region', { name: 'Scope' }))
    fireEvent.click(screen.getByRole('button', { name: 'Menu' }))
    expect(screen.getByRole('complementary')).toHaveAttribute('data-open', 'true')
  })

  it('AC7: no select lives in the top bar any more', async () => {
    render(<App />)
    await screen.findByRole('heading', { name: 'Totals' })
    expect(within(screen.getByRole('banner')).queryByLabelText('Workspace')).toBeNull()
    expect(screen.getAllByLabelText('Workspace')).toHaveLength(1)
  })

  it('options request fails: the selects are hidden and the views load unscoped', async () => {
    const base = fakeFetch()
    const f = async (url, init) =>
      String(url).startsWith('/api/workspaces') || String(url).startsWith('/api/accounts') ? { ok: false, status: 500, json: async () => ({}) } : base(url, init)
    vi.stubGlobal('fetch', f)
    window.localStorage.setItem('easy-personal-scope', JSON.stringify({ workspace: 'work', account: '1' }))
    render(<App />)
    await screen.findByRole('heading', { name: 'Totals' })
    expect(screen.queryByLabelText('Workspace')).toBeNull()
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
