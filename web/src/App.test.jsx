import { render, screen, fireEvent, within, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import App from './App.jsx'
import { act } from '@testing-library/react'
import { readToken } from './api.js'
import { agentRunOff, agentRunOn, fakeFetch, firstRun, workspaceOverrides } from './test/fixtures.js'

beforeEach(() => {
  window.location.hash = '#secret-token'
  window.localStorage.clear()
  vi.stubGlobal('fetch', fakeFetch())
})
afterEach(() => {
  vi.restoreAllMocks()
  vi.unstubAllGlobals()
})

// The N of M count is two spans (N is in accent mono): match the outer span by its whole text.
const count = (text) => (_, el) => el.tagName === 'SPAN' && el.textContent === text
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

it('the Usage period is app-wide: the rail card follows it, and it survives a view switch (HUN-196, FR-064)', async () => {
  render(<App />)
  await screen.findByRole('heading', { name: 'Totals' })
  const periods = () => within(screen.getByRole('group', { name: 'Period' }))
  expect(periods().getByRole('button', { name: 'Last 7 days' })).toHaveAttribute('aria-pressed', 'true')
  expect(await screen.findByRole('img', { name: /Cache reuse .*, last 7 days/ })).toBeInTheDocument()
  fireEvent.click(periods().getByRole('button', { name: 'Last 30 days' }))
  expect(await screen.findByRole('img', { name: /Cache reuse .*, last 30 days/ })).toBeInTheDocument()
  fireEvent.click(within(nav()).getByRole('button', { name: 'Board' }))
  expect(await screen.findByRole('img', { name: /Cache reuse .*, last 30 days/ })).toBeInTheDocument()
  fireEvent.click(within(nav()).getByRole('button', { name: 'Usage' }))
  await screen.findByRole('heading', { name: 'Totals' })
  expect(periods().getByRole('button', { name: 'Last 30 days' })).toHaveAttribute('aria-pressed', 'true')
  expect(periods().getByRole('button', { name: 'Last 7 days' })).toHaveAttribute('aria-pressed', 'false')
})

it('lists the 7 views in the sidebar and marks only the active one', () => {
  render(<App />)
  const buttons = within(nav()).getAllByRole('button').filter((b) => !b.hasAttribute('aria-expanded'))
  expect(buttons.map((b) => b.textContent)).toEqual(['Live', 'App sessions', 'Watch', 'Board', 'Scheduled', 'Memory', 'Usage', 'Warnings & plan', 'Kit', 'Workspaces', 'Agents', 'First run', 'Settings'])
  fireEvent.click(within(nav()).getByRole('button', { name: 'Kit' }))
  expect(within(nav()).getByRole('button', { name: 'Kit' })).toHaveAttribute('aria-current', 'page')
  expect(within(nav()).getByRole('button', { name: 'Usage' })).not.toHaveAttribute('aria-current')
})

describe('rail groups and First run (HUN-160)', () => {
  const groupLabels = () => within(nav()).getAllByRole('button').filter((b) => b.hasAttribute('aria-expanded')).map((b) => b.textContent)
  const allDone = { steps: firstRun.steps.map((s) => ({ ...s, done: true })) }

  it('shows the groups work, usage, kit, connect, system in that order', () => {
    render(<App />)
    expect(groupLabels()).toEqual(['// work', '// usage', '// kit', '// connect', '// system'])
    const groups = [...nav().querySelectorAll('.nav-group')]
    expect(groups.map((g) => g.dataset.group)).toEqual(['work', 'usage', 'kit', 'connect', 'system'])
    expect(groups.every((g) => !g.hasAttribute('hidden'))).toBe(true)
  })

  it('opens the Board from the work group and asks /api/board with the global scope (HUN-162)', async () => {
    const base = fakeFetch(workspaceOverrides())
    const f = vi.fn(async (url, init) =>
      String(url).startsWith('/api/board') ? { ok: true, status: 200, json: async () => ({ projects: [] }) } : base(url, init),
    )
    vi.stubGlobal('fetch', f)
    render(<App />)
    fireEvent.click(within(nav()).getByRole('button', { name: 'Board' }))
    expect(await screen.findByRole('heading', { name: 'Board' })).toBeInTheDocument()
    expect(within(nav()).getByRole('button', { name: 'Board' })).toHaveAttribute('aria-current', 'page')
    await waitFor(() => expect(f.mock.calls.some(([u]) => u === '/api/board')).toBe(true))
  })

  it('opens the Live view from the work group and starts its stream (HUN-178)', async () => {
    const opened = []
    vi.stubGlobal('EventSource', class { constructor(url) { opened.push(url) } addEventListener() {} close() {} })
    render(<App />)
    fireEvent.click(within(nav()).getByRole('button', { name: 'Live' }))
    expect(await screen.findByRole('heading', { name: 'Live' })).toBeInTheDocument()
    expect(within(nav()).getByRole('button', { name: 'Live' })).toHaveAttribute('aria-current', 'page')
    expect(opened).toEqual(['/api/live?token=secret-token'])
  })

  it('opens the Scheduled view from the work group (HUN-163)', async () => {
    render(<App />)
    fireEvent.click(within(nav()).getByRole('button', { name: 'Scheduled' }))
    expect(await screen.findByRole('heading', { name: 'Scheduled jobs' })).toBeInTheDocument()
    expect(within(nav()).getByRole('button', { name: 'Scheduled' })).toHaveAttribute('aria-current', 'page')
  })

  it('puts each view in its group', () => {
    render(<App />)
    const inGroup = (id) => [...nav().querySelectorAll(`[data-group="${id}"] [aria-expanded] ~ .group-items button`)].map((b) => b.textContent)
    expect(inGroup('work')).toEqual(['Live', 'App sessions', 'Watch', 'Board', 'Scheduled', 'Memory'])
    expect(inGroup('usage')).toEqual(['Usage', 'Warnings & plan'])
    expect(inGroup('kit')).toEqual(['Kit'])
    expect(inGroup('connect')).toEqual(['Workspaces', 'Agents'])
    expect(inGroup('system')).toEqual(['First run', 'Settings'])
  })

  it('opens the Memory view from the work group', async () => {
    render(<App />)
    fireEvent.click(within(nav()).getByRole('button', { name: 'Memory' }))
    expect(within(nav()).getByRole('button', { name: 'Memory' })).toHaveAttribute('aria-current', 'page')
    expect(await screen.findByRole('heading', { name: 'Memory' })).toBeInTheDocument()
  })

  it('shows the "N left" badge on First run', async () => {
    vi.stubGlobal('fetch', fakeFetch({ firstRun: { steps: firstRun.steps.map((s, i) => ({ ...s, done: i === 0 })) } }))
    render(<App />)
    expect(await within(nav()).findByText('3')).toBeInTheDocument()
  })

  it('shows 4 left on a fresh home', async () => {
    render(<App />)
    expect(await within(nav()).findByText('4')).toBeInTheDocument()
  })

  it('hides the badge when all four are done', async () => {
    const f = fakeFetch({ firstRun: allDone })
    vi.stubGlobal('fetch', f)
    render(<App />)
    await waitFor(() => expect(f.calls.some((c) => c.url === '/api/first-run')).toBe(true))
    await screen.findByRole('heading', { name: 'Totals' })
    expect(within(nav()).queryByTitle(/left$/)).not.toBeInTheDocument()
    expect(within(nav()).getByRole('button', { name: 'First run' })).toBeInTheDocument()
  })

  it('hides the badge when the checklist cannot be read', async () => {
    const f = fakeFetch()
    vi.stubGlobal('fetch', async (url, init) => (String(url) === '/api/first-run' ? { ok: false, status: 500, json: async () => ({}) } : f(url, init)))
    render(<App />)
    await screen.findByRole('heading', { name: 'Totals' })
    expect(within(nav()).queryByTitle(/left$/)).not.toBeInTheDocument()
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
    await within(nav()).findByText('4')
    const before = f.calls.filter((c) => c.url === '/api/first-run').length
    fireEvent.click(within(nav()).getByRole('button', { name: /^First run/ }))
    await waitFor(() => expect(f.calls.filter((c) => c.url === '/api/first-run').length).toBeGreaterThan(before))
  })

  it('shows a mono badge with the number of steps left, 2 with two open', async () => {
    vi.stubGlobal('fetch', fakeFetch({ firstRun: { steps: firstRun.steps.map((s, i) => ({ ...s, done: i < 2 })) } }))
    render(<App />)
    expect(await within(nav()).findByText('2')).toHaveClass('badge')
  })

  it('Check again reads /api/first-run again and draws the new state', async () => {
    const f = fakeFetch()
    let reads = 0
    vi.stubGlobal('fetch', async (url, init) => {
      if (String(url) !== '/api/first-run') return f(url, init)
      reads += 1
      f.calls.push({ url: String(url), init: init ?? {} })
      const upto = reads > 2 ? 2 : 1 // the 3rd read (Check again) finds the timer installed
      return { ok: true, status: 200, json: async () => ({ steps: firstRun.steps.map((s, i) => ({ ...s, done: i < upto })) }) }
    })
    render(<App />)
    fireEvent.click(within(nav()).getByRole('button', { name: /^First run/ }))
    await screen.findByText(count('1 of 4 steps done'))
    const before = f.calls.filter((c) => c.url === '/api/first-run').length
    fireEvent.click(await screen.findByRole('button', { name: 'Check again' }))
    expect(await screen.findByText(count('2 of 4 steps done'))).toBeInTheDocument()
    expect(f.calls.filter((c) => c.url === '/api/first-run').length).toBe(before + 1)
    expect(within(nav()).getByText('2')).toHaveClass('badge')
    expect(screen.getByRole('listitem', { name: 'Ingest timer installed' })).toHaveAttribute('data-state', 'done')
  })

  it('Check again keeps the old steps and shows an error line when the read fails', async () => {
    const f = fakeFetch()
    let reads = 0
    vi.stubGlobal('fetch', async (url, init) => {
      if (String(url) !== '/api/first-run') return f(url, init)
      reads += 1
      if (reads > 2) return { ok: false, status: 500, json: async () => ({}) }
      return { ok: true, status: 200, json: async () => firstRun }
    })
    render(<App />)
    fireEvent.click(within(nav()).getByRole('button', { name: /^First run/ }))
    fireEvent.click((await screen.findAllByRole('button', { name: 'Check again' }))[0])
    expect(await screen.findByRole('alert')).toHaveTextContent('Could not check again')
    expect(screen.getByText(count('0 of 4 steps done'))).toBeInTheDocument()
  })

  it('with all four done: the banner shows, Go to Usage opens Usage, and the badge is gone', async () => {
    vi.stubGlobal('fetch', fakeFetch({ firstRun: allDone }))
    render(<App />)
    fireEvent.click(within(nav()).getByRole('button', { name: /^First run/ }))
    expect(await screen.findByText('Setup is complete. Your usage appears after the next ingest.')).toBeInTheDocument()
    expect(within(nav()).queryByTitle(/left$/)).not.toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: 'Go to Usage' }))
    expect(within(nav()).getByRole('button', { name: 'Usage' })).toHaveAttribute('aria-current', 'page')
  })

  it('only ever reads /api/first-run with GET', async () => {
    const f = fakeFetch()
    vi.stubGlobal('fetch', f)
    render(<App />)
    await within(nav()).findByText('4')
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
    const kitCalls = f.calls.filter((c) => /^\/api\/kit(\?|$)/.test(c.url)) // the page read, not the by-workspace block (HUN-239)
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
      ['Easy Kit', 'https://easy-kits.vercel.app/'],
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

describe('the session the page is asked for (T095, FR-108)', () => {
  const ID = 'a1b2c3d4-0000-4000-8000-000000000001'
  const OTHER = 'a1b2c3d4-0000-4000-8000-000000000002'
  let sources
  class FakeStream {
    constructor() { this.listeners = {}; sources.push(this) }
    addEventListener(type, fn) { (this.listeners[type] ??= []).push(fn) }
    close() {}
    emit(type, data) { act(() => (this.listeners[type] ?? []).forEach((fn) => fn({ type, data: JSON.stringify(data) }))) }
  }
  const one = (id, project) => ({ session_id: id, account: 'Claude work', project, last_ts: new Date().toISOString(), tool_calls: 0, tokens_in: 0, tokens_out: 0, cost_usd: 0 })
  const snapshot = (...sessions) => sources.at(-1).emit('snapshot', { active_window_s: 300, last_id: 0, sessions })
  const startOn = (url) => window.history.replaceState(null, '', url)
  const row = (id) => document.getElementById(`live-row-${id}`)
  const selected = () => document.querySelectorAll('.live-row[aria-current="true"]')
  const viewIs = (name) => expect(within(nav()).getByRole('button', { name })).toHaveAttribute('aria-current', 'page')

  beforeEach(() => {
    sources = []
    vi.stubGlobal('EventSource', FakeStream)
    window.scrollTo = vi.fn()
  })
  afterEach(() => {
    vi.useRealTimers()
    window.history.replaceState(null, '', '/')
  })

  it('AC5: /?live=<listed id>#tok opens Live, selects that row with its panel open, and strips the query but not the fragment', async () => {
    startOn(`/?live=${ID}#tok`)
    render(<App />)
    viewIs('Live')
    expect(window.location.search).toBe('')
    expect(window.location.hash).toBe('#tok')
    expect(readToken()).toBe('tok')
    snapshot(one(OTHER, 'beta'), one(ID, 'alpha'))
    expect(row(ID)).toHaveAttribute('aria-current', 'true')
    expect(row(ID)).toHaveAttribute('aria-expanded', 'true')
    expect(row(OTHER)).not.toHaveAttribute('aria-current')
    expect(selected()).toHaveLength(1)
    expect(within(document.getElementById(`live-feed-${ID}`)).getByRole('button', { name: 'Close' })).toBeInTheDocument()
  })

  it('AC5: the row is scrolled into view, once', () => {
    const scroll = vi.fn()
    Element.prototype.scrollIntoView = scroll
    try {
      startOn(`/?live=${ID}#tok`)
      render(<App />)
      snapshot(one(ID, 'alpha'))
      expect(scroll).toHaveBeenCalledTimes(1)
      expect(scroll.mock.contexts.at(-1)).toBe(row(ID).closest('li'))
      snapshot(one(ID, 'alpha'), one(OTHER, 'beta'))
      expect(scroll).toHaveBeenCalledTimes(1)
    } finally {
      delete Element.prototype.scrollIntoView
    }
  })

  it('Stage 4 round 2 (1): the 10 s limit runs from the first snapshot: one that arrives at 11 s with the id selects it', () => {
    vi.useFakeTimers()
    startOn(`/?live=${ID}#tok`)
    render(<App />)
    act(() => vi.advanceTimersByTime(11_000))
    snapshot(one(OTHER, 'beta'), one(ID, 'alpha'))
    expect(row(ID)).toHaveAttribute('aria-current', 'true')
  })

  it('Stage 4 round 2 (1): the id first listed 9 s after a first snapshot (that came at 11 s) is selected; 10.5 s after it is not', () => {
    vi.useFakeTimers()
    startOn(`/?live=${ID}#tok`)
    render(<App />)
    act(() => vi.advanceTimersByTime(11_000))
    snapshot(one(OTHER, 'beta'))
    act(() => vi.advanceTimersByTime(9_000))
    snapshot(one(OTHER, 'beta'), one(ID, 'alpha'))
    expect(row(ID)).toHaveAttribute('aria-current', 'true')
  })

  it('Stage 4 round 2 (1): 10.5 s after a first snapshot (that came at 11 s) without the id, a later one with it selects nothing', () => {
    vi.useFakeTimers()
    startOn(`/?live=${ID}#tok`)
    render(<App />)
    act(() => vi.advanceTimersByTime(11_000))
    snapshot(one(OTHER, 'beta'))
    act(() => vi.advanceTimersByTime(10_500))
    snapshot(one(OTHER, 'beta'), one(ID, 'alpha'))
    expect(selected()).toHaveLength(0)
  })

  it('Stage 4 round 2 (2): the user opening a row ends the wish: the wanted session later listed does not take the selection', () => {
    vi.useFakeTimers()
    startOn(`/?live=${ID}#tok`)
    render(<App />)
    snapshot(one(OTHER, 'beta'))
    fireEvent.click(row(OTHER))
    act(() => vi.advanceTimersByTime(5_000))
    snapshot(one(OTHER, 'beta'), one(ID, 'alpha'))
    expect(row(OTHER)).toHaveAttribute('aria-expanded', 'true')
    expect(row(ID)).toHaveAttribute('aria-expanded', 'false')
  })

  it('Stage 4 round 2 (3): leaving Live drops a pending wish: going to Usage and back, the wanted session listed later selects nothing', () => {
    startOn(`/?live=${ID}#tok`)
    render(<App />)
    snapshot(one(OTHER, 'beta'))
    fireEvent.click(within(nav()).getByRole('button', { name: 'Usage' }))
    fireEvent.click(within(nav()).getByRole('button', { name: 'Live' }))
    snapshot(one(OTHER, 'beta'), one(ID, 'alpha'))
    expect(selected()).toHaveLength(0)
    expect(row(ID)).toHaveAttribute('aria-expanded', 'false')
  })

  it('AC5: a session that first appears in a later snapshot within 10 s is selected', () => {
    vi.useFakeTimers()
    startOn(`/?live=${ID}#tok`)
    render(<App />)
    snapshot(one(OTHER, 'beta'))
    expect(selected()).toHaveLength(0)
    act(() => vi.advanceTimersByTime(9_000))
    snapshot(one(OTHER, 'beta'), one(ID, 'alpha'))
    expect(row(ID)).toHaveAttribute('aria-current', 'true')

  })

  it('AC5: after 10 s the wish is given up: a later snapshot selects nothing', () => {
    vi.useFakeTimers()
    startOn(`/?live=${ID}#tok`)
    render(<App />)
    snapshot(one(OTHER, 'beta'))
    act(() => vi.advanceTimersByTime(10_500))
    snapshot(one(OTHER, 'beta'), one(ID, 'alpha'))
    expect(selected()).toHaveLength(0)
    expect(row(ID)).toHaveAttribute('aria-expanded', 'false')
  })

  it('AC5: the selection is made once: after the user closes the row a new snapshot does not open it again', () => {
    startOn(`/?live=${ID}#tok`)
    render(<App />)
    snapshot(one(ID, 'alpha'))
    fireEvent.click(row(ID))
    expect(row(ID)).toHaveAttribute('aria-expanded', 'false')
    snapshot(one(ID, 'alpha'), one(OTHER, 'beta'))
    expect(row(ID)).toHaveAttribute('aria-expanded', 'false')
  })

  it('AC5: a selected session that leaves the list clears the selection and shows no error', () => {
    startOn(`/?live=${ID}#tok`)
    render(<App />)
    snapshot(one(ID, 'alpha'), one(OTHER, 'beta'))
    snapshot(one(OTHER, 'beta'))
    expect(selected()).toHaveLength(0)
    expect(screen.queryByRole('alert')).toBeNull()
  })

  it.each([
    ['no id', '/?live#tok'],
    ['an empty id', '/?live=#tok'],
    ['an unknown id', '/?live=ffffffff-0000#tok'],
    ['a script', '/?live=%3Cscript%3E#tok'],
    ['a 65-character id', `/?live=${'a'.repeat(65)}#tok`],
    ['a non-hex letter', '/?live=xyz#tok'],
  ])('AC5: ?live with %s opens Live, selects nothing, shows no error, still strips the query', (_, url) => {
    startOn(url)
    render(<App />)
    viewIs('Live')
    snapshot(one(ID, 'alpha'), one('a'.repeat(65), 'long'))
    expect(selected()).toHaveLength(0)
    expect(document.querySelectorAll('.live-row[aria-expanded="true"]')).toHaveLength(0)
    expect(screen.queryByRole('alert')).toBeNull()
    expect(window.location.search).toBe('')
    expect(window.location.hash).toBe('#tok')
    expect(readToken()).toBe('tok')
  })

  it('AC5: a 64-character id is accepted', () => {
    const id = 'a'.repeat(64)
    startOn(`/?live=${id}#tok`)
    render(<App />)
    snapshot(one(id, 'alpha'))
    expect(row(id)).toHaveAttribute('aria-current', 'true')
  })

  it('AC5: with no parameter the page starts on Usage as before and leaves the address alone', () => {
    startOn('/?other=1#tok')
    render(<App />)
    viewIs('Usage')
    expect(window.location.search).toBe('?other=1')
    expect(readToken()).toBe('tok')
  })
})

describe('Settings and the first-open prompt (T092)', () => {
  const MARK = 'easy-personal-agent-run-asked'
  beforeEach(() => {
    HTMLDialogElement.prototype.showModal = function () {
      this.setAttribute('open', '')
    }
    HTMLDialogElement.prototype.close = function () {
      this.removeAttribute('open')
    }
  })

  it('opens Settings from the nav, under // system', async () => {
    vi.stubGlobal('fetch', fakeFetch({ agentRun: agentRunOn() }))
    window.localStorage.setItem(MARK, '1')
    render(<App />)
    fireEvent.click(within(nav()).getByRole('button', { name: 'Settings' }))
    expect(within(nav()).getByRole('button', { name: 'Settings' })).toHaveAttribute('aria-current', 'page')
    expect(await screen.findByRole('heading', { name: 'Agent run account' })).toBeInTheDocument()
    expect(screen.getByText('The app uses this account for every agent run.')).toBeInTheDocument()
  })

  it('offers the choice once on load when the setting is off and the browser holds no mark', async () => {
    vi.stubGlobal('fetch', fakeFetch({ agentRun: agentRunOff }))
    render(<App />)
    const dialog = await screen.findByRole('dialog', { name: 'Choose an account for agent runs' })
    fireEvent.click(within(dialog).getByRole('button', { name: 'Not now' }))
    expect(screen.queryByRole('dialog')).toBeNull()
    expect(window.localStorage.getItem(MARK)).toBe('1')
  })

  it('does not offer it while the Settings tab is open, so that view never goes stale under it', async () => {
    vi.stubGlobal('fetch', fakeFetch({ agentRun: agentRunOff }))
    render(<App />)
    await screen.findByRole('dialog', { name: 'Choose an account for agent runs' })
    fireEvent.click(within(nav()).getByRole('button', { name: 'Settings' }))
    expect(await screen.findByRole('heading', { name: 'Agent run account' })).toBeInTheDocument()
    expect(screen.queryByRole('dialog')).toBeNull()
    expect(window.localStorage.getItem(MARK)).toBeNull() // not answered: it may come back on another tab
  })

  it('does not offer it with the mark set, with the setting on, or on a server without the route', async () => {
    for (const [mark, given] of [[true, agentRunOff], [false, agentRunOn()], [false, undefined]]) {
      window.localStorage.clear()
      if (mark) window.localStorage.setItem(MARK, '1')
      vi.stubGlobal('fetch', fakeFetch({ agentRun: given }))
      const { unmount } = render(<App />)
      await screen.findByRole('heading', { name: 'Totals' })
      await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull())
      unmount()
    }
  })

  it('the Warnings & plan page links to Settings when the setting is off', async () => {
    vi.stubGlobal('fetch', fakeFetch({ agentRun: agentRunOff }))
    window.localStorage.setItem(MARK, '1')
    render(<App />)
    fireEvent.click(within(nav()).getByRole('button', { name: 'Warnings & plan' }))
    await screen.findByRole('heading', { name: 'Warnings and plan' })
    fireEvent.click(await within(screen.getByRole('main')).findByRole('button', { name: 'Settings' }))
    expect(await screen.findByRole('heading', { name: 'Agent run account' })).toBeInTheDocument()
  })
})
