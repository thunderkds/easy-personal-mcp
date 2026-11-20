import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { act, cleanup, fireEvent, render, screen, within } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import BoardView, { AMBIGUOUS, NO_REPO } from './BoardView.jsx'
import { OFF } from './NewSessionSheet.jsx'
import { ToastProvider } from './Toast.jsx'

// "Now" is pinned; only Date is faked so promises keep working.
const NOW = new Date('2026-10-31T12:00:00Z')

const task = (id, title, status) => ({ id, title, status })
const dones = Array.from({ length: 41 }, (_, n) => task(`HUN-${100 + n}`, `Finished ${n + 1}`, 'Done'))
const board = {
  projects: [
    {
      project: 'easy-personal',
      status: 'ok',
      board: 'linear',
      last_pull: '2026-10-31T10:00:00Z',
      abstained: 2,
      tasks: [
        task('HUN-140', 'Work board', 'Todo'),
        task('HUN-141', 'Second todo', 'Todo'),
        task('HUN-133', 'Neon pass', 'In Progress'),
        task('HUN-131', 'Landing family', 'Ready for Review'),
        task('HUN-90', 'Needs the other repo', 'Blocked'),
        task('HUN-7', 'Dropped idea', 'Closed (will not do)'),
        ...dones,
      ],
    },
    {
      project: 'easy-ctx',
      status: 'ok',
      board: null,
      last_pull: null,
      abstained: 0,
      tasks: [task('HUN-140', 'Same id, other project', 'Todo'), task('T001', '<img src=x onerror=alert(1)> **plain**', 'In Progress')],
    },
    { project: 'gone', status: 'no data', board: null, last_pull: null, abstained: 0, tasks: [] },
  ],
}

// One answer per route: the board, the workspaces and the agent-run state that the Run button reads, and the start call.
const workspaces = {
  workspaces: [
    { id: 1, name: 'orphan', is_orphan: true, repos: ['/home/me/easy-ctx'] },
    { id: 2, name: 'acme', is_orphan: false, repos: ['/home/me/code/easy-personal', '/home/me/code/easy-ctx/'] },
  ],
}
const agentRun = { enabled: true, account: null, claude_found: true, running: false, last: null }
const reply = (status, body) => ({ ok: status < 400, status, json: async () => body })

function stub(body = board, status = 200, extra = {}) {
  const fn = vi.fn(async (url, init = {}) => {
    if (url === '/api/workspaces') return extra.workspaces === 404 ? reply(404, {}) : reply(200, extra.workspaces ?? workspaces)
    if (url === '/api/agent-run') return extra.agentRun === 404 ? reply(404, {}) : reply(200, extra.agentRun ?? agentRun)
    if (url === '/api/app-sessions' && init.method === 'POST') return extra.start ? extra.start(init) : reply(202, { id: 'new-session-1' })
    return reply(status, body)
  })
  vi.stubGlobal('fetch', fn)
  return fn
}
const boardCalls = (fn) => fn.mock.calls.filter(([url]) => String(url).startsWith('/api/board'))
const starts = (fn) => fn.mock.calls.filter(([url, init]) => url === '/api/app-sessions' && init?.method === 'POST')
const view = (props) => (
  <ToastProvider>
    <BoardView {...props} />
  </ToastProvider>
)
async function renderView(body, scope, extra) {
  const fn = stub(body, 200, extra)
  render(view({ scope }))
  await screen.findByRole('heading', { name: 'Board' })
  await screen.findByLabelText('project')
  return fn
}
async function renderView0(body) {
  stub(body)
  render(view())
  await screen.findByRole('heading', { name: 'Board' })
  await screen.findByRole('button', { name: /without a board/ })
}
const column = (name) => screen.getByRole('region', { name: new RegExp(`^${name}`) })
const cards = (name) => within(column(name)).queryAllByRole('article')
const pick = (label) => fireEvent.change(screen.getByLabelText('project'), { target: { value: screen.getByRole('option', { name: label }).value } })
const everyone = () => pick('All projects')
const settle = () => act(() => new Promise((resolve) => setTimeout(resolve, 0)))
const runIn = (card) => within(card).queryByRole('button', { name: /^Run / })
const prompt = (id, title) => `Work on task ${id}: ${title}. Follow this project's workflow; its guide is tasks/TASK_GUIDE_${id}.md if that file exists.`
const sent = (fn) => JSON.parse(starts(fn)[0][1].body)

