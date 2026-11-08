import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import WorkspaceKit, { workspaceKitPath } from './WorkspaceKit.jsx'
import { fakeFetch, kitReports, workspaceKit } from './test/fixtures.js'

const ready = (data = workspaceKit) => ({ status: 'ready', data })
const view = (state, props = {}) => render(<WorkspaceKit state={state} {...props} />)
const row = (name) => screen.getByRole('listitem', { name })
const withWorkspaces = (...workspaces) => ({ ...workspaceKit, workspaces })
const one = (overrides) => ({ ...structuredClone(workspaceKit.workspaces[0]), ...overrides })

describe('rows (AC1)', () => {
  it('renders one row per workspace with the verifier and easy-ui text', () => {
    view(ready())
    expect(screen.getAllByRole('listitem')).toHaveLength(2)
    const acme = row('acme')
    expect(acme).toHaveTextContent('2 of 3 repos scored')
    expect(acme).toHaveTextContent('mean 70')
    expect(acme).toHaveTextContent('lowest api 60')
    expect(acme).toHaveTextContent('newest report 2026-11-04')
    expect(acme).toHaveTextContent('2 sessions · 4 calls · last 2026-11-06')
  })

  it('draws the mean as a bar with a text equivalent', () => {
    view(ready())
    const bar = within(row('acme')).getByRole('progressbar', { name: 'Verifier mean 70 of 100' })
    expect(bar).toHaveAttribute('aria-valuenow', '70')
  })

  it('shows a mean with one decimal when it has one, and the singular for one session and one call', () => {
    const w = one({ verifier: { ...workspaceKit.workspaces[0].verifier, mean: 71.5 }, ui: { status: 'ok', sessions: 1, calls: 1, last_utc: '2026-11-06T12:00:00Z' } })
    view(ready(withWorkspaces(w)))
    expect(row('acme')).toHaveTextContent('mean 71.5')
    expect(row('acme')).toHaveTextContent('1 session · 1 call · last 2026-11-06')
  })

  it('is a labelled list with a heading that carries the workspace count', () => {
    view(ready())
    expect(screen.getByRole('heading', { level: 2, name: 'Kit by workspace (2)' })).toBeInTheDocument()
    expect(screen.getByRole('list', { name: 'Kit by workspace' })).toBeInTheDocument()
  })
})

describe('no data (AC2)', () => {
  it('shows "no data" text and no bar or number for a workspace without a score, and "not used" for easy-ui', () => {
    view(ready())
    const beta = row('beta')
    expect(beta).toHaveTextContent('no data')
    expect(beta).toHaveTextContent('not used')
    expect(within(beta).queryByRole('progressbar')).toBeNull()
    expect(beta).not.toHaveTextContent(/mean|lowest|newest|\b0\b/)
  })

  it('draws a real mean of 0 as a number with a bar, unlike "no data"', () => {
    const verifier = { status: 'ok', repos_scored: 1, repos_total: 1, mean: 0, lowest: { project: 'zero', value: 0 }, newest_utc: '2026-11-04T12:00:00Z' }
    view(ready(withWorkspaces(one({ verifier }))))
    expect(row('acme')).toHaveTextContent('mean 0')
    expect(row('acme')).toHaveTextContent('lowest zero 0')
    expect(within(row('acme')).getByRole('progressbar')).toHaveAttribute('aria-valuenow', '0')
    expect(row('acme')).not.toHaveTextContent('no data')
  })
})

