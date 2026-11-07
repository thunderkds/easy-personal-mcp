import { render as rtlRender, screen, fireEvent, waitFor, within } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import WorkspacesView from './WorkspacesView.jsx'
import { ToastProvider } from './Toast.jsx'
import { fakeFetch, workspaceOverrides, workspacesFull } from './test/fixtures.js'

// HUN-207 (FR-070, FR-065): the "Repos not in a workspace" card of the Workspaces page.
const render = (ui) => rtlRender(ui, { wrapper: ToastProvider })
const SECRET = 'xoxb-PLANTED-secret-123'
const cost = (usd) => ({ usd, abstained_models: [], price_table_date: '2026-10-01' })
const repo = (name, extra = {}) => ({
  path: `/home/o/work/${name}`,
  name,
  sessions: 4,
  api_equivalent_cost: cost(12.345),
  last_used_utc: '2026-11-01T12:00:00Z',
  ...extra,
})
const repos = [repo('idea-ladder'), repo('kitchd', { sessions: 1, api_equivalent_cost: cost(null) })]
const longPath = `/home/o/${'very-long-folder-name/'.repeat(8)}deep`

/**
 * Reads come from the fixtures; `unassigned` is its own answer (a body, an Error for HTTP 500, or a promise for a slow read).
 * PATCH answers `patch(body, workspaceId)` and is recorded; a changed workspace shows up in later GET /api/workspaces.
 */
function stub({ unassigned = { repos }, workspaces = workspacesFull, patch, hold } = {}) {
  let current = workspaces
  const read = fakeFetch(workspaceOverrides({ workspaces: current }))
  const fn = vi.fn(async (url, init = {}) => {
    const path = new URL(String(url), 'http://127.0.0.1').pathname
    if (init.method === 'PATCH') {
      if (hold) await hold
      const id = Number(path.split('/').pop())
      const body = JSON.parse(init.body)
      const out = patch ? patch(body, id) : {}
      if (out instanceof Error) return { ok: false, status: 400, json: async () => ({ error: out.message }) }
      current = { workspaces: current.workspaces.map((w) => (w.id === id ? { ...w, repos: body.repos } : w)) }
      return { ok: true, status: 200, json: async () => ({ status: 0, lines: [], pending: false, workspace: current.workspaces.find((w) => w.id === id) }) }
    }
    if (path === '/api/workspaces/unassigned') {
      const out = await unassigned
      if (out instanceof Error) return { ok: false, status: 500, json: async () => ({ error: 'boom' }) }
      return { ok: true, status: 200, json: async () => out }
    }
    if (path === '/api/workspaces') return { ok: true, status: 200, json: async () => current }
    return read(url, init)
  })
  vi.stubGlobal('fetch', fn)
  return fn
}
const patches = (fn) => fn.mock.calls.filter(([, init]) => init?.method === 'PATCH').map(([url, init]) => [url, JSON.parse(init.body)])
const card = () => screen.findByRole('region', { name: 'Repos not in a workspace' })
const rowOf = (name) => screen.getByRole('listitem', { name })

beforeEach(() => {
  window.location.hash = '#secret-token'
})
afterEach(() => vi.unstubAllGlobals())

describe('the card (AC5)', () => {
  it('lists each repo with name, full path, sessions, cost and local last-used date; the heading counts them', async () => {
    stub()
    render(<WorkspacesView />)
    const c = await card()
    expect(within(c).getByRole('heading', { name: 'Repos not in a workspace (2)' })).toBeInTheDocument()
    const first = within(c).getByRole('listitem', { name: 'idea-ladder' })
    expect(within(first).getByText('/home/o/work/idea-ladder')).toHaveAttribute('title', '/home/o/work/idea-ladder')
    const day = new Date('2026-11-01T12:00:00Z')
    const local = `${day.getFullYear()}-${String(day.getMonth() + 1).padStart(2, '0')}-${String(day.getDate()).padStart(2, '0')}`
    expect(first).toHaveTextContent(`4 sessions · $12.35 · last used ${local}`)
    expect(within(c).getByRole('listitem', { name: 'kitchd' })).toHaveTextContent(/1 session · no total · last used/)
  })

  it('the select lists non-orphan workspaces only', async () => {
    stub()
    render(<WorkspacesView />)
    await card()
    const select = within(rowOf('idea-ladder')).getByLabelText('Add to workspace')
    expect(within(select).getAllByRole('option').map((o) => o.textContent)).toEqual(['Choose a workspace', 'company', 'side'])
  })

  it('a very long path keeps its full text in the title (CSS ellipsises it)', async () => {
    stub({ unassigned: { repos: [repo('deep', { path: longPath })] } })
    render(<WorkspacesView />)
    await card()
    expect(within(rowOf('deep')).getByText(longPath)).toHaveAttribute('title', longPath)
  })

  it('shows only what the server sent: a planted secret is not on the page', async () => {
    stub()
    render(<WorkspacesView />)
    await card()
    expect(document.body.textContent).not.toContain(SECRET)
  })
})

