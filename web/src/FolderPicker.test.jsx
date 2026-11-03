import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { render as rtlRender, screen, fireEvent, waitFor, within } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import WorkspacesView from './WorkspacesView.jsx'
import { ToastProvider } from './Toast.jsx'
import { fakeFetch, workspaceOverrides } from './test/fixtures.js'

const render = (ui) => rtlRender(ui, { wrapper: ToastProvider })
const click = (el) => fireEvent.click(el)

// jsdom has no <dialog>.showModal(): a stand-in that opens it (and records the call, so a plain `open` attribute would fail the test).
const modalCalls = []
beforeEach(() => {
  window.location.hash = '#secret-token'
  modalCalls.length = 0
  HTMLDialogElement.prototype.showModal = function () {
    modalCalls.push(this)
    this.setAttribute('open', '')
  }
  HTMLDialogElement.prototype.close = function () {
    this.removeAttribute('open')
  }
})
afterEach(() => vi.unstubAllGlobals())

/** A small fake disk: path -> {parent, dirs}. `/api/dirs` answers from it; everything else comes from the workspace fixtures. */
const DISK = {
  '/work': { parent: '/', dirs: [{ name: 'acme', repo: true }, { name: 'notes', repo: false }, { name: 'sp ace é', repo: false }] },
  '/work/acme': { parent: '/work', dirs: [{ name: 'web', repo: true }] },
  '/work/notes': { parent: '/work', dirs: [] },
  '/work/sp ace é': { parent: '/work', dirs: [] },
  '/': { parent: null, dirs: [{ name: 'work', repo: false }] },
  '/home/me': { parent: '/home', dirs: [] },
  '/home': { parent: '/', dirs: [{ name: 'me', repo: false }] },
}
function stub({ disk = DISK, start = '/work', failing = {}, hold } = {}) {
  const read = fakeFetch(workspaceOverrides())
  const fn = vi.fn(async (url, init = {}) => {
    const u = new URL(String(url), 'http://127.0.0.1')
    if (u.pathname !== '/api/dirs') return read(url, init)
    if (hold) await hold
    const path = u.searchParams.get('path') ?? start
    if (failing[path]) return { ok: false, status: failing[path].status, json: async () => ({ error: failing[path].error }) }
    const node = disk[path]
    if (!node) return { ok: false, status: 400, json: async () => ({ error: 'Not an existing folder' }) }
    return { ok: true, status: 200, json: async () => ({ path, parent: node.parent, home: '/home/me', dirs: node.dirs }) }
  })
  vi.stubGlobal('fetch', fn)
  return fn
}
const dirCalls = (fn) => fn.mock.calls.filter(([url]) => String(url).startsWith('/api/dirs'))

async function openForm() {
  render(<WorkspacesView />)
  await screen.findByRole('region', { name: 'company' })
  click(screen.getByRole('button', { name: '+ Add workspace' }))
  return screen.findByRole('region', { name: 'Workspace form' })
}
async function openBrowser() {
  const form = await openForm()
  click(within(form).getByRole('button', { name: 'Browse…' }))
  const dialog = await screen.findByRole('dialog', { name: 'Choose a repo folder' })
  await within(dialog).findByRole('list', { name: 'Sub-folders' })
  return { form, dialog }
}

