import { render, screen, within, waitFor, fireEvent } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import WarningsPlanView from './WarningsPlanView.jsx'
import { fakeFetch, warnings } from './test/fixtures.js'

async function renderView(overrides) {
  const f = fakeFetch(overrides)
  vi.stubGlobal('fetch', f)
  render(<WarningsPlanView />)
  await screen.findByTestId('headline')
  return f
}

beforeEach(() => {
  window.location.hash = '#secret-token'
})
afterEach(() => {
  vi.unstubAllGlobals()
})

const hit = (over) => ({ ...warnings.hits[0], ...over })
const abstainedFive = ['a', 'b', 'c', 'd', 'e'].map((k) => ({ rule_id: `rule_${k}`, title: `Title ${k}`, reason: k === 'a' ? 'no stored history' : 'less than 7 days of stored history' }))

describe('header line (AC4)', () => {
  it('counts what to fix and what to watch', async () => {
    await renderView({ warnings: { ...warnings, hits: [warnings.hits[0], warnings.hits[2]] } })
    expect(screen.getByTestId('headline')).toHaveTextContent('1 thing to fix, 1 to watch')
    expect(screen.getAllByRole('article')).toHaveLength(2)
  })

  it('counts warning and improve as things to fix', async () => {
    await renderView()
    expect(screen.getByTestId('headline')).toHaveTextContent('2 things to fix, 1 to watch')
  })

  it('says all clear when nothing fired', async () => {
    await renderView({ warnings: { as_of_utc: 'x', hits: [], abstained: abstainedFive } })
    expect(screen.getByTestId('headline')).toHaveTextContent('All clear — nothing to fix in the last 7 days')
    expect(screen.queryAllByRole('article')).toHaveLength(0)
  })
})

describe('action cards (AC5)', () => {
  it('shows chip, title, summary, why it matters and numbered steps in priority order', async () => {
    await renderView()
    const cards = screen.getAllByRole('article')
    expect(cards.map((c) => c.dataset.rule)).toEqual(['cache_hit_low', 'context_index_stale', 'top_project_share'])
    const first = cards[0]
    expect(within(first).getByText('Fix now')).toBeInTheDocument()
    expect(within(first).getByRole('heading', { name: 'Low cache reuse' })).toBeInTheDocument()
    expect(first).toHaveTextContent('31% of prompt tokens came from cache; target 50% or more, last 7 days')
    expect(first).toHaveTextContent('Why it matters')
    expect(first).toHaveTextContent('A prompt read from cache costs far less.')
    const steps = within(first).getAllByRole('listitem').map((li) => li.textContent)
    expect(steps).toEqual(['Keep the start of your prompts the same.', 'Avoid editing CLAUDE.md mid-session.'])
    expect(first.querySelector('ol.steps')).not.toBeNull()
    expect(within(cards[1]).getByText('Improve')).toBeInTheDocument()
    expect(within(cards[2]).getByText('Watch')).toBeInTheDocument()
  })

  it('lists the detail names as a plain list under the steps', async () => {
    await renderView()
    const card = screen.getAllByRole('article')[1]
    expect(within(card).getByText('alpha').closest('ul')).not.toBeNull()
  })

  it('shows no raw rule id anywhere on the page (AC5)', async () => {
    const data = { ...warnings, abstained: [...warnings.abstained, ...abstainedFive] }
    await renderView({ warnings: data })
    const ids = [...data.hits, ...data.abstained].map((i) => i.rule_id)
    for (const id of ids) expect(document.body.textContent).not.toContain(id)
    expect(screen.getAllByRole('article')[0]).toHaveAttribute('data-rule', 'cache_hit_low')
  })

  it('does not break when an old server sends no steps or summary', async () => {
    await renderView({ warnings: { as_of_utc: 'x', hits: [hit({ steps: undefined, summary: undefined, why: undefined })], abstained: [] } })
    expect(screen.getAllByRole('article')).toHaveLength(1)
  })
})

