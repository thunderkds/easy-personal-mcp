import { render, screen, within, fireEvent, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import KitView, { commitStats, projectHealth } from './KitView.jsx'
import { fakeFetch, kit } from './test/fixtures.js'

// "Now" is pinned to local noon on 2026-10-21; only Date is faked so timers keep working.
const NOW = new Date(2026, 9, 21, 12, 0, 0)

async function renderView(overrides) {
  vi.stubGlobal('fetch', fakeFetch(overrides))
  render(<KitView />)
  await screen.findByRole('heading', { name: /^Kit by project/ })
}
const card = (name) => screen.getByRole('article', { name })
const project = (name) => structuredClone(kit.projects.find((p) => p.project === name))
const withProjects = (...projects) => ({ kit: { ...kit, projects } })
async function openDetails() {
  fireEvent.click(screen.getByRole('button', { name: 'Show details' }))
}
const row = (name) => screen.getByRole('rowheader', { name: new RegExp(`^${name}`) }).closest('tr')

beforeEach(() => {
  window.location.hash = '#secret-token'
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(NOW)
})
afterEach(() => {
  vi.useRealTimers()
  vi.unstubAllGlobals()
})

describe('projectHealth (AC2)', () => {
  it('flags a high tool-call failure rate and names the reason', () => {
    const health = projectHealth({ parts: { kernel: { status: 'ok', error_rate: 0.2 } } })
    expect(health).toEqual({ level: 'attention', text: 'Needs attention: 20% of tool calls failed' })
  })
  it('does not flag exactly 10%', () => {
    expect(projectHealth({ parts: { kernel: { status: 'ok', error_rate: 0.1 } } }).level).toBe('healthy')
  })
  it('flags a stale code index, and names both reasons when both apply', () => {
    expect(projectHealth({ parts: { context: { status: 'ok', stale: true } } }).text).toBe('Needs attention: code index is out of date')
    const both = projectHealth({ parts: { kernel: { status: 'ok', error_rate: 0.5 }, context: { status: 'ok', stale: true } } })
    expect(both.text).toBe('Needs attention: 50% of tool calls failed; code index is out of date')
  })
  it('is healthy with data and no problem, and ignores a null error rate or unknown freshness', () => {
    const parts = { kernel: { status: 'ok', error_rate: null }, context: { status: 'ok', stale: null } }
    expect(projectHealth({ parts })).toEqual({ level: 'healthy', text: 'Healthy' })
  })
  it('says "No kit data" when nothing has data (no data, abstained or absent)', () => {
    expect(projectHealth({ parts: { kernel: { status: 'no data' }, git: { status: 'abstained' } } })).toEqual({ level: 'none', text: 'No kit data' })
    expect(projectHealth({ parts: {} }).level).toBe('none')
  })
})

describe('commitStats (AC3, AC4)', () => {
  it('counts today and the last 30 local days, ignoring older days', () => {
    const stats = commitStats({ '2026-10-21': 10, '2026-10-01': 5, '2025-10-21': 100 }, NOW)
    expect(stats.today).toBe(10)
    expect(stats.last30).toBe(15)
    expect(stats.series).toHaveLength(30)
    expect(stats.series[29]).toBe(10)
    expect(stats.series[9]).toBe(5)
    expect(stats.peak).toBe(10)
  })
  it('keeps the 30-day edge: 29 days ago counts, 30 days ago does not', () => {
    const stats = commitStats({ '2026-09-22': 4, '2026-09-21': 9 }, NOW)
    expect(stats.last30).toBe(4)
  })
})

describe('project cards (AC1, AC2, AC3, AC7)', () => {
  it('shows the page intro', async () => {
    await renderView()
    expect(screen.getByText('How the agent kit, code index and verifier are doing in each project.')).toBeInTheDocument()
  })

  it('lists one card per project, newest activity first, no-data last', async () => {
    await renderView()
    const names = screen.getAllByRole('article').map((a) => within(a).getByRole('heading').textContent)
    expect(names).toEqual(['delta', 'alpha', 'gamma', 'beta'])
  })

  it('names the reason in the chip: failed calls', async () => {
    await renderView()
    expect(within(card('gamma')).getByText('Needs attention: 40% of tool calls failed')).toBeInTheDocument()
  })

  it('names the reason in the chip: stale index (and highlights it)', async () => {
    await renderView()
    expect(within(card('alpha')).getByText('Needs attention: code index is out of date')).toBeInTheDocument()
    expect(within(card('alpha')).getByText('stale')).toHaveClass('stale')
  })

  it('shows Healthy and No kit data chips', async () => {
    await renderView()
    expect(within(card('delta')).getByText('Healthy')).toBeInTheDocument()
    expect(within(card('beta')).getByText('No kit data')).toBeInTheDocument()
  })

  it('writes the summary sentence from last activity, tool calls and commits', async () => {
    await renderView()
    expect(card('delta')).toHaveTextContent('Last active today. 1,200 agent tool calls, none failed. 3 commits today, 5 in the last 30 days.')
    expect(card('alpha')).toHaveTextContent('Last active yesterday. 40 agent tool calls, none failed. 0 commits today, 7 in the last 30 days.')
    expect(card('gamma')).toHaveTextContent('Last active 19 days ago. 5 agent tool calls, 2 failed.')
  })

  it('does not print failure text when the error rate is null', async () => {
    const p = project('alpha')
    p.parts.kernel.error_rate = null
    await renderView(withProjects(p))
    expect(card('alpha')).toHaveTextContent('40 agent tool calls.')
    expect(card('alpha')).not.toHaveTextContent(/none failed|failed/)
  })

  it('says there is no kit data yet for an empty project, never a zero', async () => {
    await renderView()
    expect(card('beta')).toHaveTextContent('No kit data yet.')
    expect(card('beta')).not.toHaveTextContent(/\b0\b/)
  })

  it('marks a project whose folder is missing', async () => {
    await renderView()
    expect(card('beta')).toHaveTextContent('(folder missing)')
    expect(card('delta')).not.toHaveTextContent('folder missing')
  })

  it('sorts on the newest of the kernel and the git day', async () => {
    const a = project('alpha')
    a.project = 'git-newer'
    a.parts.kernel.last_utc = '2026-10-01T12:00:00Z'
    a.parts.git.commits_by_day = { '2026-10-21': 1 }
    const b = project('alpha')
    b.project = 'kernel-newer'
    b.parts.git.commits_by_day = {}
    await renderView(withProjects(b, a))
    const names = screen.getAllByRole('article').map((el) => within(el).getByRole('heading').textContent)
    expect(names).toEqual(['git-newer', 'kernel-newer'])
  })
})

describe('commit sparkline (AC4)', () => {
  it('labels the chart with the total and the peak', async () => {
    await renderView()
    expect(within(card('delta')).getByRole('img', { name: 'Commits per day, last 30 days: 5 in total, peak 3 in one day' })).toBeInTheDocument()
  })
  it('draws none without git data', async () => {
    await renderView()
    expect(within(card('beta')).queryByRole('img')).toBeNull()
  })
})

describe('part lines (AC5)', () => {
  it('gives each part with data a plain name and meaning', async () => {
    await renderView()
    const delta = card('delta')
    expect(delta).toHaveTextContent('Agent kit (tool calls the kit traced) — 9 kit tasks')
    expect(delta).toHaveTextContent('Code index (easy-context) — indexed 2 days ago, fresh')
    expect(delta).toHaveTextContent('Verifier score — 82/100')
    expect(delta).toHaveTextContent('Git history (local commits) — 5 commits in the last 30 days')
  })

  it('shows unknown, not fresh, when the index freshness is null', async () => {
    await renderView()
    expect(card('gamma')).toHaveTextContent('Code index (easy-context) — indexed today, freshness unknown')
    expect(card('gamma')).not.toHaveTextContent('fresh,')
  })

  it('collapses no-data parts into one muted line', async () => {
    const p = project('delta')
    p.parts.context = { status: 'no data' }
    p.parts.verifier = { status: 'no data' }
    await renderView(withProjects(p))
    expect(card('delta')).toHaveTextContent('Not set up here: Code index, Verifier')
    expect(within(card('delta')).queryByText(/Code index \(easy-context\)/)).toBeNull()
  })

  it('treats an absent part like a no-data part', async () => {
    await renderView()
    expect(card('alpha')).toHaveTextContent('Not set up here: Verifier')
  })

  it('names a verifier score that could not be computed, and a part that could not be read', async () => {
    const p = project('delta')
    p.parts.verifier.overall = { value: null, abstained: true, reason_code: 'x' }
    p.parts.git = { status: 'abstained' }
    await renderView(withProjects(p))
    expect(card('delta')).toHaveTextContent('Verifier score — could not be scored')
    expect(card('delta')).toHaveTextContent('Git history (local commits) — could not be read')
  })

  it('lists an unknown extra part by its key with generic text', async () => {
    const p = project('delta')
    p.parts.ui = { status: 'ok', checks: 4, ok: true }
    await renderView(withProjects(p))
    expect(card('delta')).toHaveTextContent('ui — checks: 4; ok: true')
  })
})

describe('details toggle (AC6)', () => {
  it('hides the table until "Show details" is pressed, and hides it again', async () => {
    await renderView()
    expect(screen.queryByRole('table')).toBeNull()
    const toggle = screen.getByRole('button', { name: 'Show details' })
    expect(toggle).toHaveAttribute('aria-expanded', 'false')
    fireEvent.click(toggle)
    expect(screen.getByRole('table')).toBeInTheDocument()
    const hide = screen.getByRole('button', { name: 'Hide details' })
    expect(hide).toHaveAttribute('aria-expanded', 'true')
    fireEvent.click(hide)
    expect(screen.queryByRole('table')).toBeNull()
  })
})

describe('details table (unchanged)', () => {
  it('shows "no data" for no-data and absent parts, never zero', async () => {
    await renderView()
    await openDetails()
    const beta = row('beta')
    const cells = within(beta).getAllByRole('cell')
    // kernel, verifier, context, git
    expect(cells.map((c) => c.textContent)).toEqual(['no data', 'no data', 'no data', 'abstained'])
    expect(beta).not.toHaveTextContent(/\b0\b/)
  })

  it('always shows a verifier column, "no data" when the key is absent', async () => {
    await renderView()
    await openDetails()
    expect(screen.getByRole('columnheader', { name: 'Verifier' })).toBeInTheDocument()
    expect(within(row('alpha')).getAllByRole('cell')[1]).toHaveTextContent('no data')
  })

  it('renders a verifier part generically when the key appears', async () => {
    const withVerifier = project('alpha')
    withVerifier.parts.verifier = { status: 'ok', score: 87, dimensions: ['a'] }
    await renderView(withProjects(withVerifier))
    await openDetails()
    expect(within(row('alpha')).getAllByRole('cell')[1]).toHaveTextContent('score: 87')
  })

  it('shows kernel numbers and keeps a real zero when the data is there', async () => {
    await renderView()
    await openDetails()
    const alpha = within(row('alpha')).getAllByRole('cell')
    expect(alpha[0]).toHaveTextContent('40 tool calls')
    expect(alpha[0]).toHaveTextContent('0 errors')
    expect(alpha[3]).toHaveTextContent('7 commits')
  })

  it('highlights a stale index and shows unknown (not fresh) when stale is null', async () => {
    await renderView()
    await openDetails()
    const stale = within(row('alpha')).getByText('stale')
    expect(stale).toHaveClass('stale')
    expect(within(row('gamma')).getByText('unknown')).toBeInTheDocument()
    expect(within(row('gamma')).queryByText('fresh')).toBeNull()
    expect(within(row('gamma')).queryByText('stale')).toBeNull()
  })

  it('marks a project missing on disk', async () => {
    await renderView()
    await openDetails()
    expect(row('beta')).toHaveTextContent('missing on disk')
  })

  it('sorts the project column on click, both ways', async () => {
    await renderView()
    await openDetails()
    const names = () => screen.getAllByRole('rowheader').map((h) => h.textContent.split(' ')[0])
    expect(names()).toEqual(['alpha', 'beta', 'delta', 'gamma'])
    fireEvent.click(screen.getByRole('button', { name: /Project/ }))
    expect(names()).toEqual(['gamma', 'delta', 'beta', 'alpha'])
    expect(screen.getByRole('columnheader', { name: /Project/ })).toHaveAttribute('aria-sort', 'descending')
  })
})

describe('easy-ui overall', () => {
  it('shows easy-ui overall, or no data', async () => {
    await renderView()
    const ui = screen.getByRole('region', { name: 'easy-ui checks (overall)' })
    expect(ui).toHaveTextContent('sessions: 6')
    expect(ui).toHaveTextContent('failed: 1')
  })

  it('shows easy-ui "no data" explicitly, and the empty message with no toggle for 0 projects', async () => {
    await renderView({ kit: { projects: [], ui: { status: 'no data' } } })
    expect(screen.getByRole('region', { name: 'easy-ui checks (overall)' })).toHaveTextContent('no data')
    expect(screen.getByText(/No projects/)).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Show details' })).toBeNull()
  })
})

describe('errors', () => {
  it('reports a failed request', async () => {
    vi.stubGlobal('fetch', fakeFetch({ status: 403 }))
    render(<KitView />)
    expect(await screen.findByRole('alert')).toHaveTextContent('403')
  })
})

describe('scope (HUN-118)', () => {
  it('sends no params without a scope and says All', async () => {
    const f = fakeFetch()
    vi.stubGlobal('fetch', f)
    render(<KitView />)
    await screen.findByRole('heading', { name: /^Kit by project/ })
    expect(f.calls.map((c) => c.url)).toEqual(['/api/kit'])
    expect(screen.getByTestId('scope-line')).toHaveTextContent('Showing: All · All')
  })

  it('sends workspace= and account=, refetches on change, names the scope', async () => {
    const f = fakeFetch()
    vi.stubGlobal('fetch', f)
    const options = { workspaces: [], accounts: [{ id: 1, label: 'Claude company' }] }
    const view = render(<KitView scope={{ workspace: 'Acme', account: '' }} options={options} />)
    await screen.findByRole('heading', { name: /^Kit by project/ })
    expect(f.calls.map((c) => c.url)).toEqual(['/api/kit?workspace=Acme'])
    view.rerender(<KitView scope={{ workspace: 'Acme', account: '1' }} options={options} />)
    await waitFor(() => expect(f.calls.map((c) => c.url)).toEqual(['/api/kit?workspace=Acme', '/api/kit?workspace=Acme&account=1']))
    expect(await screen.findByTestId('scope-line')).toHaveTextContent('Showing: Acme · Claude company')
  })

  it('ignores a slow answer for an older scope', async () => {
    const answer = (name) => ({ ok: true, status: 200, json: async () => ({ projects: [{ project: name, missing_on_disk: false, parts: {} }], ui: { status: 'no data' } }) })
    let releaseOld
    const f = vi.fn((url) => (String(url).includes('account') ? Promise.resolve(answer('new-scope')) : new Promise((resolve) => (releaseOld = () => resolve(answer('old-scope'))))))
    vi.stubGlobal('fetch', f)
    const view = render(<KitView scope={{ workspace: 'Acme', account: '' }} />)
    view.rerender(<KitView scope={{ workspace: 'Acme', account: '1' }} />)
    expect(await screen.findByText('new-scope', { exact: false })).toBeInTheDocument()
    releaseOld()
    await new Promise((r) => setTimeout(r, 20))
    expect(screen.queryByText('old-scope', { exact: false })).toBeNull()
  })
})

describe('feed cards (HUN-121 AC4, AC8)', () => {
  it('the section header carries the project count', async () => {
    await renderView()
    expect(screen.getByRole('heading', { level: 2, name: 'Kit by project (4)' })).toBeInTheDocument()
  })

  it('each project is a feed card: icon with the initial, title, last-active time at the right, footer', async () => {
    await renderView()
    const delta = card('delta')
    expect(delta).toHaveClass('feed-card')
    expect(delta.querySelector('.feed-icon')).toHaveTextContent('D')
    expect(delta.querySelector('.feed-time')).toHaveTextContent('today')
    expect(card('alpha').querySelector('footer.feed-foot')).toHaveTextContent('Not set up here: Verifier')
    expect(card('beta').querySelector('.feed-time')).toBeNull()
  })
})