beforeEach(() => {
  window.location.hash = '#secret-token'
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(NOW)
})
afterEach(() => {
  vi.useRealTimers()
  vi.unstubAllGlobals()
})

describe('columns (AC4)', () => {
  it('has one column per board status, in board order, each with its count, for the opened project', async () => {
    await renderView()
    const names = screen.getAllByRole('region').map((r) => r.querySelector('h2').textContent)
    expect(names.map((n) => n.replace(/\s*\(\d+\)$/, ''))).toEqual(['Todo', 'In Progress', 'Ready for Review', 'Done', 'Closed (will not do)', 'Blocked'])
    expect(within(column('Todo')).getByRole('heading', { level: 2 })).toHaveTextContent('Todo (2)')
    expect(within(column('In Progress')).getByRole('heading', { level: 2 })).toHaveTextContent('In Progress (1)')
    expect(within(column('Ready for Review')).getByRole('heading', { level: 2 })).toHaveTextContent('(1)')
    expect(within(column('Closed')).getByRole('heading', { level: 2 })).toHaveTextContent('(1)')
    expect(within(column('Blocked')).getByRole('heading', { level: 2 })).toHaveTextContent('(1)')
  })

  it('shows id and title, and no project name, on a card of one project (AC5)', async () => {
    await renderView()
    const todo = cards('Todo')
    expect(todo).toHaveLength(2)
    expect(within(todo[0]).getByText('HUN-140')).toBeInTheDocument()
    expect(within(todo[0]).getByText('Work board')).toBeInTheDocument()
    expect(within(todo[0]).queryByText('easy-personal')).toBeNull()
    expect(within(todo[0]).queryByText('linear')).toBeNull()
  })

  it('in All projects: shows the project name on each card, the board kind only for a project that names one, and the same id in two projects as two cards', async () => {
    await renderView()
    everyone()
    const todo = cards('Todo')
    expect(todo).toHaveLength(3)
    expect(within(todo[0]).getByText('easy-personal')).toBeInTheDocument()
    expect(within(todo[0]).getByText('linear')).toBeInTheDocument()
    expect(within(todo[2]).getByText('easy-ctx')).toBeInTheDocument()
    expect(within(todo[2]).getByText('HUN-140')).toBeInTheDocument()
    expect(within(todo[2]).queryByText('linear')).toBeNull()
  })

  it('shows titles as plain text, never as markup, with markdown marks removed (AC4)', async () => {
    await renderView()
    pick(/^easy-ctx/)
    const card = cards('In Progress')[0]
    expect(within(card).getByText('<img src=x onerror=alert(1)> plain')).toBeInTheDocument()
    expect(card.querySelector('img')).toBeNull()
  })

  it.each([
    ['**Replies read plainly**', 'Replies read plainly'],
    ['__Replies__ read', 'Replies read'],
    ['Fix `board.py` now', 'Fix board.py now'],
    ['A *light* touch', 'A light touch'],
    ['keep snake_case and 2*3*4 as is', 'keep snake_case and 2*3*4 as is'],
  ])('strips emphasis and code marks: %s', async (raw, plainText) => {
    const one = { ...board.projects[0], tasks: [task('HUN-1', raw, 'Todo')] }
    await renderView({ projects: [one] })
    expect(within(cards('Todo')[0]).getByText(plainText)).toBeInTheDocument()
  })

  it('keeps the full title as the accessible name and on hover; the clamp is CSS only (AC4)', async () => {
    const full = 'Long title word '.repeat(30).trim()
    const long = `**${full}**`
    const one = { ...board.projects[0], tasks: [task('HUN-1', long, 'Todo')] }
    await renderView({ projects: [one] })
    const card = cards('Todo')[0]
    expect(screen.getByRole('article', { name: `HUN-1 ${full}` })).toBe(card)
    expect(card.querySelector('.board-title')).toHaveAttribute('title', full)
    const css = readFileSync(resolve(process.cwd(), 'src/app.css'), 'utf8')
    expect(css).toMatch(/\.board-title \{[^}]*-webkit-line-clamp: 3/)
  })

  it('shows the Done toggle as a button with a chevron, so it reads as openable (T121 F16)', async () => {
    await renderView()
    const toggle = within(column('Done')).getByRole('button', { name: /Done/ })
    expect(toggle.querySelector('.board-chevron')).not.toBeNull()
    expect(toggle).toHaveAccessibleName('Done (41)')
  })

  it('keeps Done collapsed with its exact count, and expands to the last 5 with a way to see all', async () => {
    await renderView()
    expect(within(column('Done')).getByRole('heading', { level: 2 })).toHaveTextContent('Done (41)')
    expect(cards('Done')).toHaveLength(0)
    const toggle = within(column('Done')).getByRole('button', { name: /Done/ })
    expect(toggle).toHaveAttribute('aria-expanded', 'false')
    fireEvent.click(toggle)
    expect(toggle).toHaveAttribute('aria-expanded', 'true')
    expect(cards('Done').map((c) => within(c).getByText(/^HUN-/).textContent)).toEqual(['HUN-136', 'HUN-137', 'HUN-138', 'HUN-139', 'HUN-140'])
    fireEvent.click(within(column('Done')).getByRole('button', { name: 'Show all 41' }))
    expect(cards('Done')).toHaveLength(41)
    expect(within(column('Done')).queryByRole('button', { name: /Show all/ })).toBeNull()
  })

  it('shows an empty column as 0 with no cards, and a board with 0 tasks', async () => {
    await renderView({ projects: [{ ...board.projects[1], tasks: [] }] })
    expect(within(column('Blocked')).getByRole('heading', { level: 2 })).toHaveTextContent('Blocked (0)')
    expect(within(column('Todo')).getByRole('heading', { level: 2 })).toHaveTextContent('Todo (0)')
    expect(cards('Blocked')).toHaveLength(0)
  })
})

