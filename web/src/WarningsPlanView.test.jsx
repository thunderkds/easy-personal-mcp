import { render, screen, within, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import WarningsPlanView from './WarningsPlanView.jsx'
import { fakeFetch } from './test/fixtures.js'

async function renderView(overrides) {
  const f = fakeFetch(overrides)
  vi.stubGlobal('fetch', f)
  render(<WarningsPlanView />)
  await screen.findByRole('heading', { name: 'Warnings' })
  return f
}

beforeEach(() => {
  window.location.hash = '#secret-token'
})
afterEach(() => {
  vi.unstubAllGlobals()
})

describe('warnings (AC1)', () => {
  it('shows label, rule id, measured value, threshold and fix line for each warning', async () => {
    await renderView()
    const sec = screen.getByRole('region', { name: 'Warnings' })
    const item = within(sec).getByText('cache_hit_low').closest('li')
    expect(within(item).getByText('warning')).toBeInTheDocument()
    expect(item).toHaveTextContent('Measured 0.31')
    expect(item).toHaveTextContent('threshold 0.5')
    expect(item).toHaveTextContent('Keep prompts stable so the cache is reused.')
    expect(within(sec).getAllByRole('listitem').length).toBeGreaterThanOrEqual(3)
  })

  it('names rules that could not be measured instead of hiding them', async () => {
    await renderView()
    expect(screen.getByText('kit_error_rate').closest('li')).toHaveTextContent('no kernel-trace tool calls in the window')
  })

  it('says so when nothing fired', async () => {
    await renderView({ warnings: { as_of_utc: 'x', hits: [], abstained: [] }, plan: { groups: [] } })
    expect(screen.getAllByText(/No warnings/).length).toBeGreaterThan(0)
    expect(screen.getByText(/No actions in the plan/)).toBeInTheDocument()
  })
})

describe('plan (AC2)', () => {
  it('groups items by priority in the order given', async () => {
    await renderView()
    const plan = screen.getByRole('region', { name: 'Improvement plan' })
    const heads = within(plan).getAllByRole('heading', { level: 3 }).map((h) => h.textContent)
    expect(heads).toEqual(['Priority 1', 'Priority 3'])
    const first = within(plan).getByRole('region', { name: 'Priority 1' })
    expect(first).toHaveTextContent('cache_hit_low')
    expect(first).not.toHaveTextContent('top_project_share')
  })
})

describe('errors and token', () => {
  it('sends the token and reports a failed request', async () => {
    const f = fakeFetch({ status: 403 })
    vi.stubGlobal('fetch', f)
    render(<WarningsPlanView />)
    expect(await screen.findByRole('alert')).toHaveTextContent('403')
    expect(f.calls[0].init.headers['X-Dashboard-Token']).toBe('secret-token')
  })
})

describe('scope (HUN-118)', () => {
  it('sends no params without a scope and says All', async () => {
    const f = await renderView()
    expect(f.calls.map((c) => c.url).sort()).toEqual(['/api/plan', '/api/warnings'])
    expect(screen.getByTestId('scope-line')).toHaveTextContent('Showing: All · All')
  })

  it('sends workspace= and account= to warnings and plan, refetches on change, names the scope', async () => {
    const f = fakeFetch()
    vi.stubGlobal('fetch', f)
    const options = { workspaces: [], accounts: [{ id: 2, label: 'Claude personal' }] }
    const view = render(<WarningsPlanView scope={{ workspace: 'Acme Co', account: '' }} options={options} />)
    await screen.findByRole('heading', { name: 'Warnings' })
    expect(f.calls.map((c) => c.url).sort()).toEqual(['/api/plan?workspace=Acme%20Co', '/api/warnings?workspace=Acme%20Co'])
    f.calls.length = 0
    view.rerender(<WarningsPlanView scope={{ workspace: 'Acme Co', account: '2' }} options={options} />)
    await waitFor(() => expect(f.calls.length).toBe(2))
    expect(f.calls.map((c) => c.url).sort()).toEqual(['/api/plan?workspace=Acme%20Co&account=2', '/api/warnings?workspace=Acme%20Co&account=2'])
    expect(await screen.findByTestId('scope-line')).toHaveTextContent('Showing: Acme Co · Claude personal')
  })
})