describe('loading and failure never block the page (AC4)', () => {
  it('renders the workspace list while the unassigned read is pending, then fills the card', async () => {
    let release
    stub({ unassigned: new Promise((r) => (release = r)) })
    render(<WorkspacesView />)
    expect(await screen.findByRole('region', { name: 'company' })).toBeInTheDocument()
    expect(screen.getByRole('status')).toHaveTextContent(/Loading repos not in a workspace/)
    release({ repos })
    expect(await screen.findByRole('listitem', { name: 'idea-ladder' })).toBeInTheDocument()
    expect(screen.queryByText(/Loading repos not in a workspace/)).toBeNull()
  })

  it('a failed unassigned read is an inline error in the card only', async () => {
    stub({ unassigned: new Error('x') })
    render(<WorkspacesView />)
    expect(await screen.findByRole('region', { name: 'company' })).toBeInTheDocument()
    const c = await card()
    expect(await within(c).findByRole('alert')).toHaveTextContent(/Could not load repos not in a workspace/)
    expect(screen.getAllByRole('alert')).toHaveLength(1)
    expect(screen.getByRole('button', { name: '+ Add workspace' })).toBeEnabled()
  })

  it('the unassigned read starts with the page reads, before they finish', async () => {
    const fn = stub()
    render(<WorkspacesView />)
    await card()
    const paths = fn.mock.calls.map(([url]) => String(url))
    expect(paths.indexOf('/api/workspaces/unassigned')).toBeLessThan(paths.indexOf('/api/connectors'))
  })
})

