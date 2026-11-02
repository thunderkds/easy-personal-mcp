import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { cleanup, fireEvent, render, screen, within } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import BoardView from './BoardView.jsx'

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

function stub(body = board, status = 200) {
  const fn = vi.fn(async () => ({ ok: status === 200, status, json: async () => body }))
  vi.stubGlobal('fetch', fn)
  return fn
}
async function renderView(body, scope) {
  const fn = stub(body)
  render(<BoardView scope={scope} />)
  await screen.findByRole('heading', { name: 'Board' })
  await screen.findByLabelText('project')
  return fn
}
async function renderView0(body) {
  stub(body)
  render(<BoardView />)
  await screen.findByRole('heading', { name: 'Board' })
  await screen.findByRole('button', { name: /without a board/ })
}
const column = (name) => screen.getByRole('region', { name: new RegExp(`^${name}`) })
const cards = (name) => within(column(name)).queryAllByRole('article')
const pick = (label) => fireEvent.change(screen.getByLabelText('project'), { target: { value: screen.getByRole('option', { name: label }).value } })
const everyone = () => pick('All projects')

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
    const { rerender } = render(<BoardView scope={{ workspace: 'a', account: '' }} />)
    await screen.findByLabelText('project')
    pick(/^easy-ctx/)
    expect(screen.getByLabelText('project')).toHaveDisplayValue(/^easy-ctx/)
    rerender(<BoardView scope={{ workspace: 'b', account: '' }} />)
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
    render(<BoardView />)
    expect(await screen.findByText('No projects with usage on this machine.')).toBeInTheDocument()
  })
})

describe('requests (AC4)', () => {
  it('sends the workspace and account of the global scope, and loads again when they change', async () => {
    const fn = stub()
    const { rerender } = render(<BoardView scope={{ workspace: 'acme', account: '' }} />)
    await screen.findByRole('heading', { name: 'Board' })
    await screen.findByTestId('board-summary')
    expect(fn.mock.calls.at(-1)[0]).toBe('/api/board?workspace=acme')
    rerender(<BoardView scope={{ workspace: 'acme', account: '2' }} />)
    await vi.waitFor(() => expect(fn.mock.calls.at(-1)[0]).toBe('/api/board?workspace=acme&account=2'))
  })

  it('sends the token header and no body', async () => {
    const fn = await renderView()
    expect(fn.mock.calls[0][0]).toBe('/api/board')
    expect(fn.mock.calls[0][1].headers['X-Dashboard-Token']).toBe('secret-token')
  })
})

describe('states', () => {
  it('shows a loading status and then an error alert', async () => {
    stub({}, 500)
    render(<BoardView />)
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
