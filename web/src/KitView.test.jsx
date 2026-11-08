import { render, screen, within, fireEvent, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import KitView, { commitStats, projectHealth } from './KitView.jsx'
import { fakeFetch, kit, workspaceKit } from './test/fixtures.js'

// "Now" is pinned to local noon on 2026-10-21; only Date is faked so timers keep working.
const NOW = new Date(2026, 9, 21, 12, 0, 0)

async function renderView(overrides) {
  vi.stubGlobal('fetch', fakeFetch(overrides))
  render(<KitView />)
  await screen.findByRole('heading', { name: /^Kit by project/ })
}
// The by-workspace block's two reads (HUN-239, HUN-241) are not the page's own read.
const notBlockRead = (u) => !u.startsWith('/api/kit/workspaces') && !u.startsWith('/api/kit/reports')
const card = (name) => screen.getByRole('article', { name })
const project = (name) => structuredClone(kit.projects.find((p) => p.project === name))
const withProjects = (...projects) => ({ kit: { ...kit, projects } })
async function openDetails() {
  fireEvent.click(screen.getByRole('button', { name: 'Show details' }))
}
const showMore = () => fireEvent.click(screen.getByRole('button', { name: /^Show \d+ more$/ }))
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
    showMore()
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
    showMore()
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
    showMore()
    expect(card('beta')).toHaveTextContent('No kit data yet.')
    expect(card('beta')).not.toHaveTextContent(/\b0\b/)
  })

  it('marks a project whose folder is missing', async () => {
    await renderView()
    showMore()
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
    showMore()
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
    // The page read and the by-workspace block each report their own failure.
    const alerts = await screen.findAllByRole('alert')
    expect(alerts[0]).toHaveTextContent('Could not load kit: /api/kit failed with status 403')
  })
})

describe('scope (HUN-118)', () => {
  it('sends no params without a scope (the header names the scope, see App.test.jsx)', async () => {
    const f = fakeFetch()
    vi.stubGlobal('fetch', f)
    render(<KitView />)
    await screen.findByRole('heading', { name: /^Kit by project/ })
    expect(f.calls.map((c) => c.url).filter(notBlockRead)).toEqual(['/api/kit'])
  })

  it('sends workspace= and account= and refetches on change', async () => {
    const f = fakeFetch()
    vi.stubGlobal('fetch', f)
    const view = render(<KitView scope={{ workspace: 'Acme', account: '' }} />)
    await screen.findByRole('heading', { name: /^Kit by project/ })
    const pageCalls = () => f.calls.map((c) => c.url).filter(notBlockRead)
    expect(pageCalls()).toEqual(['/api/kit?workspace=Acme'])
    view.rerender(<KitView scope={{ workspace: 'Acme', account: '1' }} />)
    await waitFor(() => expect(pageCalls()).toEqual(['/api/kit?workspace=Acme', '/api/kit?workspace=Acme&account=1']))
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
    showMore()
    const delta = card('delta')
    expect(delta).toHaveClass('feed-card')
    expect(delta.querySelector('.feed-icon')).toHaveTextContent('D')
    expect(delta.querySelector('.feed-time')).toHaveTextContent('today')
    expect(card('alpha').querySelector('footer.feed-foot')).toHaveTextContent('Not set up here: Verifier')
    expect(card('beta').querySelector('.feed-time')).toBeNull()
  })
})