describe('project picker and default (AC1)', () => {
  it('opens on the project with the newest last_pull, even when it is listed later in the response', async () => {
    const older = { ...board.projects[0], project: 'older', last_pull: '2026-10-01T00:00:00Z', tasks: [task('HUN-1', 'Older task', 'Todo')] }
    await renderView({ projects: [older, board.projects[0], board.projects[1]] })
    expect(screen.getByLabelText('project')).toHaveDisplayValue(/^easy-personal/)
    expect(cards('Todo')).toHaveLength(2)
  })

  it('opens on the first project with tasks when none has a pull', async () => {
    const empty = { ...board.projects[1], project: 'empty', tasks: [] }
    await renderView({ projects: [empty, board.projects[1]] })
    expect(screen.getByLabelText('project')).toHaveDisplayValue(/^easy-ctx/)
  })

  it('lists projects with a board first, each with its open-task count, then All projects, and never a no-data project', async () => {
    const plainRepo = { ...board.projects[1], project: 'aaa-no-kind', tasks: [task('T1', 'x', 'Todo'), task('T2', 'y', 'Done')] }
    await renderView({ projects: [plainRepo, ...board.projects] })
    const options = within(screen.getByLabelText('project')).getAllByRole('option').map((o) => o.textContent)
    expect(options).toEqual(['easy-personal (5 open)', 'aaa-no-kind (1 open)', 'easy-ctx (2 open)', 'All projects'])
  })

  it('keeps entries distinct for two projects with the same folder name', async () => {
    const twin = { ...board.projects[0], board: 'github', last_pull: '2026-10-30T00:00:00Z', tasks: [task('HUN-9', 'Twin', 'Todo')] }
    await renderView({ projects: [board.projects[0], twin] })
    const options = within(screen.getByLabelText('project')).getAllByRole('option').map((o) => o.textContent)
    expect(options).toEqual(['easy-personal · linear (5 open)', 'easy-personal · github (1 open)', 'All projects'])
    pick('easy-personal · github (1 open)')
    expect(cards('Todo')).toHaveLength(1)
  })

  it('switches the columns and the counts to the picked project', async () => {
    await renderView()
    pick(/^easy-ctx/)
    expect(within(column('Todo')).getByRole('heading', { level: 2 })).toHaveTextContent('Todo (1)')
    expect(within(column('Done')).getByRole('heading', { level: 2 })).toHaveTextContent('Done (0)')
  })

  it('says whether the opened project is synced, with its pull age', async () => {
    await renderView()
    expect(screen.getByText('linear')).toBeInTheDocument()
    expect(screen.getByText('pulled 2 h ago')).toBeInTheDocument()
    pick(/^easy-ctx/)
    expect(screen.getByText('not synced')).toBeInTheDocument()
  })

  it('goes back to the default project when the scope changes and the picked one is gone', async () => {
    stub()
    const { rerender } = render(view({ scope: { workspace: 'a', account: '' } }))
    await screen.findByLabelText('project')
    pick(/^easy-ctx/)
    expect(screen.getByLabelText('project')).toHaveDisplayValue(/^easy-ctx/)
    rerender(view({ scope: { workspace: 'b', account: '' } }))
    await vi.waitFor(() => expect(screen.getByLabelText('project')).toHaveDisplayValue(/^easy-personal/))
  })
})