describe('easy-ui checks line (AC3)', () => {
  it('shows the one overall line, once, under the rows', () => {
    view(ready())
    const line = screen.getByText('easy-ui checks (all workspaces): 3 sessions, 2 passed, 1 failed — not split by workspace')
    expect(screen.getAllByText(/not split by workspace/)).toHaveLength(1)
    expect(within(row('acme')).queryByText(/not split by workspace/)).toBeNull()
    expect(screen.getByRole('list', { name: 'Kit by workspace' }).compareDocumentPosition(line) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
  })

  it('shows no line when the checks have no data', () => {
    view(ready({ ...workspaceKit, ui_checks: { status: 'no data' } }))
    expect(screen.queryByText(/easy-ui checks/)).toBeNull()
  })

  it('shows no line when ui_checks is missing', () => {
    const { ui_checks: _gone, ...rest } = workspaceKit
    view(ready(rest))
    expect(screen.queryByText(/easy-ui checks/)).toBeNull()
  })
})

describe('loading and error (AC4)', () => {
  it('uses the Loading pill while the read is pending', () => {
    view({ status: 'loading' })
    expect(screen.getByRole('status')).toHaveTextContent('Loading kit by workspace')
    expect(screen.getByRole('heading', { name: 'Kit by workspace' })).toBeInTheDocument()
  })

  it('shows a failed read inline, in the block only, with role="alert"', () => {
    view({ status: 'error', message: '/api/kit/workspaces failed with status 500' })
    expect(screen.getByRole('alert')).toHaveTextContent('Could not load kit by workspace: /api/kit/workspaces failed with status 500')
    expect(screen.queryByRole('status')).toBeNull()
  })
})

describe('selected workspace (AC6)', () => {
  it('marks only the selected row with aria-current and still lists every row', () => {
    view(ready(), { selected: 'beta' })
    expect(row('beta')).toHaveAttribute('aria-current', 'true')
    expect(row('acme')).not.toHaveAttribute('aria-current')
    expect(screen.getAllByRole('listitem')).toHaveLength(2)
  })
  it('marks nothing without a selection', () => {
    view(ready())
    expect(document.querySelector('[aria-current]')).toBeNull()
  })
})

describe('empty and sparse data (AC7)', () => {
  it('says "No workspaces yet." and links to the Workspaces page', () => {
    const open = vi.fn()
    view(ready({ workspaces: [], ui_checks: { status: 'no data' } }), { onOpenWorkspaces: open })
    expect(screen.getByText(/No workspaces yet\./)).toBeInTheDocument()
    expect(screen.queryByRole('list', { name: 'Kit by workspace' })).toBeNull()
    fireEvent.click(screen.getByRole('button', { name: 'Open Workspaces' }))
    expect(open).toHaveBeenCalledTimes(1)
  })

  it('does not crash when optional fields are missing', () => {
    const sparse = { workspaces: [{ name: 'bare' }, { name: 'half', verifier: { status: 'ok', mean: 55 }, ui: { status: 'ok', calls: 2 } }] }
    view(ready(sparse))
    expect(row('bare')).toHaveTextContent('no data')
    expect(row('bare')).toHaveTextContent('not used')
    expect(row('half')).toHaveTextContent('mean 55')
    expect(row('half')).not.toHaveTextContent(/undefined|NaN|null/)
    expect(row('half')).toHaveTextContent('2 calls')
  })
})

describe('edge cases', () => {
  it('ellipsises a very long name and keeps the full name in the title', () => {
    const long = 'client-with-a-very-long-workspace-name-x'
    view(ready(withWorkspaces(one({ name: long }))))
    const name = screen.getByText(long)
    expect(name).toHaveAttribute('title', long)
    expect(name).toHaveClass('ws-name')
  })

  it('renders a planted markup name as text, never as HTML', () => {
    const planted = '<img src=x onerror="window.__pwned=1"><b>CANARY</b>'
    const { container } = view(ready(withWorkspaces(one({ name: planted, color: '#3366cc' }))))
    expect(container.querySelector('img')).toBeNull()
    expect(container.querySelector('b')).toBeNull()
    expect(screen.getByText(planted)).toBeInTheDocument()
    expect(window.__pwned).toBeUndefined()
  })

  it('uses the workspace colour for the dot only when it is #rrggbb', () => {
    const { container } = view(ready(withWorkspaces(one({ color: '#3366cc' }), one({ name: 'odd', color: 'red; background: url(x)' }))))
    const dots = container.querySelectorAll('.ws-dot')
    expect(dots[0].style.background).toBe('rgb(51, 102, 204)')
    expect(dots[1].getAttribute('style')).toBeNull()
  })
})

describe('request path (AC5)', () => {
  const NOW = Date.UTC(2026, 10, 7, 12, 30, 45)
  it('carries since for a period with days, and nothing for All time', () => {
    expect(workspaceKitPath('7', '', NOW)).toBe(`/api/kit/workspaces?since=${encodeURIComponent('2026-10-31T12:30:00.000Z')}`)
    expect(workspaceKitPath('30', '', NOW)).toBe(`/api/kit/workspaces?since=${encodeURIComponent('2026-10-08T12:30:00.000Z')}`)
    expect(workspaceKitPath('all', '', NOW)).toBe('/api/kit/workspaces')
  })
  it('carries the account, never the workspace', () => {
    expect(workspaceKitPath('all', '3', NOW)).toBe('/api/kit/workspaces?account=3')
    expect(workspaceKitPath('7', '3', NOW)).toBe(`/api/kit/workspaces?since=${encodeURIComponent('2026-10-31T12:30:00.000Z')}&account=3`)
  })
  it('falls back to the first period for an unknown id', () => {
    expect(workspaceKitPath('zzz', '', NOW)).toContain('since=')
  })
})

// HUN-241: report folders under each row (FR-089), from GET /api/kit/reports, read by the Kit view and passed as `reports`
describe('report folders (HUN-241)', () => {
  const folder = (overrides) => ({ repo: 'app', folder: 'reports', files: 12, files_capped: false, easy_ui_sessions: 3, newest_utc: '2026-11-05T12:00:00.000000Z', ...overrides })
  const reportsFor = (...workspaces) => ({ status: 'ready', data: { workspaces } })
  const lines = (name) => within(row(name)).queryAllByRole('listitem').map((li) => li.textContent)

  it('AC1: one muted line per folder, relative path, files, easy-ui sessions and newest day', () => {
    view(ready(), { reports: { status: 'ready', data: kitReports } })
    expect(lines('acme')).toEqual(['app/reports · 12 files · 3 easy-ui sessions · newest 2026-11-05', 'app/smoke-reports · 1 file · newest 2026-11-04'])
    expect(within(row('acme')).getByRole('list', { name: 'Report folders of acme' })).toBeInTheDocument()
  })

  it('AC1: leaves "0 easy-ui sessions" out, and says "1 easy-ui session" for one', () => {
    view(ready(), { reports: reportsFor({ name: 'acme', folders: [folder({ easy_ui_sessions: 0 }), folder({ folder: 'one', files: 2, easy_ui_sessions: 1 })] }) })
    expect(row('acme')).not.toHaveTextContent('0 easy-ui')
    expect(lines('acme')[1]).toBe('app/one · 2 files · 1 easy-ui session · newest 2026-11-05')
  })

  it('AC2: a capped count shows "2000+ files"', () => {
    view(ready(), { reports: reportsFor({ name: 'acme', folders: [folder({ files: 2000, files_capped: true })] }) })
    expect(lines('acme')[0]).toBe('app/reports · 2000+ files · 3 easy-ui sessions · newest 2026-11-05')
  })

  it('AC2: no newest part for an empty folder (newest_utc null)', () => {
    view(ready(), { reports: reportsFor({ name: 'acme', folders: [folder({ files: 0, easy_ui_sessions: 0, newest_utc: null })] }) })
    expect(lines('acme')).toEqual(['app/reports · 0 files'])
  })

  it('AC2: "no report folders" for folders [] and for a workspace missing from the answer (matched by name)', () => {
    view(ready(), { reports: reportsFor({ name: 'acme', folders: [] }) })
    expect(row('acme')).toHaveTextContent('no report folders')
    expect(row('beta')).toHaveTextContent('no report folders')
    expect(lines('beta')).toEqual([])
  })

  it('AC2: matches by name, not by position', () => {
    view(ready(), { reports: reportsFor({ name: 'beta', folders: [folder({ repo: 'zeta' })] }, { name: 'acme', folders: [] }) })
    expect(lines('beta')).toEqual(['zeta/reports · 12 files · 3 easy-ui sessions · newest 2026-11-05'])
    expect(row('acme')).toHaveTextContent('no report folders')
  })

  it('AC2: tolerates a workspace entry without a folders list', () => {
    view(ready(), { reports: reportsFor({ name: 'acme' }) })
    expect(row('acme')).toHaveTextContent('no report folders')
  })

  it('AC4: shows nothing and no pill while the reports read is pending; the rest of the row is unchanged', () => {
    view(ready(), { reports: { status: 'loading' } })
    expect(row('acme')).toHaveTextContent('2 of 3 repos scored')
    expect(row('acme')).not.toHaveTextContent(/report folders|reports/)
    expect(screen.queryByRole('status')).toBeNull()
  })

  it('AC4: a missing reports prop behaves like a pending read', () => {
    view(ready())
    expect(screen.queryByText(/report folders/)).toBeNull()
  })

  it('AC4: a failed reports read puts one small "report folders unavailable" in each row, no alert, rest unchanged', () => {
    view(ready(), { reports: { status: 'error', message: '/api/kit/reports failed with status 500' } })
    expect(screen.getAllByText('report folders unavailable')).toHaveLength(2)
    expect(within(row('acme')).getByText('report folders unavailable')).toBeInTheDocument()
    expect(screen.queryByRole('alert')).toBeNull()
    expect(screen.queryByRole('status')).toBeNull()
    expect(row('acme')).toHaveTextContent('2 of 3 repos scored')
    expect(screen.getByText(/easy-ui checks \(all workspaces\)/)).toBeInTheDocument()
  })

  it('AC4: the reports read ready while the block read is still loading shows no lines (the block has no rows yet)', () => {
    view({ status: 'loading' }, { reports: { status: 'ready', data: kitReports } })
    expect(screen.queryByText(/app\/reports/)).toBeNull()
  })

  it('AC6: ellipsises a long line with the full text in title', () => {
    const long = folder({ repo: 'a-very-long-repository-folder-name', folder: 'smoke-reports-with-an-extremely-long-folder-name' })
    view(ready(), { reports: reportsFor({ name: 'acme', folders: [long] }) })
    const item = within(row('acme')).getAllByRole('listitem')[0]
    const toggle = within(item).getByRole('button')
    expect(toggle).toHaveAttribute('title', 'a-very-long-repository-folder-name/smoke-reports-with-an-extremely-long-folder-name · 12 files · 3 easy-ui sessions · newest 2026-11-05')
    expect(item).toHaveClass('ws-report')
  })

  it('AC6: a planted markup folder name renders as text, never as HTML', () => {
    const planted = '<img src=x onerror="window.__pwned2=1"><b>CANARY</b>'
    const { container } = view(ready(), { reports: reportsFor({ name: 'acme', folders: [folder({ repo: planted, folder: '<script>1</script>' })] }) })
    expect(container.querySelector('img')).toBeNull()
    expect(container.querySelector('b')).toBeNull()
    expect(container.querySelector('script')).toBeNull()
    expect(within(row('acme')).getAllByRole('listitem')[0]).toHaveTextContent(`${planted}/<script>1</script> · 12 files`)
    expect(window.__pwned2).toBeUndefined()
  })

  it('AC6: the report styles use tokens only (no #hex or rgb())', () => {
    const css = readFileSync(resolve(process.cwd(), 'src/app.css'), 'utf8')
    const block = css.split('\n').filter((l) => l.includes('.ws-report')).join('\n')
    expect(block).toContain('.ws-reports')
    expect(block).not.toMatch(/#[0-9a-fA-F]{3,8}\b|rgba?\(/)
  })
})

// HUN-243: a report folder line opens to its files; a file opens in a new tab (FR-090)
describe('report files (HUN-243)', () => {
  const ready2 = { status: 'ready', data: kitReports }
  const folderButton = (text = /^app\/reports/) => within(row('acme')).getByRole('button', { name: text })
  const fileUrls = (f) => f.calls.map((c) => c.url).filter((u) => u.startsWith('/api/kit/report-files'))
  const tab = () => ({ opener: 'set', location: '', close: vi.fn() })
  let f
  beforeEach(() => {
    window.location.hash = '#secret-token'
    f = fakeFetch()
    vi.stubGlobal('fetch', f)
  })
  afterEach(() => {
    vi.useRealTimers()
    vi.unstubAllGlobals()
    vi.restoreAllMocks()
  })
  const openFolder = async (props = {}) => {
    view(ready(), { reports: ready2, ...props })
    fireEvent.click(folderButton())
    await screen.findByText('score.html')
  }

  it('AC1: the line is a button with aria-expanded; one request per first open, none for close/open again', async () => {
    view(ready(), { reports: ready2 })
    expect(folderButton()).toHaveAttribute('aria-expanded', 'false')
    expect(fileUrls(f)).toEqual([])
    fireEvent.click(folderButton())
    expect(folderButton()).toHaveAttribute('aria-expanded', 'true')
    await screen.findByText('score.html')
    expect(fileUrls(f)).toEqual(['/api/kit/report-files?workspace=acme&repo_no=0&folder=reports'])
    expect(f.calls.find((c) => c.url.startsWith('/api/kit/report-files')).init.headers).toEqual({ 'X-Dashboard-Token': 'secret-token' })
    fireEvent.click(folderButton())
    expect(folderButton()).toHaveAttribute('aria-expanded', 'false')
    fireEvent.click(folderButton())
    expect(folderButton()).toHaveAttribute('aria-expanded', 'true')
    expect(fileUrls(f)).toHaveLength(1)
  })

  it('AC1: the request names the repo by repo_no from the entry', async () => {
    const entry = { name: 'acme', folders: [{ repo: 'app', repo_no: 2, folder: 'reports', files: 1, files_capped: false, easy_ui_sessions: 0, newest_utc: null }] }
    view(ready(), { reports: { status: 'ready', data: { workspaces: [entry] } } })
    fireEvent.click(folderButton(/^app\/reports/))
    await screen.findByText('score.html')
    expect(fileUrls(f)).toEqual(['/api/kit/report-files?workspace=acme&repo_no=2&folder=reports'])
  })

  it('AC2: one row per file with path, size and date; openable is a button, the rest is text with a title', async () => {
    await openFolder()
    const list = within(row('acme')).getByRole('list', { name: 'Files in app/reports' })
    expect(within(list).getAllByRole('listitem').map((li) => li.textContent)).toEqual(['score.html12 KB · 2026-11-05', 'sub/notes.md300 B · 2026-11-04', 'data.bin2 MB · –'])
    expect(within(list).getByRole('button', { name: 'score.html' })).toBeInTheDocument()
    expect(within(list).getByRole('button', { name: 'sub/notes.md' })).toBeInTheDocument()
    expect(within(list).queryByRole('button', { name: 'data.bin' })).toBeNull()
    expect(screen.getByText('data.bin')).toHaveAttribute('title', 'this file type is not opened here')
  })

  it('AC2: a null mtime_utc is a dash, never "Invalid Date"', async () => {
    await openFolder()
    expect(row('acme')).not.toHaveTextContent(/Invalid Date|NaN|null/)
    expect(screen.getByText('2 MB · –')).toBeInTheDocument()
  })

  it('AC3: shows "loading files" without a Loading pill while pending', async () => {
    vi.stubGlobal('fetch', vi.fn(() => new Promise(() => {})))
    view(ready(), { reports: ready2 })
    fireEvent.click(folderButton())
    expect(screen.getByText('loading files')).toBeInTheDocument()
    expect(screen.queryByRole('status')).toBeNull()
  })

  it('AC3: HTTP 500 shows "files unavailable"; an empty answer "no files"; truncated "first 200 files shown"', async () => {
    vi.stubGlobal('fetch', fakeFetch({ kitReportFiles: 500 }))
    view(ready(), { reports: ready2 })
    fireEvent.click(folderButton())
    expect(await screen.findByText('files unavailable')).toBeInTheDocument()
    expect(screen.queryByRole('alert')).toBeNull()
  })
  it('AC3: no files', async () => {
    vi.stubGlobal('fetch', fakeFetch({ kitReportFiles: { files: [], truncated: false } }))
    view(ready(), { reports: ready2 })
    fireEvent.click(folderButton())
    expect(await screen.findByText('no files')).toBeInTheDocument()
  })
  it('AC3: truncated', async () => {
    vi.stubGlobal('fetch', fakeFetch({ kitReportFiles: { files: [{ path: 'a.txt', size: 1, mtime_utc: null, openable: true }], truncated: true } }))
    view(ready(), { reports: ready2 })
    fireEvent.click(folderButton())
    expect(await screen.findByText('first 200 files shown')).toBeInTheDocument()
  })

  it('AC4: a click opens a blank tab synchronously, nulls opener, fetches with the header, sets the blob URL, revokes after 60 s', async () => {
    await openFolder()
    const win = tab()
    const open = vi.spyOn(window, 'open').mockReturnValue(win)
    const create = vi.fn(() => 'blob:fake-1')
    const revoke = vi.fn()
    URL.createObjectURL = create
    URL.revokeObjectURL = revoke
    vi.useFakeTimers({ toFake: ['setTimeout'] })
    fireEvent.click(screen.getByRole('button', { name: 'score.html' }))
    expect(open).toHaveBeenCalledTimes(1)
    expect(open).toHaveBeenCalledWith('', '_blank')
    expect(win.opener).toBeNull()
    const call = f.calls.find((c) => c.url.startsWith('/api/kit/report-file?'))
    expect(call.url).toBe('/api/kit/report-file?workspace=acme&repo_no=0&folder=reports&path=score.html')
    expect(call.init.headers).toEqual({ 'X-Dashboard-Token': 'secret-token' })
    await act(async () => {
      await vi.advanceTimersByTimeAsync(0)
    })
    expect(create).toHaveBeenCalledTimes(1)
    const blob = create.mock.calls[0][0]
    expect(blob.type).toBe('text/html; charset=utf-8')
    expect(win.location).toBe('blob:fake-1')
    expect(revoke).not.toHaveBeenCalled()
    vi.advanceTimersByTime(59999)
    expect(revoke).not.toHaveBeenCalled()
    vi.advanceTimersByTime(1)
    expect(revoke).toHaveBeenCalledWith('blob:fake-1')
    expect(JSON.stringify(f.calls.map((c) => c.url)) + win.location).not.toContain('secret-token')
  })

  it('AC5: a failed file fetch closes the blank tab and says "cannot open this file" beside the row', async () => {
    vi.stubGlobal('fetch', fakeFetch({ kitReportFile: 404 }))
    view(ready(), { reports: ready2 })
    fireEvent.click(folderButton())
    await screen.findByText('score.html')
    const win = tab()
    vi.spyOn(window, 'open').mockReturnValue(win)
    fireEvent.click(screen.getByRole('button', { name: 'score.html' }))
    const note = await screen.findByRole('status')
    expect(note).toHaveTextContent('cannot open this file')
    expect(screen.getByText('score.html').closest('li')).toContainElement(note)
    expect(win.close).toHaveBeenCalledTimes(1)
  })

  it('AC5: a blocked pop-up shows "allow pop-ups to open the file" and makes no fetch', async () => {
    await openFolder()
    vi.spyOn(window, 'open').mockReturnValue(null)
    fireEvent.click(screen.getByRole('button', { name: 'score.html' }))
    expect(screen.getByRole('status')).toHaveTextContent('allow pop-ups to open the file')
    expect(f.calls.some((c) => c.url.startsWith('/api/kit/report-file?'))).toBe(false)
  })

  it('edge: two clicks on one file open two tabs', async () => {
    await openFolder()
    const open = vi.spyOn(window, 'open').mockImplementation(() => tab())
    URL.createObjectURL = vi.fn(() => 'blob:x')
    URL.revokeObjectURL = vi.fn()
    fireEvent.click(screen.getByRole('button', { name: 'score.html' }))
    fireEvent.click(screen.getByRole('button', { name: 'score.html' }))
    expect(open).toHaveBeenCalledTimes(2)
    await waitFor(() => expect(URL.createObjectURL).toHaveBeenCalledTimes(2))
  })

  it('AC6: markup and special characters render as text and reach the request encoded', async () => {
    const odd = '<img src=x onerror="window.__pwned3=1">&#? a/b c.html'
    const entry = { name: 'a&b #1', folders: [{ repo: 'r', repo_no: 1, folder: 'rep orts?&', files: 1, files_capped: false, easy_ui_sessions: 0, newest_utc: null }] }
    f = fakeFetch({ kitReportFiles: { files: [{ path: odd, size: 1, mtime_utc: null, openable: true }], truncated: false } })
    vi.stubGlobal('fetch', f)
    const { container } = view(ready(withWorkspaces(one({ name: 'a&b #1' }))), { reports: { status: 'ready', data: { workspaces: [entry] } } })
    fireEvent.click(screen.getByRole('button', { name: /^r\/rep orts/ }))
    expect(await screen.findByText(odd)).toBeInTheDocument()
    expect(container.querySelector('img')).toBeNull()
    expect(window.__pwned3).toBeUndefined()
    expect(fileUrls(f)).toEqual([`/api/kit/report-files?${new URLSearchParams({ workspace: 'a&b #1', repo_no: '1', folder: 'rep orts?&' })}`])
    expect(fileUrls(f)[0]).toBe('/api/kit/report-files?workspace=a%26b+%231&repo_no=1&folder=rep+orts%3F%26')
    vi.spyOn(window, 'open').mockReturnValue(tab())
    URL.createObjectURL = vi.fn(() => 'blob:y')
    fireEvent.click(screen.getByRole('button', { name: odd }))
    const url = f.calls.find((c) => c.url.startsWith('/api/kit/report-file?')).url
    expect(new URL(url, 'http://x').searchParams.get('path')).toBe(odd)
    expect(url).not.toMatch(/[<> "#]/)
    expect(screen.getByText(odd)).toHaveAttribute('title', odd)
  })

  it('AC7: the keyboard-reachable controls are native buttons; the focus ring and styles use tokens only', async () => {
    await openFolder()
    for (const b of [folderButton(), screen.getByRole('button', { name: 'score.html' })]) expect(b.tagName).toBe('BUTTON')
    expect(folderButton()).toHaveAttribute('type', 'button')
    const css = readFileSync(resolve(process.cwd(), 'src/app.css'), 'utf8')
    expect(css).toMatch(/:focus-visible \{ outline: 2px solid var\(--color-accent\)/)
    const block = css.split('\n').filter((l) => /\.ws-(report|file|files|chevron)/.test(l)).join('\n')
    expect(block).not.toMatch(/#[0-9a-fA-F]{3,8}\b|rgba?\(/)
  })

  it('edge: closing the folder while its list loads does not update after unmount', async () => {
    let release
    vi.stubGlobal('fetch', vi.fn(() => new Promise((r) => (release = () => r({ ok: true, status: 200, json: async () => ({ files: [], truncated: false }) })))))
    const errors = vi.spyOn(console, 'error').mockImplementation(() => {})
    const { unmount } = view(ready(), { reports: ready2 })
    fireEvent.click(folderButton())
    unmount()
    release()
    await Promise.resolve()
    expect(errors).not.toHaveBeenCalled()
  })
})