describe('recent projects first, the rest folded (HUN-188 AC5, AC6)', () => {
  const idle = (name, lastUtc) => ({
    project: name,
    missing_on_disk: false,
    parts: { kernel: { ...project('alpha').parts.kernel, last_utc: lastUtc }, git: { status: 'no data' }, context: { status: 'no data' } },
  })
  const names = () => screen.getAllByRole('article').map((a) => within(a).getByRole('heading').textContent)

  it('shows projects active in the last 30 days and folds the others behind "Show N more"', async () => {
    await renderView(withProjects(project('delta'), idle('old-a', '2026-08-01T12:00:00Z'), idle('old-b', '2026-07-01T12:00:00Z'), project('alpha')))
    expect(names()).toEqual(['delta', 'alpha'])
    const toggle = screen.getByRole('button', { name: 'Show 2 more' })
    expect(toggle).toHaveAttribute('aria-expanded', 'false')
    fireEvent.click(toggle)
    expect(names()).toEqual(['delta', 'alpha', 'old-a', 'old-b'])
    const open = screen.getByRole('button', { name: 'Show fewer' })
    expect(open).toHaveAttribute('aria-expanded', 'true')
    fireEvent.click(open)
    expect(names()).toEqual(['delta', 'alpha'])
  })

  it('counts exactly 30 days back as recent and 31 as folded', async () => {
    await renderView(withProjects(idle('day-30', '2026-09-21T12:00:00Z'), idle('day-31', '2026-09-20T12:00:00Z')))
    expect(names()).toEqual(['day-30'])
    expect(screen.getByRole('button', { name: 'Show 1 more' })).toBeInTheDocument()
  })

  it('has no fold button when every project is recent', async () => {
    await renderView(withProjects(project('delta'), project('alpha')))
    expect(screen.queryByRole('button', { name: /^Show \d+ more$/ })).toBeNull()
  })

  it('when every project is idle shows the 10 newest and folds the rest', async () => {
    const many = Array.from({ length: 13 }, (_, i) => idle(`p${String(i).padStart(2, '0')}`, `2026-0${1 + (i % 6)}-0${1 + (i % 9)}T12:00:00Z`))
    await renderView(withProjects(...many))
    expect(names()).toHaveLength(10)
    expect(screen.getByRole('button', { name: 'Show 3 more' })).toHaveAttribute('aria-expanded', 'false')
  })

  it('says how many gone folders were hidden when the response has `gone` > 0, nothing otherwise', async () => {
    await renderView({ kit: { ...kit, gone: 3 } })
    expect(screen.getByText('3 gone folders hidden')).toBeInTheDocument()
  })
  it('says "1 gone folder hidden" for one', async () => {
    await renderView({ kit: { ...kit, gone: 1 } })
    expect(screen.getByText('1 gone folder hidden')).toBeInTheDocument()
  })
  it('shows no gone line when `gone` is 0 or missing', async () => {
    await renderView()
    expect(screen.queryByText(/gone folder/)).toBeNull()
  })
})

// HUN-226: a part whose status is "not in use" is not shown and never counted (FR-080)
describe('part "not in use" (HUN-226)', () => {
  const NOT_IN_USE = { status: 'not in use' }
  const unused = (name) => {
    const p = project(name)
    p.parts.context = { ...NOT_IN_USE }
    return p
  }

  it('AC2: projectHealth ignores a "not in use" part: alone it gives "No kit data", with a healthy kernel "Healthy"', () => {
    expect(projectHealth({ parts: { context: NOT_IN_USE } })).toEqual({ level: 'none', text: 'No kit data' })
    expect(projectHealth({ parts: { kernel: { status: 'ok', error_rate: 0 }, context: NOT_IN_USE } })).toEqual({ level: 'healthy', text: 'Healthy' })
  })
  it('AC2: a "not in use" part never adds the stale reason, even if a stale flag came with it', () => {
    const health = projectHealth({ parts: { kernel: { status: 'ok', error_rate: 0.5 }, context: { status: 'not in use', stale: true } } })
    expect(health.text).toBe('Needs attention: 50% of tool calls failed')
  })
  it('AC3: an in-use stale index still gives the "code index is out of date" reason', () => {
    expect(projectHealth({ parts: { context: { status: 'ok', stale: true } } }).text).toBe('Needs attention: code index is out of date')
  })

  it('AC1: cards show no Code index line, no "Not set up here" entry and no stale chip for a "not in use" project', async () => {
    await renderView(withProjects(unused('alpha'), unused('delta')))
    for (const name of ['alpha', 'delta']) {
      expect(card(name)).not.toHaveTextContent(/Code index|code index|easy-context|stale|out of date/)
    }
    expect(card('alpha')).toHaveTextContent('Not set up here: Verifier')
    expect(card('alpha')).toHaveTextContent('Healthy')
    expect(card('delta')).toHaveTextContent('Agent kit (tool calls the kit traced) — 9 kit tasks')
    expect(card('delta')).toHaveTextContent('Verifier score — 82/100')
    expect(card('delta')).not.toHaveTextContent('Not set up here')
  })

  it('AC1: when every project is "not in use" the Context column and the intro mention disappear from the page', async () => {
    await renderView(withProjects(unused('alpha'), unused('delta')))
    await openDetails()
    expect(screen.queryByRole('columnheader', { name: 'Context' })).toBeNull()
    expect(screen.getAllByRole('columnheader').map((h) => h.textContent.replace(/\s*[▲▼]$/, ''))).toEqual(['Project', 'Kernel', 'Verifier', 'Git'])
    expect(within(row('alpha')).getAllByRole('cell')).toHaveLength(3)
    expect(document.body).not.toHaveTextContent(/Code index|code index|Context/)
    expect(screen.getByText('How the agent kit and verifier are doing in each project.')).toBeInTheDocument()
  })

  it('mixed: only the "not in use" project hides the part; the column and the intro stay while another project has it', async () => {
    await renderView(withProjects(unused('alpha'), project('delta')))
    expect(card('alpha')).not.toHaveTextContent(/Code index|out of date/)
    expect(card('alpha')).toHaveTextContent('Healthy')
    expect(card('delta')).toHaveTextContent('Code index (easy-context) — indexed 2 days ago, fresh')
    expect(screen.getByText('How the agent kit, code index and verifier are doing in each project.')).toBeInTheDocument()
    await openDetails()
    expect(screen.getByRole('columnheader', { name: 'Context' })).toBeInTheDocument()
    expect(within(row('alpha')).getAllByRole('cell')[2].textContent).toBe('')
    expect(within(row('delta')).getAllByRole('cell')[2]).toHaveTextContent('fresh')
  })

  it('a context key missing from every project is treated as today: no data, the column stays', async () => {
    const a = project('alpha')
    delete a.parts.context
    await renderView(withProjects(a))
    await openDetails()
    expect(screen.getByRole('columnheader', { name: 'Context' })).toBeInTheDocument()
    expect(within(row('alpha')).getAllByRole('cell')[2]).toHaveTextContent('no data')
    expect(screen.getByText('How the agent kit, code index and verifier are doing in each project.')).toBeInTheDocument()
  })

  it('the "not in use" part never counts as data: an unused-only project reads "No kit data"', async () => {
    const p = { project: 'solo', missing_on_disk: false, parts: { context: { ...NOT_IN_USE } } }
    await renderView(withProjects(p))
    expect(card('solo')).toHaveTextContent('No kit data')
    expect(card('solo')).not.toHaveTextContent('Healthy')
    expect(card('solo')).not.toHaveTextContent(/Code index/)
  })
})

