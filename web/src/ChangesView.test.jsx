// T137 (FR-141, DDR-0010): the Changes view over the diff routes of T136.
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { cleanup, fireEvent, render, screen, within } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import ChangesView, { patchLines } from './ChangesView.jsx'

const workspaces = {
  workspaces: [
    { id: 1, name: 'orphan', is_orphan: true, repos: ['/home/me/hidden'] },
    { id: 2, name: 'acme', is_orphan: false, repos: ['/home/me/code/app', '/home/me/code/site'] },
    { id: 3, name: 'home', is_orphan: false, repos: ['/home/me/notes'] },
  ],
}
const overview = {
  name: 'app',
  tree: 'aaaa',
  base: 'main',
  worktrees: [
    { id: 'aaaa', folder: 'app', branch: 'main', head: 'c1', main: true },
    { id: 'bbbb', folder: 'app-wt', branch: 'topic', head: 'c2', main: false },
  ],
  branches: [
    { name: 'main', shown: 'main', kind: 'branch' },
    { name: 'topic', shown: 'topic', kind: 'branch' },
    { name: 'v1', shown: 'v1', kind: 'tag' },
  ],
  commits: [
    { id: '1111111aaaaaaaa', subject: 'Add the page', author: 'Ann', time: 1_790_000_000 },
    { id: '2222222bbbbbbbb', subject: 'First', author: 'Ann', time: 1_780_000_000 },
  ],
}
const row = (area, status, path, added, removed, extra = {}) => ({ area, status, path, shown: path, old_path: null, shown_old: null, added, removed, binary: false, openable: true, ...extra })
const listing = {
  files: [
    row('staged', 'renamed', 'new.js', 0, 0, { old_path: 'old.js', shown_old: 'old.js' }),
    row('unstaged', 'modified', 'src/a b.js', 3, 1),
    row('untracked', 'untracked', 'notes.txt', null, null),
  ],
  total: 3,
  truncated: false,
  base: null,
  head: null,
  filters_off: false,
}
const patch = {
  patch: 'diff --git a/x b/x\n--- a/x\n+++ b/x\n@@ -1,2 +1,2 @@\n same\n-<img src=x onerror=alert(1)>\n+++plus\n',
  cut: false,
  binary: false,
  note: null,
}

const reply = (status, body) => ({ ok: status < 400, status, json: async () => body })

function stub(answers = {}) {
  const fn = vi.fn(async (url) => {
    const [path, query = ''] = url.split('?')
    if (path === '/api/workspaces') return reply(200, answers.workspaces ?? workspaces)
    const given = answers[path]
    const found = typeof given === 'function' ? given(new URLSearchParams(query)) : given
    if (found) return found
    const repo = { ...overview, tree: new URLSearchParams(query).get('tree') ?? overview.tree } // the server answers with the tree asked for
    return reply(200, { '/api/diff/repo': repo, '/api/diff/files': listing, '/api/diff/file': patch }[path])
  })
  vi.stubGlobal('fetch', fn)
  return fn
}

const calls = (fn, path) => fn.mock.calls.map(([url]) => url).filter((url) => url.split('?')[0] === path)
const last = (fn, path) => new URLSearchParams(calls(fn, path).at(-1).split('?')[1])
const fileButton = (name) => screen.findByRole('button', { name: new RegExp(name) })
const choose = (label, value) => fireEvent.change(screen.getByRole('combobox', { name: label }), { target: { value } })
const compare = (name) => fireEvent.click(within(screen.getByRole('group', { name: 'Compare' })).getByRole('button', { name }))

afterEach(() => {
  cleanup()
  vi.unstubAllGlobals()
})