describe('projects without a board (AC2)', () => {
  it('hides no-data projects behind an "N without a board" toggle with aria-expanded', async () => {
    await renderView()
    expect(screen.queryByText('gone')).toBeNull()
    expect(screen.queryByRole('list', { name: 'Projects without a board' })).toBeNull()
    const toggle = screen.getByRole('button', { name: '1 without a board' })
    expect(toggle).toHaveAttribute('aria-expanded', 'false')
    fireEvent.click(toggle)
    expect(toggle).toHaveAttribute('aria-expanded', 'true')
    const list = screen.getByRole('list', { name: 'Projects without a board' })
    expect(within(list).getByText('gone')).toBeInTheDocument()
  })

  it('has no toggle when every project has a board', async () => {
    await renderView({ projects: [board.projects[0]] })
    expect(screen.queryByRole('button', { name: /without a board/ })).toBeNull()
  })

  it('says so, and keeps the toggle, when no project has a board', async () => {
    await renderView0({ projects: [board.projects[2], { ...board.projects[2], project: 'tmp-2' }] })
    expect(screen.getByText('No project on this machine has a board file.')).toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: '2 without a board' }))
    expect(screen.getByText('tmp-2')).toBeInTheDocument()
    expect(screen.queryByLabelText('project')).toBeNull()
  })
})

describe('gone folders (AC3, FR-054)', () => {
  it('says how many gone folders are hidden', async () => {
    await renderView({ ...board, gone: 138 })
    expect(screen.getByText('138 gone folders hidden')).toBeInTheDocument()
  })

  it('says nothing, and does not fail, when gone is missing or 0', async () => {
    await renderView()
    expect(screen.queryByText(/gone folder/)).toBeNull()
    cleanup()
    await renderView({ ...board, gone: 0 })
    expect(screen.queryByText(/gone folder/)).toBeNull()
  })
})

describe('header line and rows (FR-038)', () => {
  it('counts the projects with a board and the synced ones with the newest pull age', async () => {
    await renderView()
    expect(screen.getByTestId('board-summary')).toHaveTextContent('2 projects with a board · 1 synced (last pull 2 h ago)')
  })

  it('says how many rows could not be read, for the opened project', async () => {
    await renderView()
    expect(screen.getByText(/2 rows could not be read/)).toBeInTheDocument()
    pick(/^easy-ctx/)
    expect(screen.queryByText(/rows could not be read/)).toBeNull()
  })

  it('uses singular words for one project', async () => {
    await renderView({ projects: [board.projects[1]] })
    expect(screen.getByTestId('board-summary')).toHaveTextContent('1 project with a board · 0 synced')
  })

  it('says so when there are no projects', async () => {
    vi.stubGlobal('fetch', vi.fn(async () => ({ ok: true, status: 200, json: async () => ({ projects: [] }) })))
    render(view())
    expect(await screen.findByText('No projects with usage on this machine.')).toBeInTheDocument()
  })
})