describe('folder browser (HUN-203, FR-068)', () => {
  it('Browse… opens a labelled modal dialog at the dashboard folder, with the path, git badges and focus inside', async () => {
    const fn = stub()
    const { dialog } = await openBrowser()
    expect(modalCalls).toEqual([dialog])
    expect(dialog).toHaveAccessibleName('Choose a repo folder')
    expect(dirCalls(fn)[0][0]).toBe('/api/dirs') // no path: the server answers with where it was started
    expect(dirCalls(fn)[0][1].headers['X-Dashboard-Token']).toBe('secret-token')
    expect(within(dialog).getByLabelText('Current folder')).toHaveTextContent('/work')
    expect(dialog.contains(document.activeElement)).toBe(true)
    const acme = within(dialog).getByRole('button', { name: /acme/ })
    expect(within(acme).getByText('git')).toBeInTheDocument()
    expect(within(within(dialog).getByRole('button', { name: /notes/ })).queryByText('git')).toBeNull()
  })

  it('clicking a folder goes into it; Up goes to the parent; Home goes home; Up is disabled at the top', async () => {
    const fn = stub()
    const { dialog } = await openBrowser()
    click(within(dialog).getByRole('button', { name: /acme/ }))
    await waitFor(() => expect(within(dialog).getByLabelText('Current folder')).toHaveTextContent('/work/acme'))
    expect(dirCalls(fn)[1][0]).toBe('/api/dirs?path=%2Fwork%2Facme')
    expect(await within(dialog).findByRole('button', { name: /web/ })).toBeInTheDocument()
    click(within(dialog).getByRole('button', { name: 'Up' }))
    await waitFor(() => expect(within(dialog).getByLabelText('Current folder')).toHaveTextContent(/^\/work$/))
    click(within(dialog).getByRole('button', { name: 'Home' }))
    await waitFor(() => expect(within(dialog).getByLabelText('Current folder')).toHaveTextContent('/home/me'))
    expect(within(dialog).getByText('No sub-folders here.')).toBeInTheDocument()
    click(within(dialog).getByRole('button', { name: 'Up' }))
    await waitFor(() => expect(within(dialog).getByLabelText('Current folder')).toHaveTextContent(/^\/home$/ ))
  })

  it('disables Up at the file system root and builds child paths without a double slash', async () => {
    const fn = stub({ start: '/' })
    const { dialog } = await openBrowser()
    expect(within(dialog).getByRole('button', { name: 'Up' })).toBeDisabled()
    click(within(dialog).getByRole('button', { name: /work/ }))
    await waitFor(() => expect(dirCalls(fn)[1][0]).toBe('/api/dirs?path=%2Fwork'))
  })

  it('encodes a folder name with spaces and unicode in the query', async () => {
    const fn = stub()
    const { dialog } = await openBrowser()
    click(within(dialog).getByRole('button', { name: /sp ace é/ }))
    await waitFor(() => expect(dirCalls(fn)).toHaveLength(2))
    expect(new URL(dirCalls(fn)[1][0], 'http://x').searchParams.get('path')).toBe('/work/sp ace é')
  })

  it('Choose this folder adds the current path as a repo row and closes; focus returns to Browse…', async () => {
    stub()
    const { form, dialog } = await openBrowser()
    click(within(dialog).getByRole('button', { name: /acme/ }))
    await waitFor(() => expect(within(dialog).getByLabelText('Current folder')).toHaveTextContent('/work/acme'))
    await within(dialog).findByRole('button', { name: /web/ })
    click(within(dialog).getByRole('button', { name: 'Choose this folder' }))
    expect(screen.queryByRole('dialog')).toBeNull()
    expect(within(form).getByLabelText('Repo 1')).toHaveValue('/work/acme')
    expect(within(form).getByRole('button', { name: 'Browse…' })).toHaveFocus()
  })

  it('does not add a path that is already listed', async () => {
    stub()
    const { form } = await openBrowser()
    click(screen.getByRole('button', { name: 'Choose this folder' }))
    click(within(form).getByRole('button', { name: 'Browse…' }))
    const again = await screen.findByRole('dialog')
    await within(again).findByRole('list', { name: 'Sub-folders' })
    click(within(again).getByRole('button', { name: 'Choose this folder' }))
    expect(within(form).getAllByLabelText(/^Repo \d/)).toHaveLength(1)
  })

  it('does not add a duplicate of a typed path (trimmed)', async () => {
    stub()
    const form = await openForm()
    click(within(form).getByRole('button', { name: '+ Type a path' }))
    fireEvent.change(within(form).getByLabelText('Repo 1'), { target: { value: '  /work ' } })
    click(within(form).getByRole('button', { name: 'Browse…' }))
    const dialog = await screen.findByRole('dialog')
    await within(dialog).findByRole('list', { name: 'Sub-folders' })
    click(within(dialog).getByRole('button', { name: 'Choose this folder' }))
    expect(within(form).getAllByLabelText(/^Repo \d/)).toHaveLength(1)
  })

  it('Cancel closes without adding a row and returns the focus', async () => {
    stub()
    const { form, dialog } = await openBrowser()
    click(within(dialog).getByRole('button', { name: 'Cancel' }))
    expect(screen.queryByRole('dialog')).toBeNull()
    expect(within(form).queryByLabelText('Repo 1')).toBeNull()
    expect(within(form).getByRole('button', { name: 'Browse…' })).toHaveFocus()
  })

  it('Esc (the dialog cancel event) closes it, adds nothing and returns the focus', async () => {
    stub()
    const { form, dialog } = await openBrowser()
    const notCancelled = fireEvent(dialog, new Event('cancel', { cancelable: true }))
    expect(notCancelled).toBe(false) // the component took over: the native close is prevented, React unmounts the dialog
    expect(screen.queryByRole('dialog')).toBeNull()
    expect(within(form).queryByLabelText('Repo 1')).toBeNull()
    expect(within(form).getByRole('button', { name: 'Browse…' })).toHaveFocus()
  })

  it('"+ Type a path" still adds an empty editable row; each row keeps its input and ✕', async () => {
    stub()
    const form = await openForm()
    click(within(form).getByRole('button', { name: '+ Type a path' }))
    const input = within(form).getByLabelText('Repo 1')
    expect(input).toHaveValue('')
    fireEvent.change(input, { target: { value: '/typed' } })
    expect(input).toHaveValue('/typed')
    expect(within(form).getByRole('button', { name: 'Remove repo 1' })).toBeInTheDocument()
    expect(screen.queryByRole('dialog')).toBeNull()
    expect(within(form).queryByRole('button', { name: '+ Add repo' })).toBeNull()
  })

  it('shows an API error inline in the dialog with role=alert and keeps the last listing usable', async () => {
    stub({ failing: { '/work/acme': { status: 403, error: 'Cannot read this folder' } } })
    const { dialog } = await openBrowser()
    click(within(dialog).getByRole('button', { name: /acme/ }))
    expect(await within(dialog).findByRole('alert')).toHaveTextContent('Cannot read this folder')
    expect(within(dialog).getByLabelText('Current folder')).toHaveTextContent(/^\/work$/)
    expect(within(dialog).getByRole('button', { name: /notes/ })).toBeInTheDocument()
    click(within(dialog).getByRole('button', { name: /notes/ }))
    await waitFor(() => expect(within(dialog).queryByRole('alert')).toBeNull())
  })

  it('shows the error with only Cancel usable when the first listing fails', async () => {
    stub({ failing: { '/work': { status: 400, error: 'Not an existing folder' } } })
    const form = await openForm()
    click(within(form).getByRole('button', { name: 'Browse…' }))
    const dialog = await screen.findByRole('dialog')
    expect(await within(dialog).findByRole('alert')).toHaveTextContent('Not an existing folder')
    expect(within(dialog).getByRole('button', { name: 'Choose this folder' })).toBeDisabled()
    expect(within(dialog).getByRole('button', { name: 'Home' })).toBeDisabled()
    click(within(dialog).getByRole('button', { name: 'Cancel' }))
    expect(screen.queryByRole('dialog')).toBeNull()
  })

  it('shows a status inside the dialog while loading and keeps Choose disabled', async () => {
    let release
    const hold = new Promise((r) => (release = r))
    stub({ hold })
    const form = await openForm()
    click(within(form).getByRole('button', { name: 'Browse…' }))
    const dialog = await screen.findByRole('dialog')
    expect(within(dialog).getByRole('status')).toHaveTextContent('Loading folders')
    expect(within(dialog).getByRole('button', { name: 'Choose this folder' })).toBeDisabled()
    release()
    await within(dialog).findByRole('list', { name: 'Sub-folders' })
    expect(within(dialog).queryByRole('status')).toBeNull()
    expect(within(dialog).getByRole('button', { name: 'Choose this folder' })).toBeEnabled()
  })

  it('says when the list was cut', async () => {
    const disk = { ...DISK, '/work': { ...DISK['/work'], truncated: true } }
    vi.stubGlobal(
      'fetch',
      vi.fn(async (url, init = {}) =>
        String(url).startsWith('/api/dirs')
          ? { ok: true, status: 200, json: async () => ({ path: '/work', parent: '/', home: '/h', dirs: disk['/work'].dirs, truncated: true }) }
          : fakeFetch(workspaceOverrides())(url, init),
      ),
    )
    const { dialog } = await openBrowser()
    expect(within(dialog).getByText('Showing the first 3 folders.')).toBeInTheDocument()
  })

  it('the Browse… buttons never submit the form (nothing is written)', async () => {
    const fn = stub()
    const { dialog } = await openBrowser()
    click(within(dialog).getByRole('button', { name: 'Choose this folder' }))
    expect(fn.mock.calls.filter(([, init]) => init?.method)).toEqual([])
  })
})

describe('styles (HUN-203)', () => {
  const css = readFileSync(resolve(process.cwd(), 'src/app.css'), 'utf8')
  const rule = (selector) => css.match(new RegExp(`(?:^|\\n)${selector.replace(/[.[\]]/g, '\\$&')}\\s*\\{([^}]*)\\}`))?.[1] ?? ''

  it('puts at least --space-4 between the Repos buttons and the form Save / Back buttons', () => {
    expect(rule('.form-actions')).toMatch(/margin-top:\s*var\(--space-4\)/)
  })

  it('fits the dialog 16px (--space-4) inside a phone and scrolls the list inside it', () => {
    expect(rule('.folder-picker')).toContain('width: min(32rem, calc(100% - 2 * var(--space-4)))')
    expect(rule('.folder-picker')).toContain('max-height: calc(100dvh - 2 * var(--space-4))')
    expect(rule('.folder-list')).toContain('overflow-y: auto')
  })

  it('uses tokens only in its own rules', () => {
    const own = css.slice(css.indexOf('/* Folder browser (HUN-203)'))
    expect(own).not.toMatch(/#[0-9a-fA-F]{3,8}\b|rgba?\(/)
  })
})