describe('Changes view', () => {
  it('lists the uncommitted files of the first workspace repo, grouped by area, with the token header', async () => {
    const fn = stub()
    render(<ChangesView />)
    expect(screen.getByRole('heading', { name: 'Changes' })).toBeInTheDocument()
    await fileButton('src/a b.js')
    expect(screen.getAllByRole('heading', { level: 3 }).map((h) => h.textContent)).toEqual(['Staged 1', 'Not staged 1', 'Untracked 1'])
    expect(await fileButton('new.js')).toHaveTextContent('renamed')
    expect(await fileButton('new.js')).toHaveTextContent('old.js → new.js')
    expect(await fileButton('src/a b.js')).toHaveTextContent('+3 −1')
    const repos = within(screen.getByRole('combobox', { name: 'Repository' })).getAllByRole('option').map((o) => o.textContent)
    expect(repos).toEqual(['app (acme)', 'site (acme)', 'notes (home)'])
    expect(Object.fromEntries(last(fn, '/api/diff/files'))).toEqual({ workspace: 'acme', repo_no: '0', tree: 'aaaa', compare: 'uncommitted' })
    expect(fn.mock.calls.at(-1)[1].headers).toHaveProperty('X-Dashboard-Token')
    expect(calls(fn, '/api/diff/file')).toEqual([])
  })

  it('opens one file and draws its patch as text with added and removed lines marked', async () => {
    const fn = stub()
    const { container } = render(<ChangesView />)
    const button = await fileButton('src/a b.js')
    fireEvent.click(button)
    expect(button).toHaveAttribute('aria-pressed', 'true')
    const region = await screen.findByRole('region', { name: 'Changes in src/a b.js' })
    expect(Object.fromEntries(last(fn, '/api/diff/file'))).toEqual({ workspace: 'acme', repo_no: '0', tree: 'aaaa', compare: 'uncommitted', area: 'unstaged', path: 'src/a b.js' })
    expect(region).toHaveTextContent('<img src=x onerror=alert(1)>')
    expect(container.querySelector('img')).toBeNull()
    expect(region.querySelector('[data-kind="del"]')).toHaveTextContent('-<img')
    expect(region.querySelector('[data-kind="add"]')).toHaveTextContent('+++plus')
  })

  it('filters the repositories by the workspace scope and asks for the picked one', async () => {
    const fn = stub()
    render(<ChangesView scope={{ workspace: 'home', account: '' }} />)
    await fileButton('notes.txt')
    expect(within(screen.getByRole('combobox', { name: 'Repository' })).getAllByRole('option')).toHaveLength(1)
    expect(last(fn, '/api/diff/repo').get('workspace')).toBe('home')
  })

  it('picks another work tree by its id', async () => {
    const fn = stub()
    render(<ChangesView />)
    await fileButton('notes.txt')
    choose('Work tree', 'bbbb')
    await fileButton('notes.txt')
    expect(last(fn, '/api/diff/files').get('tree')).toBe('bbbb')
    choose('Repository', 'acme/1')
    await fileButton('notes.txt')
    expect(last(fn, '/api/diff/repo').get('tree')).toBeNull()
    expect(last(fn, '/api/diff/files').get('repo_no')).toBe('1')
  })

  it('asks for each compare mode with its refs', async () => {
    const fn = stub()
    render(<ChangesView />)
    await fileButton('notes.txt')
    const asked = () => {
      const q = Object.fromEntries(last(fn, '/api/diff/files'))
      delete q.workspace
      delete q.repo_no
      delete q.tree
      return q
    }
    compare('Branch')
    await vi.waitFor(() => expect(asked()).toEqual({ compare: 'branch', base: 'refs/heads/main' }))
    expect(screen.getByText(/Committed changes only/)).toBeInTheDocument()
    compare('Commit')
    await vi.waitFor(() => expect(asked()).toEqual({ compare: 'commit', head: '1111111aaaaaaaa' }))
    choose('Commit', '2222222bbbbbbbb')
    await vi.waitFor(() => expect(asked().head).toBe('2222222bbbbbbbb'))
    compare('Two refs')
    await vi.waitFor(() => expect(asked()).toEqual({ compare: 'range', base: 'refs/heads/main', head: 'refs/heads/main' }))
    choose('To', 'refs/tags/v1')
    await vi.waitFor(() => expect(asked().head).toBe('refs/tags/v1'))
    choose('From', 'refs/heads/topic')
    await vi.waitFor(() => expect(asked()).toEqual({ compare: 'range', base: 'refs/heads/topic', head: 'refs/tags/v1' }))
  })

  it('shows and asks for a remote default base', async () => {
    const remote = {
      ...overview,
      base: 'origin/trunk',
      worktrees: [{ ...overview.worktrees[0], branch: 'a-feature' }],
      branches: [
        { name: 'a-feature', shown: 'a-feature', kind: 'branch' },
        { name: 'origin/trunk', shown: 'origin/trunk', kind: 'remote' },
      ],
    }
    const fn = stub({ '/api/diff/repo': reply(200, remote) })
    render(<ChangesView />)
    await fileButton('notes.txt')
    compare('Branch')
    await vi.waitFor(() => expect(last(fn, '/api/diff/files').get('base')).toBe('refs/remotes/origin/trunk'))
    expect(screen.getByRole('combobox', { name: 'Base' })).toHaveDisplayValue('origin/trunk')
    compare('Two refs')
    expect(screen.getByRole('combobox', { name: 'From' })).toHaveDisplayValue('origin/trunk')
    expect(screen.getByRole('combobox', { name: 'To' })).toHaveDisplayValue('a-feature')
  })

  it('shows a default base that is not in the ref list as itself, never as another ref', async () => {
    stub({ '/api/diff/repo': reply(200, { ...overview, base: 'origin/gone' }) })
    render(<ChangesView />)
    await fileButton('notes.txt')
    compare('Branch')
    expect(screen.getByRole('combobox', { name: 'Base' })).toHaveDisplayValue('origin/gone')
  })

  it('shows the escaped form of a default base that is not in the list, and asks with the real name', async () => {
    const odd = 'origin/ev\u202eil'
    const fn = stub({ '/api/diff/repo': reply(200, { ...overview, base: odd, base_shown: 'origin/ev\\u{202e}il' }) })
    render(<ChangesView />)
    await fileButton('notes.txt')
    compare('Branch')
    expect(screen.getByRole('combobox', { name: 'Base' })).toHaveDisplayValue('origin/ev\\u{202e}il')
    await vi.waitFor(() => expect(last(fn, '/api/diff/files').get('base')).toBe(`refs/remotes/${odd}`))
  })

  it('takes a remote default base as the remote one, even beside a local branch of that name', async () => {
    const branches = [
      { name: 'origin/trunk', shown: 'origin/trunk', kind: 'branch' },
      { name: 'origin/trunk', shown: 'origin/trunk', kind: 'remote' },
    ]
    const fn = stub({ '/api/diff/repo': reply(200, { ...overview, base: 'origin/trunk', branches }) })
    render(<ChangesView />)
    await fileButton('notes.txt')
    compare('Branch')
    await vi.waitFor(() => expect(last(fn, '/api/diff/files').get('base')).toBe('refs/remotes/origin/trunk'))
  })

  it('keeps showing the picked commit when it is no longer listed, never another one', async () => {
    let amended = false
    const newer = { id: '3333333ccccccc', subject: 'Add the page again', author: 'Ann', time: 1_790_000_100 }
    const fn = stub({ '/api/diff/repo': () => (amended ? reply(200, { ...overview, commits: [newer, overview.commits[1]] }) : null) })
    render(<ChangesView />)
    await fileButton('notes.txt')
    compare('Commit')
    choose('Commit', '2222222bbbbbbbb')
    choose('Commit', '1111111aaaaaaaa')
    await fileButton('notes.txt')
    amended = true
    const read = calls(fn, '/api/diff/files').length
    fireEvent.click(screen.getByRole('button', { name: 'Refresh' }))
    await vi.waitFor(() => expect(calls(fn, '/api/diff/files').length).toBeGreaterThan(read))
    const select = await screen.findByRole('combobox', { name: 'Commit' })
    expect(select).toHaveDisplayValue('(no longer listed) 1111111 Add the page — Ann')
    expect(select).toHaveValue('1111111aaaaaaaa')
    expect(last(fn, '/api/diff/files').get('head')).toBe('1111111aaaaaaaa')
    choose('Commit', '3333333ccccccc')
    await vi.waitFor(() => expect(last(fn, '/api/diff/files').get('head')).toBe('3333333ccccccc'))
    expect(screen.queryByRole('option', { name: /no longer listed/ })).toBeNull()
  })

  it('names a picked ref that is gone as it was shown when picked', async () => {
    const odd = 'to\u202epic'
    const there = { ...overview, branches: [overview.branches[0], { name: odd, shown: 'to\\u{202e}pic', kind: 'branch' }] }
    let removed = false
    const fn = stub({ '/api/diff/repo': () => reply(200, removed ? { ...there, branches: [overview.branches[0]] } : there) })
    render(<ChangesView />)
    await fileButton('notes.txt')
    compare('Two refs')
    choose('From', `refs/heads/${odd}`)
    await fileButton('notes.txt')
    removed = true
    fireEvent.click(screen.getByRole('button', { name: 'Refresh' }))
    await vi.waitFor(() => expect(screen.getByRole('combobox', { name: 'From' })).toHaveDisplayValue('(no longer listed) to\\u{202e}pic'))
    expect(last(fn, '/api/diff/files').get('base')).toBe(`refs/heads/${odd}`)
  })

  it('shows no base as picked when the repository has no default one', async () => {
    const fn = stub({ '/api/diff/repo': reply(200, { ...overview, base: null }) })
    render(<ChangesView />)
    await fileButton('notes.txt')
    compare('Branch')
    expect(screen.getByRole('combobox', { name: 'Base' })).toHaveDisplayValue('Pick a base')
    await vi.waitFor(() => expect(last(fn, '/api/diff/files').get('compare')).toBe('branch'))
    expect(last(fn, '/api/diff/files').get('base')).toBeNull()
    compare('Two refs')
    expect(screen.getByRole('combobox', { name: 'From' })).toHaveDisplayValue('main')
    await vi.waitFor(() => expect(last(fn, '/api/diff/files').get('base')).toBe('refs/heads/main'))
    compare('Branch')
    choose('Base', 'refs/tags/v1')
    await vi.waitFor(() => expect(last(fn, '/api/diff/files').get('base')).toBe('refs/tags/v1'))
  })

  it('keeps From where it is when To is picked first and there is no default base', async () => {
    const fn = stub({ '/api/diff/repo': reply(200, { ...overview, base: null }) })
    render(<ChangesView />)
    await fileButton('notes.txt')
    compare('Two refs')
    choose('To', 'refs/heads/topic')
    await vi.waitFor(() => expect(Object.fromEntries(last(fn, '/api/diff/files'))).toMatchObject({ base: 'refs/heads/main', head: 'refs/heads/topic' }))
    expect(screen.getByRole('combobox', { name: 'From' })).toHaveDisplayValue('main')
  })

  it('takes a master default base as the local branch, and a full default base as it is', async () => {
    for (const [base, shown] of [['master', 'master'], ['refs/heads/master', 'master'], ['refs/remotes/origin/trunk', 'origin/trunk']]) {
      const full = base.startsWith('refs/') ? base : `refs/heads/${base}`
      const branches = [{ name: shown, shown, kind: full.startsWith('refs/heads/') ? 'branch' : 'remote' }]
      const fn = stub({ '/api/diff/repo': reply(200, { ...overview, base, base_shown: shown, branches }) })
      render(<ChangesView />)
      await fileButton('notes.txt')
      compare('Branch')
      await vi.waitFor(() => expect(last(fn, '/api/diff/files').get('base')).toBe(full))
      expect(screen.getByRole('combobox', { name: 'Base' })).toHaveDisplayValue(shown)
      expect(screen.queryByRole('option', { name: /no longer listed/ })).toBeNull()
      cleanup()
    }
  })

  it('shows a commit picked among two refs under its own label', async () => {
    stub()
    render(<ChangesView />)
    await fileButton('notes.txt')
    compare('Two refs')
    choose('From', '2222222bbbbbbbb')
    expect(screen.getByRole('combobox', { name: 'From' })).toHaveDisplayValue('2222222 First')
    expect(screen.queryByRole('option', { name: /no longer listed/ })).toBeNull()
  })

  it('compares up to the newest commit in a work tree with no branch', async () => {
    const detached = { ...overview, worktrees: [{ ...overview.worktrees[0], branch: null }] }
    const fn = stub({ '/api/diff/repo': reply(200, detached) })
    render(<ChangesView />)
    await fileButton('notes.txt')
    compare('Two refs')
    await vi.waitFor(() => expect(last(fn, '/api/diff/files').get('head')).toBe('1111111aaaaaaaa'))
    expect(screen.getByRole('combobox', { name: 'To' })).toHaveDisplayValue('1111111 Add the page')
  })

  it('says a work tree that is not in the list is read, never naming another', async () => {
    stub({ '/api/diff/repo': reply(200, { ...overview, tree: null }) })
    render(<ChangesView />)
    await fileButton('notes.txt')
    expect(screen.getByRole('combobox', { name: 'Work tree' })).toHaveDisplayValue('This folder (not listed)')
  })

  it('tells two repositories of one folder name apart', async () => {
    stub({ workspaces: { workspaces: [{ id: 1, name: 'acme', is_orphan: false, repos: ['/home/me/a/app', '/home/me/b/app', '/home/me/site'] }] } })
    render(<ChangesView />)
    await fileButton('notes.txt')
    const labels = within(screen.getByRole('combobox', { name: 'Repository' })).getAllByRole('option').map((o) => o.textContent)
    expect(labels).toEqual(['/home/me/a/app (acme)', '/home/me/b/app (acme)', 'site (acme)'])
  })

  it('picks the branch of a work tree by its shown name', async () => {
    const odd = 'to\u202epic'
    const shown = 'to\\u{202e}pic'
    const body = { ...overview, worktrees: [{ ...overview.worktrees[0], branch: shown }], branches: [overview.branches[0], { name: odd, shown, kind: 'branch' }] }
    const fn = stub({ '/api/diff/repo': reply(200, body) })
    render(<ChangesView />)
    await fileButton('notes.txt')
    compare('Two refs')
    await vi.waitFor(() => expect(last(fn, '/api/diff/files').get('head')).toBe(`refs/heads/${odd}`))
  })

  it('says when branches, tags or work trees were left out', async () => {
    stub()
    render(<ChangesView />)
    await fileButton('notes.txt')
    expect(screen.queryByText(/are not listed/)).toBeNull()
    cleanup()
    stub({ '/api/diff/repo': reply(200, { ...overview, truncated: true }) })
    render(<ChangesView />)
    await fileButton('notes.txt')
    expect(screen.getByText('Some branches, tags or work trees are not listed: this repository has too many.')).toBeInTheDocument()
  })

  it('says there is no commit yet instead of asking for a compare that cannot work', async () => {
    const empty = { ...overview, base: null, branches: [], commits: [], worktrees: [{ ...overview.worktrees[0], head: null }] }
    const fn = stub({ '/api/diff/repo': reply(200, empty) })
    render(<ChangesView />)
    await fileButton('notes.txt')
    const asked = calls(fn, '/api/diff/files').length
    for (const mode of ['Branch', 'Commit', 'Two refs']) {
      compare(mode)
      expect(screen.getByText('This repository has no commit yet.')).toBeInTheDocument()
      expect(screen.queryByRole('combobox', { name: /Base|From|To|^Commit/ })).toBeNull()
    }
    expect(calls(fn, '/api/diff/files')).toHaveLength(asked)
  })

  it('says a branch has no commit when the repository has others, and still compares two refs', async () => {
    const branches = [overview.branches[1], overview.branches[0]] // the default base is not the first ref
    const lone = { ...overview, commits: [], branches, worktrees: [{ ...overview.worktrees[0], branch: 'lone', head: '0'.repeat(40) }] }
    const fn = stub({ '/api/diff/repo': reply(200, lone) })
    render(<ChangesView />)
    await fileButton('notes.txt')
    const asked = calls(fn, '/api/diff/files').length
    for (const mode of ['Branch', 'Commit']) {
      compare(mode)
      expect(screen.getByText('This branch has no commit yet.')).toBeInTheDocument()
      expect(screen.queryByText('This repository has no commit yet.')).toBeNull()
    }
    expect(calls(fn, '/api/diff/files')).toHaveLength(asked)
    compare('Two refs')
    expect(screen.queryByText(/no commit yet/)).toBeNull()
    expect(screen.getByRole('combobox', { name: 'To' })).toHaveDisplayValue('main')
    await vi.waitFor(() => expect(Object.fromEntries(last(fn, '/api/diff/files'))).toMatchObject({ compare: 'range', base: 'refs/heads/main', head: 'refs/heads/main' }))
  })

  it('compares the first ref with itself when a branch has no commit and there is no default base', async () => {
    const lone = { ...overview, base: null, commits: [], branches: [overview.branches[2]], worktrees: [{ ...overview.worktrees[0], branch: 'lone' }] }
    const fn = stub({ '/api/diff/repo': reply(200, lone) })
    render(<ChangesView />)
    await fileButton('notes.txt')
    compare('Two refs')
    await vi.waitFor(() => expect(Object.fromEntries(last(fn, '/api/diff/files'))).toMatchObject({ base: 'refs/tags/v1', head: 'refs/tags/v1' }))
  })

  it('does not take the focus back when the owner moved elsewhere during the read', async () => {
    let release
    stub({ '/api/diff/repo': (q) => (q.get('tree') === 'bbbb' ? new Promise((done) => (release = () => done(reply(200, { ...overview, tree: 'bbbb' })))) : null) })
    render(<ChangesView />)
    await fileButton('notes.txt')
    screen.getByRole('combobox', { name: 'Work tree' }).focus()
    choose('Work tree', 'bbbb')
    screen.getByRole('button', { name: 'Refresh' }).focus()
    release()
    await fileButton('notes.txt')
    expect(screen.getByRole('button', { name: 'Refresh' })).toHaveFocus()
  })

  it('keeps the focus on the work tree select while the other work tree is read', async () => {
    stub()
    render(<ChangesView />)
    await fileButton('notes.txt')
    screen.getByRole('combobox', { name: 'Work tree' }).focus()
    choose('Work tree', 'bbbb')
    await fileButton('notes.txt')
    expect(screen.getByRole('combobox', { name: 'Work tree' })).toHaveFocus()
    screen.getByRole('combobox', { name: 'Work tree' }).blur()
    compare('Commit') // any later draw of the page leaves the focus where it is
    await fileButton('notes.txt')
    expect(document.body).toHaveFocus()
  })

  it('offers a way back when the picked work tree is gone', async () => {
    let removed = false
    const fn = stub({ '/api/diff/repo': (q) => (removed && q.get('tree') === 'bbbb' ? reply(404, { error: 'no such work tree' }) : null) })
    render(<ChangesView />)
    await fileButton('notes.txt')
    choose('Work tree', 'bbbb')
    await fileButton('notes.txt')
    removed = true
    fireEvent.click(screen.getByRole('button', { name: 'Refresh' }))
    expect(await screen.findByRole('alert')).toHaveTextContent('no such work tree')
    fireEvent.click(screen.getByRole('button', { name: 'Back to the default work tree' }))
    await fileButton('notes.txt')
    expect(last(fn, '/api/diff/repo').get('tree')).toBeNull()
    expect(screen.queryByRole('alert')).toBeNull()
  })

  it('drops the picked refs on the way back to the default work tree', async () => {
    let removed = false
    const fn = stub({ '/api/diff/repo': (q) => (removed && q.get('tree') === 'bbbb' ? reply(404, { error: 'no such work tree' }) : null) })
    render(<ChangesView />)
    await fileButton('notes.txt')
    choose('Work tree', 'bbbb')
    await fileButton('notes.txt')
    compare('Two refs')
    choose('To', 'refs/tags/v1')
    await vi.waitFor(() => expect(last(fn, '/api/diff/files').get('head')).toBe('refs/tags/v1'))
    removed = true
    fireEvent.click(screen.getByRole('button', { name: 'Refresh' }))
    fireEvent.click(await screen.findByRole('button', { name: 'Back to the default work tree' }))
    await fileButton('notes.txt')
    expect(screen.getByRole('combobox', { name: 'To' })).toHaveDisplayValue('main')
    await vi.waitFor(() => expect(last(fn, '/api/diff/files').get('tree')).toBe('aaaa'))
    expect(last(fn, '/api/diff/files').get('head')).toBe('refs/heads/main')
  })

  it('reads everything again on Refresh and keeps the open file', async () => {
    const fn = stub()
    render(<ChangesView />)
    fireEvent.click(await fileButton('notes.txt'))
    await screen.findByRole('region', { name: 'Changes in notes.txt' })
    const before = [calls(fn, '/api/diff/repo').length, calls(fn, '/api/diff/files').length, calls(fn, '/api/diff/file').length]
    fireEvent.click(screen.getByRole('button', { name: 'Refresh' }))
    await screen.findByRole('region', { name: 'Changes in notes.txt' })
    await vi.waitFor(() => expect([calls(fn, '/api/diff/repo').length, calls(fn, '/api/diff/files').length, calls(fn, '/api/diff/file').length]).toEqual(before.map((n) => n + 1)))
  })

  it('says that it is reading again, keeps the last answer, then says when it read', async () => {
    vi.useFakeTimers({ toFake: ['Date'] })
    vi.setSystemTime(new Date(2026, 10, 26, 10, 0, 0))
    let wait = false
    const waiting = []
    stub({ '/api/diff/files': () => (wait ? new Promise((done) => waiting.push(() => done(reply(200, listing)))) : null) })
    render(<ChangesView />)
    await fileButton('notes.txt')
    const status = () => screen.getByRole('status')
    const at = (hour) => `Read at ${new Date(2026, 10, 26, hour, 0, 0).toLocaleTimeString()}`
    await vi.waitFor(() => expect(status()).toHaveTextContent(at(10)))
    wait = true
    vi.setSystemTime(new Date(2026, 10, 26, 11, 0, 0))
    fireEvent.click(screen.getByRole('button', { name: 'Refresh' }))
    await vi.waitFor(() => expect(waiting).toHaveLength(1))
    expect(status()).toHaveTextContent('Reading again…')
    expect(screen.getByRole('button', { name: /notes\.txt/ })).toBeInTheDocument()
    waiting[0]()
    await vi.waitFor(() => expect(status()).toHaveTextContent(at(11)))
    vi.setSystemTime(new Date(2026, 10, 26, 12, 0, 0)) // opening a file does not read the list again
    fireEvent.click(screen.getByRole('button', { name: /notes\.txt/ }))
    await screen.findByRole('region', { name: 'Changes in notes.txt' })
    expect(status()).toHaveTextContent(at(11))
    vi.useRealTimers()
  })

  it('opens on a session folder when Live asks, and leaves it for a repository', async () => {
    const fn = stub()
    render(<ChangesView changesTarget={{ session: 'abc-123', title: 'fix the page' }} />)
    await fileButton('notes.txt')
    expect(screen.getByRole('combobox', { name: 'Repository' })).toHaveDisplayValue('Session: fix the page')
    expect(Object.fromEntries(last(fn, '/api/diff/files'))).toEqual({ session: 'abc-123', tree: 'aaaa', compare: 'uncommitted' })
    expect(last(fn, '/api/diff/repo').get('tree')).toBeNull()
    choose('Repository', 'home/0')
    await vi.waitFor(() => expect(last(fn, '/api/diff/files').get('workspace')).toBe('home'))
    expect(last(fn, '/api/diff/files').get('session')).toBeNull()
  })

  it('says why a repo or a file cannot be read, in the server words', async () => {
    stub({ '/api/diff/repo': reply(404, { error: 'this folder is not a git work tree' }) })
    render(<ChangesView />)
    expect(await screen.findByRole('alert')).toHaveTextContent('this folder is not a git work tree')
    expect(screen.getByRole('status')).toHaveTextContent(/^Read at /) // so Refresh is seen on an error too
    cleanup()
    stub({ '/api/diff/file': reply(404, { error: 'that file is not in this compare; refresh the list' }) })
    render(<ChangesView />)
    fireEvent.click(await fileButton('notes.txt'))
    expect(await screen.findByRole('alert')).toHaveTextContent('refresh the list')
  })

  it('shows loading, never the last file\'s patch, while another file is read', async () => {
    let wait = false
    stub({ '/api/diff/file': () => (wait ? new Promise(() => {}) : null) })
    render(<ChangesView />)
    fireEvent.click(await fileButton('notes.txt'))
    await screen.findByRole('region', { name: 'Changes in notes.txt' })
    wait = true
    fireEvent.click(await fileButton('src/a b.js'))
    expect(screen.getByText('Loading the changes')).toBeInTheDocument()
    expect(screen.queryByRole('region', { name: /^Changes in/ })).toBeNull()
    expect(screen.queryByText(/onerror=alert/)).toBeNull()
  })

  it('says why the files cannot be listed', async () => {
    stub({ '/api/diff/files': reply(400, { error: 'that ref is not in this repository' }) })
    render(<ChangesView />)
    expect(await screen.findByRole('alert')).toHaveTextContent('that ref is not in this repository')
  })

  it('says when there is no repository, no change, a cut list, filters off, a binary file and a cut patch', async () => {
    stub({ workspaces: { workspaces: [] } })
    render(<ChangesView />)
    expect(await screen.findByText(/Add a repository to a workspace/)).toBeInTheDocument()
    cleanup()
    stub({ '/api/diff/files': reply(200, { ...listing, files: [], total: 0 }) })
    render(<ChangesView />)
    expect(await screen.findByText('No changes.')).toBeInTheDocument()
    cleanup()
    stub({
      '/api/diff/files': reply(200, { ...listing, total: 4000, truncated: true, filters_off: true }),
      '/api/diff/file': (q) => reply(200, q.get('path') === 'notes.txt' ? { patch: '', cut: false, binary: true, note: null } : { ...patch, cut: true, note: 'a note' }),
    })
    render(<ChangesView />)
    fireEvent.click(await fileButton('notes.txt'))
    expect(screen.getByText(/Showing 3 of 4,000 files/)).toBeInTheDocument()
    expect(screen.getByText(/git filters are off/)).toBeInTheDocument()
    expect(await screen.findByText('A binary file: its content is not shown.')).toBeInTheDocument()
    fireEvent.click(await fileButton('src/a b.js'))
    expect(await screen.findByText(/The patch is cut/)).toBeInTheDocument()
    expect(screen.getByText('a note')).toBeInTheDocument()
  })

  it('says binary where a file has no line counts to show', async () => {
    stub({ '/api/diff/files': reply(200, { ...listing, files: [row('unstaged', 'modified', 'logo.png', null, null, { binary: true })], total: 1 }) })
    render(<ChangesView />)
    expect(await fileButton('logo.png')).toHaveTextContent('binary')
  })

  it('cannot open a file whose name is not text', async () => {
    stub({ '/api/diff/files': reply(200, { ...listing, files: [row('unstaged', 'modified', 'bad�name', 1, 1, { openable: false })], total: 1 }) })
    render(<ChangesView />)
    expect(await fileButton('bad')).toBeDisabled()
    const button = await fileButton('bad')
    expect(within(button).getByText('cannot open: the name is not text')).toHaveClass('diff-note')
    expect(button.querySelector('.diff-count')).toBeNull()
    const css = readFileSync(resolve(process.cwd(), 'src/changes.css'), 'utf8')
    expect(css).toMatch(/\.diff-note \{[^}]*grid-column: 1 \/ -1;[^}]*grid-row: 3;/) // beside the status it was drawn over it
  })
})