// HUN-239: the by-workspace block (FR-087, FR-088, FR-065); the block itself is tested in WorkspaceKit.test.jsx
describe('by workspace block (HUN-239)', () => {
  const MINUTE = 60000
  const sinceNow = (days) => new Date(Math.floor((NOW.getTime() - days * 86400000) / MINUTE) * MINUTE).toISOString()
  const wsUrls = (f) => f.calls.map((c) => c.url).filter((u) => u.startsWith('/api/kit/workspaces'))
  const blockHeading = () => screen.findByRole('heading', { name: /^Kit by workspace/ })

  it('AC1: shows one row per workspace from /api/kit/workspaces, above the project list', async () => {
    await renderView()
    const block = await blockHeading()
    expect(screen.getAllByRole('listitem', { name: /^(acme|beta)$/ })).toHaveLength(2)
    const projects = screen.getByRole('heading', { name: /^Kit by project/ })
    expect(block.compareDocumentPosition(projects) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
  })

  it('AC4: the project list renders while /api/kit/workspaces is pending, with the block loading', async () => {
    const base = fakeFetch()
    vi.stubGlobal('fetch', vi.fn((url, init) => (String(url).startsWith('/api/kit/workspaces') ? new Promise(() => {}) : base(url, init))))
    render(<KitView />)
    await screen.findByRole('heading', { name: /^Kit by project/ })
    expect(screen.getAllByRole('article').length).toBeGreaterThan(0)
    expect(screen.getByRole('status')).toHaveTextContent('Loading kit by workspace')
    expect(screen.getAllByRole('status')).toHaveLength(1)
  })

  it('AC4: a failed block read shows an inline alert inside the block only; the project list stays', async () => {
    const base = fakeFetch()
    vi.stubGlobal('fetch', vi.fn(async (url, init) => (String(url).startsWith('/api/kit/workspaces') ? { ok: false, status: 500, json: async () => ({}) } : base(url, init))))
    render(<KitView />)
    await screen.findByRole('heading', { name: /^Kit by project/ })
    const alert = await screen.findByRole('alert')
    expect(alert).toHaveTextContent('Could not load kit by workspace')
    expect(screen.getByRole('region', { name: /^Kit by workspace/ })).toContainElement(alert)
    expect(screen.getAllByRole('article').length).toBeGreaterThan(0)
  })

  it('AC5: the request carries the period as since and the account', async () => {
    const f = fakeFetch()
    vi.stubGlobal('fetch', f)
    render(<KitView scope={{ workspace: '', account: '3' }} period="7" />)
    await blockHeading()
    expect(wsUrls(f)).toEqual([`/api/kit/workspaces?since=${encodeURIComponent(sinceNow(7))}&account=3`])
  })

  it('AC5: changing the period refetches the block with the new since, All time sends none, the page read does not repeat', async () => {
    const f = fakeFetch()
    vi.stubGlobal('fetch', f)
    const view = render(<KitView period="7" />)
    await blockHeading()
    view.rerender(<KitView period="30" />)
    await waitFor(() => expect(wsUrls(f)).toHaveLength(2))
    expect(wsUrls(f)[1]).toBe(`/api/kit/workspaces?since=${encodeURIComponent(sinceNow(30))}`)
    view.rerender(<KitView period="all" />)
    await waitFor(() => expect(wsUrls(f)).toHaveLength(3))
    expect(wsUrls(f)[2]).toBe('/api/kit/workspaces')
    expect(f.calls.filter((c) => c.url === '/api/kit')).toHaveLength(1)
  })

  it('AC5: a late answer for an older period does not overwrite the newer one', async () => {
    const answer = (name) => ({ ok: true, status: 200, json: async () => ({ workspaces: [{ name }], ui_checks: { status: 'no data' } }) })
    const base = fakeFetch()
    let releaseOld
    vi.stubGlobal(
      'fetch',
      vi.fn((url, init) => {
        const u = String(url)
        if (!u.startsWith('/api/kit/workspaces')) return base(url, init)
        return u.includes(encodeURIComponent(sinceNow(7))) ? new Promise((resolve) => (releaseOld = () => resolve(answer('old-period')))) : Promise.resolve(answer('new-period'))
      }),
    )
    const view = render(<KitView period="7" />)
    view.rerender(<KitView period="30" />)
    expect(await screen.findByRole('listitem', { name: 'new-period' })).toBeInTheDocument()
    releaseOld()
    await new Promise((r) => setTimeout(r, 20))
    expect(screen.queryByText('old-period')).toBeNull()
    expect(screen.getByRole('listitem', { name: 'new-period' })).toBeInTheDocument()
  })

  it('AC6: the workspace filter marks its row, every workspace stays listed, and the filter is not sent', async () => {
    const f = fakeFetch()
    vi.stubGlobal('fetch', f)
    render(<KitView scope={{ workspace: 'beta', account: '' }} />)
    await blockHeading()
    expect(screen.getByRole('listitem', { name: 'beta' })).toHaveAttribute('aria-current', 'true')
    expect(screen.getByRole('listitem', { name: 'acme' })).not.toHaveAttribute('aria-current')
    expect(wsUrls(f)[0]).not.toContain('workspace=')
  })

  it('AC7: no workspaces gives the plain message and the Workspaces link', async () => {
    const open = vi.fn()
    vi.stubGlobal('fetch', fakeFetch({ workspaceKit: { workspaces: [], ui_checks: workspaceKit.ui_checks } }))
    render(<KitView onOpenWorkspaces={open} />)
    expect(await screen.findByText(/No workspaces yet\./)).toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: 'Open Workspaces' }))
    expect(open).toHaveBeenCalledTimes(1)
  })
})