describe('Add (AC6)', () => {
  const choose = (name, workspace) => fireEvent.change(within(rowOf(name)).getByLabelText('Add to workspace'), { target: { value: String(workspace) } })
  const add = (name) => fireEvent.click(within(rowOf(name)).getByRole('button', { name: 'Add' }))

  it('Add is off until a workspace is chosen', async () => {
    stub()
    render(<WorkspacesView />)
    await card()
    expect(within(rowOf('idea-ladder')).getByRole('button', { name: 'Add' })).toBeDisabled()
  })

  it('PATCHes the workspace repos plus the path (no other field), toasts, drops the row, bumps the repo count', async () => {
    const fn = stub()
    render(<WorkspacesView />)
    const company = await screen.findByRole('region', { name: 'company' })
    expect(within(company).getByText('2 repos')).toBeInTheDocument()
    await card()
    choose('idea-ladder', 2)
    add('idea-ladder')
    expect(await screen.findByText('Added idea-ladder to company')).toBeInTheDocument()
    expect(patches(fn)).toEqual([['/api/workspaces/2', { repos: ['/work/company', '/work/company-web', '/home/o/work/idea-ladder'] }]])
    await waitFor(() => expect(screen.queryByRole('listitem', { name: 'idea-ladder' })).toBeNull())
    expect(screen.getByRole('heading', { name: 'Repos not in a workspace (1)' })).toBeInTheDocument()
    expect(within(screen.getByRole('region', { name: 'company' })).getByText('3 repos')).toBeInTheDocument()
    expect(screen.getByRole('listitem', { name: 'kitchd' })).toBeInTheDocument()
  })

  it('reads the workspace fresh before the PATCH: a repo added in another tab is kept', async () => {
    const fn = stub()
    render(<WorkspacesView />)
    await card()
    const stale = fn.mock.calls.length
    // Another tab added a repo after the page loaded.
    const changed = { workspaces: workspacesFull.workspaces.map((w) => (w.id === 3 ? { ...w, repos: ['/other/tab'] } : w)) }
    fn.mockImplementationOnce(async () => ({ ok: true, status: 200, json: async () => changed }))
    choose('idea-ladder', 3)
    add('idea-ladder')
    await screen.findByText('Added idea-ladder to side')
    expect(patches(fn)).toEqual([['/api/workspaces/3', { repos: ['/other/tab', '/home/o/work/idea-ladder'] }]])
    expect(fn.mock.calls.length).toBeGreaterThan(stale)
  })

  it('does not add the path twice when the fresh workspace already has it', async () => {
    const fn = stub()
    render(<WorkspacesView />)
    await card()
    const changed = { workspaces: workspacesFull.workspaces.map((w) => (w.id === 3 ? { ...w, repos: ['/home/o/work/idea-ladder'] } : w)) }
    fn.mockImplementationOnce(async () => ({ ok: true, status: 200, json: async () => changed }))
    choose('idea-ladder', 3)
    add('idea-ladder')
    await screen.findByText('Added idea-ladder to side')
    expect(patches(fn)[0][1]).toEqual({ repos: ['/home/o/work/idea-ladder'] })
  })

  it('a double click sends one PATCH; the button is busy while it runs', async () => {
    let release
    const fn = stub({ hold: new Promise((r) => (release = r)) })
    render(<WorkspacesView />)
    await card()
    choose('idea-ladder', 2)
    add('idea-ladder')
    const busy = await within(rowOf('idea-ladder')).findByRole('button', { name: 'Adding…' })
    expect(busy).toBeDisabled()
    fireEvent.click(busy)
    expect(within(rowOf('kitchd')).getByRole('button', { name: 'Add' })).toBeDisabled()
    release()
    await screen.findByText('Added idea-ladder to company')
    expect(patches(fn)).toHaveLength(1)
  })

  it('an error shows an error toast and the row stays', async () => {
    stub({ patch: () => new Error('repo path must be absolute') })
    render(<WorkspacesView />)
    await card()
    choose('idea-ladder', 2)
    add('idea-ladder')
    const toast = await screen.findByText(/Could not add idea-ladder: repo path must be absolute/)
    expect(toast.closest('[role="alert"]')).not.toBeNull()
    expect(screen.getByRole('listitem', { name: 'idea-ladder' })).toBeInTheDocument()
    expect(within(rowOf('idea-ladder')).getByRole('button', { name: 'Add' })).toBeEnabled()
    expect(within(screen.getByRole('region', { name: 'company' })).getByText('2 repos')).toBeInTheDocument()
  })
})

describe('empty states (AC7)', () => {
  it('only orphan: a hint replaces the select and the button', async () => {
    stub({ workspaces: { workspaces: [workspacesFull.workspaces[0]] } })
    render(<WorkspacesView />)
    await card()
    const row = rowOf('idea-ladder')
    expect(within(row).getByText('Add a workspace first')).toBeInTheDocument()
    expect(within(row).queryByLabelText('Add to workspace')).toBeNull()
    expect(within(row).queryByRole('button', { name: 'Add' })).toBeNull()
  })

  it('no unassigned repo: "Every repo with usage is in a workspace."', async () => {
    stub({ unassigned: { repos: [] } })
    render(<WorkspacesView />)
    const c = await card()
    expect(await within(c).findByText('Every repo with usage is in a workspace.')).toBeInTheDocument()
    expect(within(c).getByRole('heading', { name: 'Repos not in a workspace (0)' })).toBeInTheDocument()
  })

  it('the last row leaving shows the empty text', async () => {
    stub({ unassigned: { repos: [repo('idea-ladder')] } })
    render(<WorkspacesView />)
    await card()
    fireEvent.change(within(rowOf('idea-ladder')).getByLabelText('Add to workspace'), { target: { value: '2' } })
    fireEvent.click(within(rowOf('idea-ladder')).getByRole('button', { name: 'Add' }))
    expect(await screen.findByText('Every repo with usage is in a workspace.')).toBeInTheDocument()
  })
})