describe('requests (AC4)', () => {
  it('sends the workspace and account of the global scope, and loads again when they change', async () => {
    const fn = stub()
    const { rerender } = render(view({ scope: { workspace: 'acme', account: '' } }))
    await screen.findByRole('heading', { name: 'Board' })
    await screen.findByTestId('board-summary')
    expect(boardCalls(fn).at(-1)[0]).toBe('/api/board?workspace=acme')
    rerender(view({ scope: { workspace: 'acme', account: '2' } }))
    await vi.waitFor(() => expect(boardCalls(fn).at(-1)[0]).toBe('/api/board?workspace=acme&account=2'))
  })

  it('sends the token header and no body', async () => {
    const fn = await renderView()
    expect(boardCalls(fn)[0][0]).toBe('/api/board')
    expect(boardCalls(fn)[0][1].headers['X-Dashboard-Token']).toBe('secret-token')
  })
})

describe('Run button (T131)', () => {
  it('AC1: every Todo, In Progress, Ready for Review and Blocked card has Run; Done and Closed cards have none', async () => {
    await renderView()
    await screen.findAllByRole('button', { name: /^Run / })
    for (const [name, n] of [['Todo', 2], ['In Progress', 1], ['Ready for Review', 1], ['Blocked', 1]]) {
      expect(cards(name)).toHaveLength(n)
      for (const card of cards(name)) expect(runIn(card)).not.toBeNull()
    }
    fireEvent.click(screen.getByRole('button', { name: /^Done/ }))
    expect(cards('Done').length).toBeGreaterThan(0)
    for (const card of [...cards('Done'), ...cards('Closed')]) expect(runIn(card)).toBeNull()
  })

  it('AC2, AC3, AC4: one click posts the fixed body for the matching workspace repo, then opens Live on the new id', async () => {
    const onOpenAppSession = vi.fn()
    const fn = stub()
    render(view({ onOpenAppSession }))
    const run = await within(await screen.findByRole('region', { name: /^Todo/ })).findAllByRole('button', { name: 'Run HUN-140' })
    fireEvent.click(run[0])
    await vi.waitFor(() => expect(onOpenAppSession).toHaveBeenCalledWith('new-session-1'))
    expect(starts(fn)).toHaveLength(1)
    const [, init] = starts(fn)[0]
    expect(init.headers['X-Dashboard-Token']).toBe('secret-token')
    expect(JSON.parse(init.body)).toEqual({
      workspace: 'acme',
      repo: '/home/me/code/easy-personal',
      account: null,
      model: 'default',
      effort: 'default',
      mode: 'default',
      worktree: false,
      prompt: prompt('HUN-140', 'Work board'),
      confirmed: true,
    })
    expect(init.body).not.toContain('bypassPermissions')
  })

  it('AC3: the title goes through the plain-text step', async () => {
    const one = { ...board.projects[0], tasks: [task('HUN-1', 'Fix `board.py` **now**', 'Todo')] }
    const fn = await renderView({ projects: [one] })
    fireEvent.click(await screen.findByRole('button', { name: 'Run HUN-1' }))
    await vi.waitFor(() => expect(starts(fn)).toHaveLength(1))
    expect(sent(fn).prompt).toBe(prompt('HUN-1', 'Fix board.py now'))
  })

  it('AC3: a huge title is cut so the whole prompt stays within 100 000 UTF-8 bytes, whole characters only, tail intact', async () => {
    const one = { ...board.projects[0], tasks: [task('HUN-1', '😀'.repeat(60_000), 'Todo')] }
    const fn = await renderView({ projects: [one] })
    fireEvent.click(await screen.findByRole('button', { name: 'Run HUN-1' }))
    await vi.waitFor(() => expect(starts(fn)).toHaveLength(1))
    const text = sent(fn).prompt
    expect(new TextEncoder().encode(text).length).toBeLessThanOrEqual(100_000)
    expect(text.isWellFormed()).toBe(true)
    expect(text.startsWith('Work on task HUN-1: 😀')).toBe(true)
    expect(text.endsWith(". Follow this project's workflow; its guide is tasks/TASK_GUIDE_HUN-1.md if that file exists.")).toBe(true)
  })

  it('AC5: app sessions off: Run is disabled and the off text is visible once, with a Settings link', async () => {
    const onOpenSettings = vi.fn()
    stub(board, 200, { agentRun: { ...agentRun, enabled: false } })
    render(view({ onOpenSettings }))
    const run = await screen.findAllByRole('button', { name: /^Run / })
    for (const button of run) expect(button).toBeDisabled()
    expect(screen.getAllByText(new RegExp(OFF.slice(0, 20)))).toHaveLength(1)
    fireEvent.click(screen.getByRole('button', { name: 'Open Settings' }))
    expect(onOpenSettings).toHaveBeenCalledTimes(1)
  })

  it('AC5: a project in no workspace: Run is disabled and the reason is visible text on its cards', async () => {
    await renderView(undefined, undefined, { workspaces: { workspaces: [{ id: 2, name: 'acme', is_orphan: false, repos: ['/home/me/code/other'] }] } })
    const run = await screen.findAllByRole('button', { name: /^Run / })
    for (const button of run) expect(button).toBeDisabled()
    expect(within(cards('Todo')[0]).getByText(NO_REPO)).toBeInTheDocument()
    expect(NO_REPO).toBe('Add this repo to a workspace to run it')
  })

  it('AC5: a repo kept only in the orphan workspace does not count', async () => {
    await renderView(undefined, undefined, { workspaces: { workspaces: [{ id: 1, name: 'orphan', is_orphan: true, repos: ['/home/me/code/easy-personal'] }] } })
    await screen.findAllByRole('button', { name: /^Run / })
    expect(within(cards('Todo')[0]).getByText(NO_REPO)).toBeInTheDocument()
  })

  it('AC5: two workspace repos with the same folder name: Run is disabled and says so', async () => {
    const two = { workspaces: [{ id: 2, name: 'acme', is_orphan: false, repos: ['/a/easy-personal'] }, { id: 3, name: 'home', is_orphan: false, repos: ['/b/easy-personal'] }] }
    const fn = await renderView(undefined, undefined, { workspaces: two })
    await screen.findAllByRole('button', { name: /^Run / })
    expect(runIn(cards('Todo')[0])).toBeDisabled()
    expect(within(cards('Todo')[0]).getByText(AMBIGUOUS)).toBeInTheDocument()
    fireEvent.click(runIn(cards('Todo')[0]))
    await settle()
    expect(starts(fn)).toHaveLength(0)
  })

  it('AC5: app sessions not available (agent-run answers 404): no Run button, no reason text', async () => {
    const fn = await renderView(undefined, undefined, { agentRun: 404 })
    await vi.waitFor(() => expect(fn.mock.calls.some(([url]) => url === '/api/agent-run')).toBe(true))
    await settle()
    expect(screen.queryAllByRole('button', { name: /^Run / })).toHaveLength(0)
    expect(screen.queryByText(OFF)).toBeNull()
    expect(screen.queryByText(NO_REPO)).toBeNull()
  })

  it('AC5: the board still shows when the workspaces cannot be read, with no Run button', async () => {
    await renderView(undefined, undefined, { workspaces: 404 })
    await settle()
    expect(screen.queryAllByRole('button', { name: /^Run / })).toHaveLength(0)
    expect(cards('Todo')).toHaveLength(2)
  })

  it('AC6: one click is one session: the button is busy while the post is out, a second click sends nothing', async () => {
    let finish
    const onOpenAppSession = vi.fn()
    const fn = stub(board, 200, { start: () => new Promise((resolve) => (finish = () => resolve(reply(202, { id: 's-9' })))) })
    render(view({ onOpenAppSession }))
    const run = (await screen.findAllByRole('button', { name: 'Run HUN-140' }))[0]
    fireEvent.click(run)
    await vi.waitFor(() => expect(run).toBeDisabled())
    expect(run).toHaveTextContent('Starting')
    fireEvent.click(run)
    await settle()
    expect(starts(fn)).toHaveLength(1)
    finish()
    await vi.waitFor(() => expect(onOpenAppSession).toHaveBeenCalledWith('s-9'))
    expect(starts(fn)).toHaveLength(1)
  })

  it.each([
    [403, 'App sessions are off. Choose an agent run account in Settings.'],
    [409, 'a session is already running in this folder'],
    [400, 'repo must be a repo of the workspace'],
    [500, 'internal error'],
  ])('AC6: a %s answer shows its message in a toast, opens nothing and is not retried', async (status, message) => {
    const onOpenAppSession = vi.fn()
    const fn = stub(board, 200, { start: () => reply(status, { error: message }) })
    render(view({ onOpenAppSession }))
    const run = (await screen.findAllByRole('button', { name: 'Run HUN-140' }))[0]
    fireEvent.click(run)
    expect(await screen.findByRole('alert')).toHaveTextContent(message)
    await settle()
    expect(starts(fn)).toHaveLength(1)
    expect(onOpenAppSession).not.toHaveBeenCalled()
    expect(run).toBeEnabled()
  })

  it('AC6: a failed request with no server message shows a fixed line', async () => {
    stub(board, 200, { start: () => Promise.reject(new TypeError('network down')) })
    render(view())
    fireEvent.click((await screen.findAllByRole('button', { name: 'Run HUN-140' }))[0])
    expect(await screen.findByRole('alert')).toHaveTextContent('Could not start the session. Try again.')
  })

  it('AC7: the same id in two projects stays tied to its own card (All projects)', async () => {
    const fn = await renderView()
    everyone()
    await screen.findAllByRole('button', { name: /^Run / })
    const todo = cards('Todo')
    expect(todo).toHaveLength(3)
    fireEvent.click(runIn(todo[2]))
    await vi.waitFor(() => expect(starts(fn)).toHaveLength(1))
    expect(sent(fn)).toMatchObject({ workspace: 'acme', repo: '/home/me/code/easy-ctx/', prompt: prompt('HUN-140', 'Same id, other project') })
    fireEvent.click(runIn(todo[0]))
    await vi.waitFor(() => expect(starts(fn)).toHaveLength(2))
    expect(JSON.parse(starts(fn)[1][1].body)).toMatchObject({ repo: '/home/me/code/easy-personal', prompt: prompt('HUN-140', 'Work board') })
  })

  it('AC10: the Run button uses the existing accent style (class primary)', async () => {
    await renderView()
    for (const run of await screen.findAllByRole('button', { name: /^Run / })) expect(run).toHaveClass('primary')
  })

  it('AC8: the card keeps its accessible name (id and title only)', async () => {
    await renderView()
    await screen.findAllByRole('button', { name: /^Run / })
    expect(screen.getByRole('article', { name: 'HUN-140 Work board' })).toBe(cards('Todo')[0])
  })
})

describe('states', () => {
  it('shows a loading status and then an error alert', async () => {
    stub({}, 500)
    render(view())
    expect(screen.getByRole('status')).toHaveTextContent('Loading board')
    expect(await screen.findByRole('alert')).toHaveTextContent('Could not load board')
  })
})

describe('design tokens (UI AC 2)', () => {
  it('styles the board with tokens only, no raw colour literals', () => {
    const css = readFileSync(resolve(process.cwd(), 'src/app.css'), 'utf8')
    const rules = css.split('\n').filter((l) => /^\.board|^ul\.board|^\.project-dot/.test(l.trim()))
    expect(rules.length).toBeGreaterThan(5)
    for (const line of rules) expect(line).not.toMatch(/#[0-9a-fA-F]{3,8}\b|rgba?\(|hsla?\(/)
  })
})