describe('patchLines', () => {
  const kinds = (text) => patchLines(text).map((l) => l.kind)

  it('marks only lines inside a hunk, and starts over at the next file', () => {
    expect(kinds('diff --git a/x b/x\n--- a/x\n+++ b/x\n@@ -1 +1 @@\n--- gone\n+++ came\n\\ No newline\ndiff --git a/y b/y\n--- a/y\n')).toEqual(['meta', 'meta', 'meta', 'hunk', 'del', 'add', 'meta', 'meta', 'meta'])
  })

  it('reads both mark columns of a conflict', () => {
    expect(kinds('diff --cc a.txt\n@@@ -1,1 -1,1 +1,5 @@@\n++<<<<<<< HEAD\n +mine\n- theirs\n  same\n')).toEqual(['meta', 'hunk', 'add', 'add', 'del', 'same'])
  })

  it('draws nothing for an empty patch', () => {
    expect(patchLines('')).toEqual([])
  })
})

it('styles the view with tokens only', () => {
  const css = readFileSync(resolve(process.cwd(), 'src/changes.css'), 'utf8')
  expect(css).not.toMatch(/#[0-9a-f]{3,8}\b|rgba?\(|hsl\(/i)
  expect(css).not.toMatch(/font-family:(?!\s*var\()/)
})

it('keeps long ref and folder names inside the page, and the compare buttons in two columns on a phone', () => {
  const css = readFileSync(resolve(process.cwd(), 'src/changes.css'), 'utf8')
  expect(css).toMatch(/\.changes \.field \{[^}]*min-width: 0;[^}]*max-width: 100%;/)
  expect(css).toMatch(/@media \(max-width: 39\.999rem\) \{\s*\.changes \.periods \{[^}]*grid-template-columns: 1fr 1fr;/)
})
