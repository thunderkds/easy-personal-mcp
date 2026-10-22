import { render, screen, within, fireEvent } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import KitView from './KitView.jsx'
import { fakeFetch, kit } from './test/fixtures.js'

async function renderView(overrides) {
  vi.stubGlobal('fetch', fakeFetch(overrides))
  render(<KitView />)
  await screen.findByRole('heading', { name: 'Kit by project' })
}
const row = (name) => screen.getByRole('rowheader', { name: new RegExp(`^${name}`) }).closest('tr')

beforeEach(() => {
  window.location.hash = '#secret-token'
})
afterEach(() => {
  vi.unstubAllGlobals()
})

describe('kit panel (AC3)', () => {
  it('shows "no data" for no-data and absent parts, never zero', async () => {
    await renderView()
    const beta = row('beta')
    const cells = within(beta).getAllByRole('cell')
    // kernel, verifier, context, git
    expect(cells.map((c) => c.textContent)).toEqual(['no data', 'no data', 'no data', 'abstained'])
    expect(beta).not.toHaveTextContent(/\b0\b/)
  })

  it('always shows a verifier column, "no data" when the key is absent', async () => {
    await renderView()
    expect(screen.getByRole('columnheader', { name: 'Verifier' })).toBeInTheDocument()
    expect(within(row('alpha')).getAllByRole('cell')[1]).toHaveTextContent('no data')
  })

  it('renders a verifier part generically when the key appears', async () => {
    const withVerifier = structuredClone(kit)
    withVerifier.projects[1].parts.verifier = { status: 'ok', score: 87, dimensions: ['a'] }
    await renderView({ kit: withVerifier })
    expect(within(row('alpha')).getAllByRole('cell')[1]).toHaveTextContent('score: 87')
  })

  it('shows kernel numbers and keeps a real zero when the data is there', async () => {
    await renderView()
    const alpha = within(row('alpha')).getAllByRole('cell')
    expect(alpha[0]).toHaveTextContent('40 tool calls')
    expect(alpha[0]).toHaveTextContent('0 errors')
    expect(alpha[3]).toHaveTextContent('7 commits')
  })

  it('highlights a stale index and shows unknown (not fresh) when stale is null', async () => {
    await renderView()
    const stale = within(row('alpha')).getByText('stale')
    expect(stale).toHaveClass('stale')
    expect(within(row('gamma')).getByText('unknown')).toBeInTheDocument()
    expect(within(row('gamma')).queryByText('fresh')).toBeNull()
    expect(within(row('gamma')).queryByText('stale')).toBeNull()
  })

  it('marks a project missing on disk', async () => {
    await renderView()
    expect(row('beta')).toHaveTextContent('missing on disk')
  })

  it('shows easy-ui overall, or no data', async () => {
    await renderView()
    const ui = screen.getByRole('region', { name: 'easy-ui checks (overall)' })
    expect(ui).toHaveTextContent('sessions: 6')
    expect(ui).toHaveTextContent('failed: 1')
  })

  it('shows easy-ui "no data" explicitly', async () => {
    await renderView({ kit: { projects: [], ui: { status: 'no data' } } })
    expect(screen.getByRole('region', { name: 'easy-ui checks (overall)' })).toHaveTextContent('no data')
    expect(screen.getByText(/No projects/)).toBeInTheDocument()
  })
})

describe('sorting', () => {
  it('sorts the project column on click, both ways', async () => {
    await renderView()
    const names = () => screen.getAllByRole('rowheader').map((h) => h.textContent.split(' ')[0])
    expect(names()).toEqual(['alpha', 'beta', 'gamma'])
    fireEvent.click(screen.getByRole('button', { name: /Project/ }))
    expect(names()).toEqual(['gamma', 'beta', 'alpha'])
    expect(screen.getByRole('columnheader', { name: /Project/ })).toHaveAttribute('aria-sort', 'descending')
  })
})

describe('errors', () => {
  it('reports a failed request', async () => {
    vi.stubGlobal('fetch', fakeFetch({ status: 403 }))
    render(<KitView />)
    expect(await screen.findByRole('alert')).toHaveTextContent('403')
  })
})