// HUN-241: report folders under each workspace row (FR-089, FR-065); the lines themselves are tested in WorkspaceKit.test.jsx
describe('report folders in the by-workspace block (HUN-241)', () => {
  const reportUrls = (f) => f.calls.map((c) => c.url).filter((u) => u.startsWith('/api/kit/reports'))
  const rowOf = (name) => screen.getByRole('listitem', { name })

  it('AC1: the rows show the lines from /api/kit/reports', async () => {
    vi.stubGlobal('fetch', fakeFetch())
    render(<KitView />)
    expect(await screen.findByText('app/reports · 12 files · 3 easy-ui sessions · newest 2026-11-05')).toBeInTheDocument()
    expect(rowOf('beta')).toHaveTextContent('no report folders')
  })

  it('AC5: the request is GET /api/kit/reports with no query string and the page token header', async () => {
    const f = fakeFetch()
    vi.stubGlobal('fetch', f)
    render(<KitView scope={{ workspace: 'beta', account: '3' }} period="30" />)
    await screen.findByText(/app\/reports/)
    expect(reportUrls(f)).toEqual(['/api/kit/reports'])
    expect(f.calls.find((c) => c.url === '/api/kit/reports').init.headers).toEqual({ 'X-Dashboard-Token': 'secret-token' })
  })

  it('AC5: a period or scope change does not refetch it', async () => {
    const f = fakeFetch()
    vi.stubGlobal('fetch', f)
    const view = render(<KitView period="7" />)
    await screen.findByText(/app\/reports/)
    view.rerender(<KitView period="30" />)
    await waitFor(() => expect(f.calls.filter((c) => c.url.startsWith('/api/kit/workspaces'))).toHaveLength(2))
    view.rerender(<KitView period="all" scope={{ workspace: '', account: '1' }} />)
    await waitFor(() => expect(f.calls.filter((c) => c.url.startsWith('/api/kit/workspaces'))).toHaveLength(3))
    expect(reportUrls(f)).toEqual(['/api/kit/reports'])
  })

  it('AC4: the rows and the project list render while /api/kit/reports is pending; one pill at most, none for the reports', async () => {
    const base = fakeFetch()
    vi.stubGlobal('fetch', vi.fn((url, init) => (String(url).startsWith('/api/kit/reports') ? new Promise(() => {}) : base(url, init))))
    render(<KitView />)
    await screen.findByRole('heading', { name: /^Kit by project/ })
    expect(rowOf('acme')).toHaveTextContent('2 of 3 repos scored')
    expect(screen.getAllByRole('article').length).toBeGreaterThan(0)
    expect(screen.queryAllByRole('status')).toHaveLength(0)
    expect(screen.queryByText(/report folders/)).toBeNull()
  })

  it('AC4: the reports read starts with the other two reads, not after them', async () => {
    const f = fakeFetch()
    vi.stubGlobal('fetch', f)
    render(<KitView />)
    expect(f.calls.map((c) => c.url).filter((u) => u.startsWith('/api/kit')).sort()).toEqual(['/api/kit', '/api/kit/reports', expect.stringMatching(/^\/api\/kit\/workspaces/)].sort())
    await screen.findByText(/app\/reports/)
  })

  it('AC4: a failed reports read (HTTP 500) shows "report folders unavailable" in each row; the block data, the project list and no second pill', async () => {
    const base = fakeFetch()
    vi.stubGlobal('fetch', vi.fn(async (url, init) => (String(url).startsWith('/api/kit/reports') ? { ok: false, status: 500, json: async () => ({}) } : base(url, init))))
    render(<KitView />)
    await screen.findByRole('heading', { name: /^Kit by project/ })
    expect(await screen.findAllByText('report folders unavailable')).toHaveLength(2)
    expect(rowOf('acme')).toHaveTextContent('2 of 3 repos scored')
    expect(screen.queryByRole('alert')).toBeNull()
    expect(screen.queryAllByRole('status')).toHaveLength(0)
    expect(screen.getAllByRole('article').length).toBeGreaterThan(0)
  })

  it('AC4: reports answer first, block answer later: the lines appear with the rows', async () => {
    const base = fakeFetch()
    let release
    vi.stubGlobal('fetch', vi.fn((url, init) => (String(url).startsWith('/api/kit/workspaces') ? new Promise((r) => (release = () => r(base(url, init)))) : base(url, init))))
    render(<KitView />)
    await screen.findByRole('heading', { name: /^Kit by project/ })
    expect(screen.queryByText(/app\/reports/)).toBeNull()
    release()
    expect(await screen.findByText('app/reports · 12 files · 3 easy-ui sessions · newest 2026-11-05')).toBeInTheDocument()
  })

  it('HUN-243 AC7: opening a folder fetches its files once; a period change refetches no report files', async () => {
    const f = fakeFetch()
    vi.stubGlobal('fetch', f)
    const view = render(<KitView period="7" />)
    fireEvent.click(await screen.findByRole('button', { name: /^app\/reports/ }))
    await screen.findByText('score.html')
    const fileReads = () => f.calls.filter((c) => c.url.startsWith('/api/kit/report-files')).map((c) => c.url)
    expect(fileReads()).toEqual(['/api/kit/report-files?workspace=acme&repo_no=0&folder=reports'])
    view.rerender(<KitView period="30" />)
    await waitFor(() => expect(f.calls.filter((c) => c.url.startsWith('/api/kit/workspaces'))).toHaveLength(2))
    expect(fileReads()).toHaveLength(1)
  })
})
