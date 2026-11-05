import { render, screen, within, waitFor, fireEvent } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import WarningsPlanView from './WarningsPlanView.jsx'
import { ToastProvider } from './Toast.jsx'
import { fakeFetch, reportList, reportOne, warnings } from './test/fixtures.js'

// The copy button notifies through the toast host App provides.
const withToasts = (ui) => <ToastProvider>{ui}</ToastProvider>

async function renderView(overrides) {
  const f = fakeFetch(overrides)
  vi.stubGlobal('fetch', f)
  render(withToasts(<WarningsPlanView />))
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
    expect(screen.getByTestId('headline')).toHaveTextContent('Nothing to fix this week.')
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
    render(withToasts(<WarningsPlanView onOpenAgents={onOpenAgents} />))
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
    render(withToasts(<WarningsPlanView onOpenAgents={() => {}} />))
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
    render(withToasts(<WarningsPlanView />))
    expect(await screen.findByRole('alert')).toHaveTextContent('403')
    expect(f.calls[0].init.headers['X-Dashboard-Token']).toBe('secret-token')
  })
})

describe('scope (HUN-118)', () => {
  it('sends no params without a scope (the header names the scope, see App.test.jsx)', async () => {
    const f = await renderView()
    expect(f.calls.map((c) => c.url).filter((u) => !u.startsWith('/api/reports'))).toEqual(['/api/warnings'])
  })

  it('sends workspace= and account= to warnings and plan, and refetches on change', async () => {
    const f = fakeFetch()
    vi.stubGlobal('fetch', f)
    const view = render(withToasts(<WarningsPlanView scope={{ workspace: 'Acme Co', account: '' }} />))
    await screen.findByTestId('headline')
    const warningCalls = () => f.calls.map((c) => c.url).filter((u) => !u.startsWith('/api/reports')).sort()
    expect(warningCalls()).toEqual(['/api/warnings?workspace=Acme%20Co'])
    f.calls.length = 0
    view.rerender(withToasts(<WarningsPlanView scope={{ workspace: 'Acme Co', account: '2' }} />))
    await waitFor(() => expect(warningCalls().length).toBe(1))
    expect(warningCalls()).toEqual(['/api/warnings?workspace=Acme%20Co&account=2'])
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

  it('HUN-126 AC5: a "Fix now" card has the amber icon tile, no raw rule id is visible', async () => {
    await renderView()
    const cards = screen.getAllByRole('article')
    const tones = cards.map((c) => [c.querySelector('.chip').textContent, c.querySelector('.feed-icon').classList.contains('tone-amber')])
    for (const [chip, amber] of tones) expect(amber).toBe(chip === 'Fix now')
    expect(cards.some((c) => c.querySelector('.chip').textContent === 'Fix now')).toBe(true)
    for (const c of cards) expect(c.textContent).not.toContain(c.getAttribute('data-rule'))
  })
})

const COMMAND = '/mcp__easy-personal__investigate'
const jump = hit({
  rule_id: 'tokens_jump', label: 'aware', priority: 3, title: 'Token use jumped', cache_read_share: 0.9691, diagnosis: 'more_sessions',
  summary: 'Token use rose 213.1% in the last 7 days against the 7 days before; target 50% or less',
  steps: ['The rise comes from more sessions (81 → 396), not bigger ones.', 'Biggest rise: alpha. Decide if that pace is intended.'],
  causes: [
    { kind: 'project', name: 'alpha', new: false, text: 'alpha: 476.4k → 443.1M tokens, 2 → 11 sessions' },
    { kind: 'project', name: 'fresh-repo', new: true, text: 'fresh-repo: 0 → 12.5M tokens, 0 → 3 sessions' },
  ],
})

describe('one list, highest priority first (HUN-216 AC1)', () => {
  it('orders 2 hits by priority and shows each once', async () => {
    const a = hit({ rule_id: 'rule_a', title: 'Alpha rule', priority: 3 })
    const b = hit({ rule_id: 'rule_b', title: 'Beta rule', priority: 1 })
    await renderView({ warnings: { as_of_utc: 'x', hits: [a, b], abstained: [] } })
    const cards = screen.getAllByRole('article')
    expect(cards.map((c) => c.dataset.rule)).toEqual(['rule_b', 'rule_a'])
    expect(cards[0].querySelector('.feed-icon')).toHaveTextContent('1')
    expect(screen.getAllByText('Alpha rule')).toHaveLength(1)
    expect(screen.getAllByText('Beta rule')).toHaveLength(1)
  })
})

describe('Why this happened (HUN-216 AC2, AC3)', () => {
  it('lists every cause as a nested list item, marks a new project, then the diagnosis steps', async () => {
    await renderView({ warnings: { as_of_utc: 'x', hits: [jump], abstained: [] } })
    const card = screen.getByRole('article')
    const why = within(card).getByRole('group', { name: 'Why this happened' })
    const items = within(why).getAllByRole('listitem')
    expect(items).toHaveLength(2)
    expect(items[0]).toHaveTextContent('alpha: 476.4k → 443.1M tokens, 2 → 11 sessions')
    expect(items[0]).not.toHaveTextContent('new')
    expect(items[1]).toHaveTextContent('fresh-repo: 0 → 12.5M tokens, 0 → 3 sessions')
    expect(within(items[1]).getByText('new')).toBeInTheDocument()
    expect(card).toHaveTextContent('Token use jumped')
    expect(card).toHaveTextContent('Watch')
    expect(card).toHaveTextContent('Token use rose 213.1%')
    const steps = [...card.querySelectorAll('ol.steps > li')].map((li) => li.textContent)
    expect(steps).toEqual(jump.steps)
  })

  it('shows the cache-read share only when the hit has one', async () => {
    const plain = hit({ rule_id: 'rule_plain', title: 'Plain', priority: 2, cache_read_share: undefined })
    await renderView({ warnings: { as_of_utc: 'x', hits: [jump, plain], abstained: [] } })
    const cards = screen.getAllByRole('article')
    const byRule = (id) => cards.find((c) => c.dataset.rule === id)
    expect(byRule('tokens_jump')).toHaveTextContent('97% of these tokens are cache reads')
    expect(byRule('rule_plain').textContent).not.toMatch(/cache reads/)
  })

  it('shows 0% when the share is 0, not nothing', async () => {
    await renderView({ warnings: { as_of_utc: 'x', hits: [{ ...jump, cache_read_share: 0 }], abstained: [] } })
    expect(screen.getByRole('article')).toHaveTextContent('0% of these tokens are cache reads')
  })

  it('shows no block for a hit without causes, and still shows its steps', async () => {
    await renderView({ warnings: { as_of_utc: 'x', hits: [{ ...jump, causes: [], cache_read_share: undefined }], abstained: [] } })
    const card = screen.getByRole('article')
    expect(within(card).queryByText('Why this happened')).toBeNull()
    expect(card.querySelectorAll('ol.steps > li')).toHaveLength(2)
  })
})

describe('abstained list and investigate hint (HUN-216 AC4, AC5, AC6)', () => {
  it('lists the abstained rules with their reason after the hits, and the hint last', async () => {
    await renderView({ warnings: { as_of_utc: 'x', hits: [jump], abstained: abstainedFive.slice(0, 1) } })
    const card = screen.getByRole('article')
    const notChecked = screen.getByText('Not checked (1)').closest('details')
    const hint = screen.getByTestId('investigate-hint')
    expect(notChecked).toHaveTextContent('Title a')
    expect(notChecked).toHaveTextContent('No stored history')
    expect(card.compareDocumentPosition(notChecked) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
    expect(notChecked.compareDocumentPosition(hint) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
  })

  it('with no hits says nothing to fix this week, and still shows the abstained list and the hint', async () => {
    await renderView({ warnings: { as_of_utc: 'x', hits: [], abstained: abstainedFive } })
    expect(screen.getByTestId('headline')).toHaveTextContent('Nothing to fix this week.')
    expect(screen.getByText('Not checked (5)')).toBeInTheDocument()
    expect(screen.getByTestId('investigate-hint')).toHaveTextContent(`Ask Claude to investigate: run ${COMMAND} in Claude Code.`)
  })

  it('the copy button writes exactly the command to the clipboard and answers with a toast', async () => {
    const writeText = vi.fn().mockResolvedValue(undefined)
    vi.stubGlobal('navigator', { ...navigator, clipboard: { writeText } })
    await renderView()
    fireEvent.click(screen.getByRole('button', { name: 'Copy the investigate command' }))
    expect(writeText).toHaveBeenCalledWith(COMMAND)
    expect(await within(screen.getByTestId('toasts')).findByRole('status')).toHaveTextContent('Copied')
  })

  it('without a clipboard the button does not crash and an error toast says to copy by hand', async () => {
    vi.stubGlobal('navigator', { ...navigator, clipboard: undefined })
    await renderView()
    fireEvent.click(screen.getByRole('button', { name: 'Copy the investigate command' }))
    expect(await within(screen.getByTestId('toasts')).findByRole('alert')).toHaveTextContent('copy it by hand')
    expect(screen.getByTestId('investigate-hint')).toHaveTextContent(COMMAND)
  })

  it('a refused clipboard write also ends in the error toast', async () => {
    vi.stubGlobal('navigator', { ...navigator, clipboard: { writeText: vi.fn().mockRejectedValue(new Error('denied')) } })
    await renderView()
    fireEvent.click(screen.getByRole('button', { name: 'Copy the investigate command' }))
    expect(await within(screen.getByTestId('toasts')).findByRole('alert')).toHaveTextContent('copy it by hand')
  })
})

describe('HUN-217: the "new" cause chip', () => {
  it('wins over the base .chip colours (selector .chip.chip-new, amber)', async () => {
    const { readFileSync } = await import('node:fs')
    const { resolve } = await import('node:path')
    const css = readFileSync(resolve(process.cwd(), 'src/app.css'), 'utf8')
    expect(css).toMatch(/\n\.chip\.chip-new\s*\{[^}]*background: var\(--color-amber-soft\);[^}]*color: var\(--color-amber\)/)
  })
})

// HUN-222: Saved reports (FR-078 UI)
const EMPTY = 'No saved reports yet. Run /mcp__easy-personal__investigate in Claude Code.'
const LINES = 'Before and now, one line per rule'
const flat = (el) => el.textContent.replace(/\s+/g, ' ')
const renderReports = async (overrides = {}) => {
  const f = await renderView({ reports: reportList, ...overrides })
  const section = await screen.findByRole('region', { name: 'Saved reports' })
  await within(section).findAllByRole('button')
  return { f, section }
}
const reportCalls = (f) => f.calls.map((c) => c.url).filter((u) => /^\/api\/reports\/\d+$/.test(u))
const openFirst = async (section) => {
  fireEvent.click(within(section).getAllByRole('button')[0])
  await within(section).findByRole('list', { name: LINES })
  return within(within(section).getByRole('list', { name: LINES })).getAllByRole('listitem')
}

describe('saved reports list (HUN-222 AC1)', () => {
  it('lists both reports newest first with date, scope, fired count and the summary first line', async () => {
    const { f, section } = await renderReports()
    expect(f.calls.map((c) => c.url)).toContain('/api/reports')
    const rows = within(section).getAllByRole('button')
    expect(rows).toHaveLength(2)
    expect(rows[0]).toHaveTextContent(new Date('2026-11-05T01:40:00Z').toLocaleString())
    expect(rows[0]).toHaveTextContent('all workspaces')
    expect(rows[0]).toHaveTextContent('1 rule fired')
    expect(rows[0]).toHaveTextContent('Token use rose 218% from more sessions, not bigger ones.')
    expect(rows[1]).toHaveTextContent(new Date('2026-10-29T08:00:00Z').toLocaleString())
    expect(rows[1]).toHaveTextContent('Acme Co')
    expect(rows[1]).toHaveTextContent('0 rules fired')
    expect(rows[0]).toHaveAttribute('aria-expanded', 'false')
  })

  it('names the account when an account was saved', async () => {
    const list = { reports: [{ ...reportList.reports[0], workspace: 'Acme Co', account: '2' }, { ...reportList.reports[1], workspace: null, account: '3' }] }
    const { section } = await renderReports({ reports: list })
    const rows = within(section).getAllByRole('button')
    expect(rows[0]).toHaveTextContent('Acme Co / account 2')
    expect(rows[1]).toHaveTextContent('account 3')
  })

  it('AC5: an empty list shows the exact empty text and no rows', async () => {
    const f = fakeFetch({ reports: { reports: [] } })
    vi.stubGlobal('fetch', f)
    render(withToasts(<WarningsPlanView />))
    const section = await screen.findByRole('region', { name: 'Saved reports' })
    expect(await within(section).findByText(/No saved reports yet/)).toHaveTextContent(EMPTY)
    expect(within(section).queryAllByRole('button')).toHaveLength(0)
  })

  it('AC6: sits under the warnings list; the warnings and the investigate hint are unchanged', async () => {
    const { section } = await renderReports()
    const card = screen.getAllByRole('article')[0]
    expect(card.compareDocumentPosition(section) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
    expect(screen.getByTestId('headline')).toHaveTextContent('2 things to fix, 1 to watch')
    expect(screen.getByTestId('investigate-hint')).toHaveTextContent(`Ask Claude to investigate: run ${COMMAND} in Claude Code.`)
  })

  it('AC6: the section has no control that writes, closed or open', async () => {
    const { section } = await renderReports()
    await openFirst(section)
    for (const b of within(section).getAllByRole('button')) expect(b).toHaveAttribute('aria-expanded')
    expect(section.querySelector('input, textarea, select, form, a[href]')).toBeNull()
  })

  it('a list error says so in the section and leaves the warnings list shown', async () => {
    const f = fakeFetch()
    const failing = async (url, init) => (String(url).startsWith('/api/reports') ? { ok: false, status: 500, json: async () => ({}) } : f(url, init))
    vi.stubGlobal('fetch', failing)
    render(withToasts(<WarningsPlanView />))
    await screen.findByTestId('headline')
    const section = await screen.findByRole('region', { name: 'Saved reports' })
    expect(await within(section).findByText(/Could not load saved reports/)).toBeInTheDocument()
    expect(screen.getAllByRole('article')).toHaveLength(3)
  })
})

describe('open a saved report (HUN-222 AC2, AC3, AC4, AC7)', () => {
  it('AC2: rows are real buttons (Enter and Space work); click loads the report; opening another closes the first', async () => {
    const { f, section } = await renderReports()
    const [first, second] = within(section).getAllByRole('button')
    expect(first.tagName).toBe('BUTTON')
    expect(first).toHaveAttribute('type', 'button')
    fireEvent.click(first)
    expect(first).toHaveAttribute('aria-expanded', 'true')
    await within(section).findByText('Start a new session per task.')
    expect(reportCalls(f)).toEqual(['/api/reports/2'])
    fireEvent.click(second)
    expect(first).toHaveAttribute('aria-expanded', 'false')
    expect(second).toHaveAttribute('aria-expanded', 'true')
    expect(within(section).queryByText('Start a new session per task.')).toBeNull()
    expect(reportCalls(f)).toEqual(['/api/reports/2', '/api/reports/1'])
    expect(section.querySelectorAll('[aria-expanded="true"]')).toHaveLength(1)
  })

  it('a second click on the open row closes it', async () => {
    const { section } = await renderReports()
    const [first] = within(section).getAllByRole('button')
    await openFirst(section)
    fireEvent.click(first)
    expect(first).toHaveAttribute('aria-expanded', 'false')
    expect(within(section).queryByRole('list', { name: LINES })).toBeNull()
  })

  it('AC3: shows summary, fixes with tool and number, and one before / now line per rule', async () => {
    const { section } = await renderReports()
    const lines = await openFirst(section)
    expect(section).toHaveTextContent('Use <b>smaller</b> sessions.')
    const fixes = [...section.querySelectorAll('ol.report-fixes > li')]
    expect(fixes).toHaveLength(2)
    expect(fixes[0]).toHaveTextContent('Start a new session per task.')
    expect(fixes[0]).toHaveTextContent('session_outliers: 645,301')
    expect(fixes[1].textContent).toBe('Compact earlier.')
    expect(section).toHaveTextContent('now = last 7 days')
    expect(lines).toHaveLength(5)
    const line = (title) => lines.find((li) => li.textContent.includes(title))
    expect(flat(line('Token use jumped'))).toContain('then 218% fired → now 40% clear')
    expect(flat(line('Sessions grow too large'))).toContain('then 212,345 tokens fired → now 98,000 tokens clear')
    expect(flat(line('Low cache reuse'))).toContain('then 96.9% clear → now 97.1% clear')
    const jump = line('Token use jumped')
    expect(within(jump).getByText('→')).toHaveAttribute('aria-hidden', 'true')
    expect(within(jump).getByText('fired')).toHaveClass('chip-warning')
    expect(within(jump).getByText('clear')).toHaveClass('chip-ok')
  })

  it('AC4: an abstaining rule shows its reason, a rule not measured now says so; never a blank or 0', async () => {
    const { section } = await renderReports()
    const lines = await openFirst(section)
    const line = (title) => lines.find((li) => li.textContent.includes(title))
    expect(flat(line('Code index is out of date'))).toContain('then abstained: easy-context is down → now abstained: easy-context is down')
    const errors = line('Many tool calls fail')
    expect(flat(errors)).toContain('then 2% clear → now not measured now')
    for (const li of lines) expect(li.textContent).not.toMatch(/NaN|undefined|null/)
  })

  it('a count rule shows its count, and a measured 0 shows 0, not blank', async () => {
    const rules = [
      { rule_id: 'context_index_stale', title: 'Code index is out of date', before: { state: 'fired', measured: 2, threshold: 0, fired: true }, now: { state: 'clear', measured: 0, threshold: 0, fired: false } },
      { rule_id: 'tool_errors_high', title: 'Many tool calls fail', before: { state: 'clear', measured: 0, threshold: 0.1, fired: false }, now: { state: 'clear', measured: 0, threshold: 0.1, fired: false } },
    ]
    const { section } = await renderReports({ report: { ...reportOne, rules } })
    const lines = await openFirst(section)
    expect(flat(lines[0])).toContain('then 2 stale fired → now 0 stale clear')
    expect(flat(lines[1])).toContain('then 0% clear → now 0% clear')
  })

  it('shows no fixes block when the report has none', async () => {
    const { section } = await renderReports({ report: { ...reportOne, fixes: [] } })
    await openFirst(section)
    expect(section.querySelector('ol.report-fixes')).toBeNull()
    expect(within(section).queryByText('Fixes')).toBeNull()
  })

  it('edge: a report gone (404) shows "Report not found" inline and the list stays', async () => {
    const { section } = await renderReports({ report: 404 })
    fireEvent.click(within(section).getAllByRole('button')[0])
    expect(await within(section).findByText('Report not found')).toBeInTheDocument()
    expect(within(section).getAllByRole('button')).toHaveLength(2)
  })

  it('edge: another load error says it could not load the report', async () => {
    const f = fakeFetch({ reports: reportList })
    const failing = async (url, init) => (/^\/api\/reports\/\d+$/.test(String(url)) ? { ok: false, status: 500, json: async () => ({}) } : f(url, init))
    vi.stubGlobal('fetch', failing)
    render(withToasts(<WarningsPlanView />))
    const section = await screen.findByRole('region', { name: 'Saved reports' })
    fireEvent.click((await within(section).findAllByRole('button'))[0])
    expect(await within(section).findByText(/Could not load the report/)).toBeInTheDocument()
  })

  it('AC7: report text renders as text; a <b> in the summary shows literally', async () => {
    const { section } = await renderReports({ report: { ...reportOne, summary: 'Use <b>bold</b> <img src=x onerror=alert(1)>' } })
    await openFirst(section)
    expect(section).toHaveTextContent('Use <b>bold</b> <img src=x onerror=alert(1)>')
    expect(section.querySelector('b, img')).toBeNull()
  })

  it('edge: a very long first line sits in the clipped row line, and is shown in full when open', async () => {
    const long = 'Very long first line '.repeat(30).trim()
    const list = { reports: [{ ...reportList.reports[0], summary_first_line: long }] }
    const { section } = await renderReports({ reports: list, report: { ...reportOne, summary: long } })
    const row = within(section).getAllByRole('button')[0]
    expect(row.querySelector('.report-line')).toHaveTextContent(long)
    await openFirst(section)
    expect(section.querySelector('.report-summary')).toHaveTextContent(long)
  })
})

describe('saved reports style (HUN-222)', () => {
  it('uses tokens only: no colour literal in the new rules; the row line clips with an ellipsis', async () => {
    const { readFileSync } = await import('node:fs')
    const { resolve } = await import('node:path')
    const css = readFileSync(resolve(process.cwd(), 'src/app.css'), 'utf8')
    const rules = css.split('\n').filter((l) => /\.report-|\.ba-/.test(l))
    expect(rules.length).toBeGreaterThan(3)
    for (const l of rules) expect(l).not.toMatch(/#[0-9a-f]{3,8}\b|rgba?\(|hsla?\(/i)
    expect(css).toMatch(/\.report-line[^}]*text-overflow: ellipsis/)
  })
})
