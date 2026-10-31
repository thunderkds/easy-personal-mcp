import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { fireEvent, render, screen, within } from '@testing-library/react'
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
  await screen.findByTestId('board-summary')
  return fn
}
const column = (name) => screen.getByRole('region', { name: new RegExp(`^${name}`) })
const cards = (name) => within(column(name)).queryAllByRole('article')

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
  it('has one column per board status, in board order, each with its count', async () => {
    await renderView()
    const names = screen.getAllByRole('region').map((r) => r.querySelector('h2').textContent)
    expect(names.map((n) => n.replace(/\s*\(\d+\)$/, ''))).toEqual(['Todo', 'In Progress', 'Ready for Review', 'Done', 'Closed (will not do)', 'Blocked'])
    expect(within(column('Todo')).getByRole('heading', { level: 2 })).toHaveTextContent('Todo (3)')
    expect(within(column('In Progress')).getByRole('heading', { level: 2 })).toHaveTextContent('In Progress (2)')
    expect(within(column('Ready for Review')).getByRole('heading', { level: 2 })).toHaveTextContent('(1)')
    expect(within(column('Closed')).getByRole('heading', { level: 2 })).toHaveTextContent('(1)')
    expect(within(column('Blocked')).getByRole('heading', { level: 2 })).toHaveTextContent('(1)')
  })

  it('shows id, title and project on a card, and the same id in two projects as two cards', async () => {
    await renderView()
    const todo = cards('Todo')
    expect(todo).toHaveLength(3)
    const first = within(todo[0])
    expect(first.getByText('HUN-140')).toBeInTheDocument()
    expect(first.getByText('Work board')).toBeInTheDocument()
    expect(first.getByText('easy-personal')).toBeInTheDocument()
    expect(within(todo[2]).getByText('easy-ctx')).toBeInTheDocument()
    expect(within(todo[2]).getByText('HUN-140')).toBeInTheDocument()
  })

  it('tags cards of a project that names a board, and only those', async () => {
    await renderView()
    const todo = cards('Todo')
    expect(within(todo[0]).getByText('linear')).toBeInTheDocument()
    expect(within(todo[2]).queryByText('linear')).toBeNull()
  })

  it('shows titles as plain text, never as markup', async () => {
    await renderView()
    const card = cards('In Progress')[1]
    expect(within(card).getByText('<img src=x onerror=alert(1)> **plain**')).toBeInTheDocument()
    expect(card.querySelector('img')).toBeNull()
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

  it('shows an empty column as 0 with no cards', async () => {
    await renderView({ projects: [board.projects[1]] })
    expect(within(column('Blocked')).getByRole('heading', { level: 2 })).toHaveTextContent('Blocked (0)')
    expect(cards('Blocked')).toHaveLength(0)
  })
})

describe('header line and projects (AC4, FR-038)', () => {
  it('counts projects, synced projects with the newest pull age, and no data', async () => {
    await renderView()
    expect(screen.getByTestId('board-summary')).toHaveTextContent('3 projects · 1 synced (last pull 2 h ago) · 1 no data')
  })

  it('lists each project with its pull age, "not synced" or "no data"', async () => {
    await renderView()
    const list = screen.getByRole('list', { name: 'Projects' })
    const items = within(list).getAllByRole('listitem')
    expect(items[0]).toHaveTextContent('easy-personal')
    expect(items[0]).toHaveTextContent('linear')
    expect(items[0]).toHaveTextContent('pulled 2 h ago')
    expect(items[1]).toHaveTextContent('easy-ctx')
    expect(items[1]).toHaveTextContent('not synced')
    expect(items[2]).toHaveTextContent('gone')
    expect(items[2]).toHaveTextContent('no data')
  })

  it('says how many rows could not be read', async () => {
    await renderView()
    expect(screen.getByText(/2 rows could not be read/)).toBeInTheDocument()
  })

  it('uses singular words for one project and omits synced when none is', async () => {
    await renderView({ projects: [board.projects[1]] })
    expect(screen.getByTestId('board-summary')).toHaveTextContent('1 project · 0 synced · 0 no data')
  })

  it('says so when there are no projects', async () => {
    vi.stubGlobal('fetch', vi.fn(async () => ({ ok: true, status: 200, json: async () => ({ projects: [] }) })))
    render(<BoardView />)
    expect(await screen.findByText('No projects with usage on this machine.')).toBeInTheDocument()
  })
})

describe('filters (AC4)', () => {
  it('filters the columns by project and the counts follow', async () => {
    await renderView()
    fireEvent.change(screen.getByLabelText('project'), { target: { value: 'easy-ctx' } })
    expect(within(column('Todo')).getByRole('heading', { level: 2 })).toHaveTextContent('Todo (1)')
    expect(within(column('Done')).getByRole('heading', { level: 2 })).toHaveTextContent('Done (0)')
    expect(screen.getByTestId('board-summary')).toHaveTextContent('3 projects')
  })

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