describe('not checked (AC6)', () => {
  it('collapses abstained rules into a closed details with plain reasons and the Agents hint', async () => {
    const onOpenAgents = vi.fn()
    vi.stubGlobal('fetch', fakeFetch({ warnings: { as_of_utc: 'x', hits: [], abstained: abstainedFive } }))
    render(<WarningsPlanView onOpenAgents={onOpenAgents} />)
    const summary = await screen.findByText('Not checked (5)')
    const details = summary.closest('details')
    expect(details).not.toBeNull()
    expect(details.open).toBe(false)
    expect(details).toHaveTextContent('Title a')
    expect(details).toHaveTextContent('No stored history')
    fireEvent.click(within(details).getByRole('button', { name: 'Confirm the accounts you use' }))
    expect(onOpenAgents).toHaveBeenCalledTimes(1)
  })

  it('omits the Agents hint when no reason is about missing data', async () => {
    const warn = { as_of_utc: 'x', hits: [], abstained: [{ rule_id: 'r_x', title: 'T', reason: 'less than 7 days of stored history' }] }
    vi.stubGlobal('fetch', fakeFetch({ warnings: warn }))
    render(<WarningsPlanView onOpenAgents={() => {}} />)
    await screen.findByText('Not checked (1)')
    expect(screen.queryByRole('button', { name: 'Confirm the accounts you use' })).toBeNull()
  })

  it('shows no Not checked block when everything was checked', async () => {
    await renderView({ warnings: { ...warnings, abstained: [] } })
    expect(screen.queryByText(/Not checked/)).toBeNull()
  })
})

describe('one list (AC7)', () => {
  it('has no separate Improvement plan section and no duplicate cards', async () => {
    await renderView()
    expect(screen.queryByRole('region', { name: 'Improvement plan' })).toBeNull()
    expect(screen.queryByText('Improvement plan')).toBeNull()
    expect(screen.getAllByText('Low cache reuse')).toHaveLength(1)
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
    expect(f.calls.map((c) => c.url)).toEqual(['/api/warnings'])
    expect(screen.getByTestId('scope-line')).toHaveTextContent('Showing: All · All')
  })

  it('sends workspace= and account= to warnings and plan, refetches on change, names the scope', async () => {
    const f = fakeFetch()
    vi.stubGlobal('fetch', f)
    const options = { workspaces: [], accounts: [{ id: 2, label: 'Claude personal' }] }
    const view = render(<WarningsPlanView scope={{ workspace: 'Acme Co', account: '' }} options={options} />)
    await screen.findByTestId('headline')
    expect(f.calls.map((c) => c.url).sort()).toEqual(['/api/warnings?workspace=Acme%20Co'])
    f.calls.length = 0
    view.rerender(<WarningsPlanView scope={{ workspace: 'Acme Co', account: '2' }} options={options} />)
    await waitFor(() => expect(f.calls.length).toBe(1))
    expect(f.calls.map((c) => c.url).sort()).toEqual(['/api/warnings?workspace=Acme%20Co&account=2'])
    expect(await screen.findByTestId('scope-line')).toHaveTextContent('Showing: Acme Co · Claude personal')
  })
})

describe('feed cards (HUN-121 AC8)', () => {
  it('each action is a feed card with its rank as icon, the window at the right and a step count in the footer', async () => {
    await renderView()
    const [first, second] = screen.getAllByRole('article')
    expect(first).toHaveClass('feed-card')
    expect(first.querySelector('.feed-icon')).toHaveTextContent('1')
    expect(second.querySelector('.feed-icon')).toHaveTextContent('2')
    expect(first.querySelector('.feed-time')).toHaveTextContent('last 7 days')
    expect(first.querySelector('footer.feed-foot')).toHaveTextContent('2 steps')
    expect(second.querySelector('footer.feed-foot')).toHaveTextContent('1 step')
  })
})
