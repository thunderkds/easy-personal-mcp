import { render, screen, fireEvent, waitFor, within } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import App from './App.jsx'
import AgentsView from './AgentsView.jsx'
import { fakeFetch } from './test/fixtures.js'

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
    fireEvent.click(screen.getByRole('tab', { name: 'Agents' }))
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
    const r = await screen.findByRole('listitem', { name: '/home/o/.claude-new' })
    fireEvent.change(within(r).getByLabelText('Name'), { target: { value: 'Claude side' } })
    fireEvent.click(within(r).getByRole('button', { name: 'Confirm & name' }))
    await waitFor(() => expect(writes(fn)).toContainEqual(['POST', '/api/accounts/5/confirm', { label: 'Claude side' }]))
  })

  it('AC2: Ignore posts an empty body', async () => {
    const fn = stub()
    render(<AgentsView />)
    const r = await screen.findByRole('listitem', { name: '/home/o/.claude-new' })
    fireEvent.click(within(r).getByRole('button', { name: 'Ignore' }))
    await waitFor(() => expect(writes(fn)).toContainEqual(['POST', '/api/accounts/5/ignore', {}]))
  })

  it('AC2: Confirm & name needs a label', async () => {
    const fn = stub()
    render(<AgentsView />)
    const r = await screen.findByRole('listitem', { name: '/home/o/.claude-new' })
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
