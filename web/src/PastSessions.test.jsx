import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import LiveView from './LiveView.jsx'
import { ALL_TIME_FROM, WINDOW_NOTE } from './PastSessions.jsx'
import { appServer } from './test/appSessions.js'
import { FakeEventSource, entryAssistant, entryUser, liveRow, page, pastServer, snapshot, stream } from './test/liveStream.js'

// The Past switch of the Live view (T119, FR-126, FR-128): past sessions from the store with the filters that fit, a date range, Load more, the 30-day
// window and Show older, the read-only conversation, Resume / Fork. The routes are mocked as the T117 guide writes them; nothing leaves the test.
const FILTERS = 'easy-personal-live-filters'
const WINDOW = { from: '2026-10-20T00:00:00.000000Z', to: null, default: true }
const past = (id, over = {}) => ({ id, project: 'alpha', branch: 'main', first: '2026-11-18T07:00:00.000000Z', last: '2026-11-18T08:00:00.000000Z', kind: 'terminal', account: 'Claude work', model: 'sonnet', name: null, ...over })
const answer = (sessions, over = {}) => ({ sessions, next: null, window: WINDOW, ...over })
const A = 'aaaaaaaa-1111-2222'
const B = 'bbbbbbbb-3333-4444'

beforeEach(() => {
  window.location.hash = '#secret-token'
  window.localStorage.clear()
  FakeEventSource.instances = []
  vi.stubGlobal('EventSource', FakeEventSource)
})
afterEach(() => {
  vi.useRealTimers()
  vi.unstubAllGlobals()
})

/** Render the Live view and switch to Past. `server` is a `pastServer`; `props` go to LiveView. */
async function openPast(server, props = {}, live = []) {
  vi.stubGlobal('fetch', server)
  render(<LiveView {...props} />)
  stream().emit('snapshot', snapshot(live))
  fireEvent.click(screen.getByRole('button', { name: 'Past' }))
  if (!vi.isFakeTimers()) await waitFor(() => expect(screen.queryByText('Loading past sessions')).toBeNull())
}
const rowEl = (id) => document.getElementById(`past-row-${id}`)
const shown = () => [...document.querySelectorAll('.past .live-row .live-id')].map((e) => e.textContent)
const lastParams = (server) => server.pasts().at(-1).params
const panel = () => screen.getByRole('region', { name: /^Past session in / })

describe('the Past switch (AC 3)', () => {
  it('lists the sessions of the store, newest first as the server gives them, with the token in the header only', async () => {
    const server = pastServer({ past: () => answer([past(A, { name: 'my review' }), past(B, { project: 'beta', kind: 'headless' })]) })
    await openPast(server)
    expect(shown()).toEqual(['aaaaaaaa', 'bbbbbbbb'])
    expect(within(rowEl(A)).getByText('my review')).toBeInTheDocument()
    expect(within(rowEl(B)).getByText('beta')).toBeInTheDocument()
    expect(rowEl(B)).toHaveTextContent('headless')
    const [first] = server.pasts()
    expect(first.url).toBe('/api/sessions/past')
    expect(first.init.headers['X-Dashboard-Token']).toBe('secret-token')
  })

  it('Live and Past switch the page: Past hides the live list and Live brings it back', async () => {
    await openPast(pastServer({ past: () => answer([past(A)]) }), {}, [liveRow('cccccccc-5')])
    expect(screen.queryByRole('group', { name: 'Filter sessions' })).toBeNull()
    expect(screen.getByRole('button', { name: 'Past' })).toHaveAttribute('aria-pressed', 'true')
    fireEvent.click(screen.getByRole('button', { name: 'Live' }))
    expect(screen.getByRole('group', { name: 'Filter sessions' })).toBeInTheDocument()
    expect(screen.queryByRole('region', { name: 'Past sessions' })).toBeNull()
  })

  it('says so when no past session matches, and when the route is not there (the Python server)', async () => {
    await openPast(pastServer({ past: () => answer([]) }))
    expect(screen.getByText('No past sessions match.')).toBeInTheDocument()
  })

  it('shows the server\'s one line for a refused filter', async () => {
    await openPast(pastServer({ past: () => ({ status: 400, body: { error: 'kind must be one of terminal, headless' } }) }))
    expect(await screen.findByRole('alert')).toHaveTextContent('kind must be one of terminal, headless')
  })

  it('a server without the route says the list cannot be read', async () => {
    await openPast(pastServer({ past: () => ({ status: 404, body: {} }) }))
    expect(await screen.findByRole('alert')).toHaveTextContent(/cannot be listed/)
  })

  it('an answer of the wrong shape says so and lists nothing', async () => {
    await openPast(pastServer({ past: () => ({ nothing: 1 }) }))
    expect(await screen.findByRole('alert')).toHaveTextContent('shape this page cannot read')
    expect(shown()).toEqual([])
  })

  it('an entry without a usable id is dropped; text is shown as text', async () => {
    await openPast(pastServer({ past: () => answer([past(A, { name: '<b>bold</b>' }), { project: 'x' }, null, past('', {})]) }))
    expect(shown()).toEqual(['aaaaaaaa'])
    expect(within(rowEl(A)).getByText('<b>bold</b>')).toBeInTheDocument()
  })
})

describe('the 30-day window and Show older (AC 3)', () => {
  it('says "Showing the last 30 days." while the server\'s window is the default, with a Show older control', async () => {
    const server = pastServer({ past: () => answer([past(A)]) })
    await openPast(server)
    expect(screen.getByText(WINDOW_NOTE, { exact: false })).toBeInTheDocument()
    expect(WINDOW_NOTE).toBe('Showing the last 30 days.')
    expect(lastParams(server).has('from')).toBe(false)
  })

  it('Show older sends an explicit from and the note goes; Last 30 days takes it back', async () => {
    const server = pastServer({ past: (p) => answer([past(A)], { window: p.has('from') ? { from: '2000-01-01', to: null, default: false } : WINDOW }) })
    await openPast(server)
    fireEvent.click(screen.getByRole('button', { name: 'Show older' }))
    await waitFor(() => expect(lastParams(server).get('from')).toBe(ALL_TIME_FROM))
    await waitFor(() => expect(screen.queryByText(WINDOW_NOTE, { exact: false })).toBeNull())
    expect(screen.queryByRole('button', { name: 'Show older' })).toBeNull()
    fireEvent.click(screen.getByRole('button', { name: 'Last 30 days' }))
    await waitFor(() => expect(lastParams(server).has('from')).toBe(false))
    expect(await screen.findByRole('button', { name: 'Show older' })).toBeInTheDocument()
  })

  it('no note when the window is not the default', async () => {
    await openPast(pastServer({ past: () => answer([past(A)], { window: { from: '2026-01-01', to: null, default: false } }) }))
    expect(screen.queryByText(WINDOW_NOTE, { exact: false })).toBeNull()
  })

  it('with only a To date the note names that day, and Show older still sends a from', async () => {
    const server = pastServer({ past: () => answer([past(A)]) })
    await openPast(server)
    fireEvent.change(screen.getByLabelText('To'), { target: { value: '2026-11-10' } })
    expect(await screen.findByText('Showing the 30 days up to and including 2026-11-10.', { exact: false })).toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: 'Show older' }))
    await waitFor(() => expect(lastParams(server).get('from')).toBe(ALL_TIME_FROM))
  })
})

describe('date range (AC 3)', () => {
  it('From is sent as picked; To is sent as the next day, so the picked day is in', async () => {
    const server = pastServer({ past: () => answer([past(A)]) })
    await openPast(server)
    fireEvent.change(screen.getByLabelText('From'), { target: { value: '2026-11-01' } })
    await waitFor(() => expect(lastParams(server).get('from')).toBe('2026-11-01'))
    fireEvent.change(screen.getByLabelText('To'), { target: { value: '2026-11-30' } })
    await waitFor(() => expect(lastParams(server).get('to')).toBe('2026-12-01'))
  })

  it('a From after the To is said at once and asks the server nothing', async () => {
    const server = pastServer({ past: () => answer([past(A)]) })
    await openPast(server)
    fireEvent.change(screen.getByLabelText('From'), { target: { value: '2026-11-20' } })
    await waitFor(() => expect(server.pasts().length).toBe(2))
    const asked = server.pasts().length
    fireEvent.change(screen.getByLabelText('To'), { target: { value: '2026-11-10' } })
    expect(await screen.findByRole('alert')).toHaveTextContent('From is after To')
    expect(server.pasts().length).toBe(asked)
  })
})

describe('the filters (AC 1, AC 3)', () => {
  it('Kind asks the server for that kind; the project box asks after a pause; both are the Live list\'s filters, kept in localStorage', async () => {
    const server = pastServer({ past: () => answer([past(A)]) })
    await openPast(server)
    vi.useFakeTimers({ shouldAdvanceTime: true })
    fireEvent.change(screen.getByLabelText('Kind'), { target: { value: 'headless' } })
    await waitFor(() => expect(lastParams(server).get('kind')).toBe('headless'))
    const before = server.pasts().length
    fireEvent.change(screen.getByLabelText('Project'), { target: { value: 'alp' } })
    fireEvent.change(screen.getByLabelText('Project'), { target: { value: 'alpha' } })
    expect(server.pasts().length).toBe(before) // not asked per key
    await act(async () => {
      await vi.advanceTimersByTimeAsync(400)
    })
    await waitFor(() => expect(lastParams(server).get('project')).toBe('alpha'))
    expect(JSON.parse(window.localStorage.getItem(FILTERS))).toEqual({ state: 'all', project: 'alpha', kind: 'headless' })
  })

  it('starts with the stored project; a stored kind the store does not know (app) asks for no kind, is shown as such, and All kinds clears it', async () => {
    window.localStorage.setItem(FILTERS, JSON.stringify({ state: 'working', project: 'beta', kind: 'app' }))
    const server = pastServer({ past: () => answer([]) })
    await openPast(server)
    expect(lastParams(server).get('project')).toBe('beta')
    expect(lastParams(server).has('kind')).toBe(false)
    expect(screen.getByLabelText('Project')).toHaveValue('beta')
    expect(screen.getByLabelText('Kind')).toHaveValue('app') // the select says what is stored, so All kinds is a change
    fireEvent.change(screen.getByLabelText('Kind'), { target: { value: '' } })
    expect(screen.getByLabelText('Kind')).toHaveValue('')
    expect(JSON.parse(window.localStorage.getItem(FILTERS)).kind).toBe('')
    expect(within(screen.getByLabelText('Kind')).queryByRole('option', { name: /App/ })).toBeNull()
  })

  it('a state filter is a live thing: Past has none', async () => {
    await openPast(pastServer({ past: () => answer([]) }))
    expect(screen.queryByRole('button', { name: /^Needs me/ })).toBeNull()
  })

  it('Clear filters empties the project, kind and dates and asks again', async () => {
    const server = pastServer({ past: () => answer([past(A)]) })
    await openPast(server)
    fireEvent.change(screen.getByLabelText('Kind'), { target: { value: 'terminal' } })
    fireEvent.change(screen.getByLabelText('From'), { target: { value: '2026-11-01' } })
    await waitFor(() => expect(lastParams(server).get('kind')).toBe('terminal'))
    fireEvent.click(screen.getByRole('button', { name: 'Clear filters' }))
    await waitFor(() => expect([...lastParams(server).keys()]).toEqual([]))
    expect(screen.getByLabelText('From')).toHaveValue('')
    expect(screen.queryByRole('button', { name: 'Clear filters' })).toBeNull()
  })

  it('an answer that belongs to an older filter is dropped', async () => {
    const held = []
    const server = pastServer({
      past: (p) => (p.get('kind') === 'terminal' ? new Promise((resolve) => held.push(resolve)) : answer([past(p.get('kind') === 'headless' ? B : A)])),
    })
    await openPast(server)
    fireEvent.change(screen.getByLabelText('Kind'), { target: { value: 'terminal' } })
    await waitFor(() => expect(held).toHaveLength(1))
    fireEvent.change(screen.getByLabelText('Kind'), { target: { value: 'headless' } })
    await waitFor(() => expect(shown()).toEqual(['bbbbbbbb']))
    await act(async () => held[0](answer([past('cccccccc-9')])))
    expect(shown()).toEqual(['bbbbbbbb'])
  })
})

describe('Load more (AC 3)', () => {
  it('is offered while the page says there is a next one, sends it as before, and appends', async () => {
    const server = pastServer({ past: (p) => (p.get('before') === 'cursor-1' ? answer([past(B)]) : answer([past(A)], { next: 'cursor-1' })) })
    await openPast(server)
    fireEvent.click(screen.getByRole('button', { name: 'Load more' }))
    await waitFor(() => expect(shown()).toEqual(['aaaaaaaa', 'bbbbbbbb']))
    expect(lastParams(server).get('before')).toBe('cursor-1')
    expect(screen.queryByRole('button', { name: 'Load more' })).toBeNull()
  })

  it('keeps the filters on the next page and does not list a session twice', async () => {
    const server = pastServer({ past: (p) => (p.get('before') ? answer([past(A), past(B)]) : answer([past(A)], { next: 'c' })) })
    await openPast(server)
    fireEvent.change(screen.getByLabelText('Kind'), { target: { value: 'terminal' } })
    await waitFor(() => expect(lastParams(server).get('kind')).toBe('terminal'))
    await waitFor(() => expect(screen.getByRole('button', { name: 'Load more' })).toBeEnabled())
    fireEvent.click(screen.getByRole('button', { name: 'Load more' }))
    await waitFor(() => expect(shown()).toEqual(['aaaaaaaa', 'bbbbbbbb']))
    expect(lastParams(server).get('kind')).toBe('terminal')
  })

  it('a failed page keeps the list, says so, and can be tried again', async () => {
    let fail = true
    const server = pastServer({ past: (p) => (p.get('before') ? (fail ? { status: 500, body: {} } : answer([past(B)])) : answer([past(A)], { next: 'c' })) })
    await openPast(server)
    fireEvent.click(screen.getByRole('button', { name: 'Load more' }))
    expect(await screen.findByRole('alert')).toHaveTextContent('Could not load more')
    expect(shown()).toEqual(['aaaaaaaa'])
    fail = false
    fireEvent.click(screen.getByRole('button', { name: 'Load more' }))
    await waitFor(() => expect(shown()).toEqual(['aaaaaaaa', 'bbbbbbbb']))
  })

  it('one request at a time: Load more is off while one is out', async () => {
    let release
    const server = pastServer({ past: (p) => (p.get('before') ? new Promise((r) => (release = r)) : answer([past(A)], { next: 'c' })) })
    await openPast(server)
    fireEvent.click(screen.getByRole('button', { name: 'Load more' }))
    await waitFor(() => expect(screen.getByRole('button', { name: 'Load more' })).toBeDisabled())
    fireEvent.click(screen.getByRole('button', { name: 'Load more' }))
    expect(server.pasts().filter((c) => c.params.has('before'))).toHaveLength(1)
    await act(async () => release(answer([past(B)])))
    await waitFor(() => expect(shown()).toEqual(['aaaaaaaa', 'bbbbbbbb']))
  })
})

describe('a past session: read-only conversation, Resume, Fork (AC 3)', () => {
  const server = () => pastServer({ past: () => answer([past(A, { name: 'my review' })]), pages: [page([entryUser('what did we do'), entryAssistant('we fixed it')])] })

  it('opens the conversation from the transcript route, with no composer, no Stop and no controls of a running session', async () => {
    const s = server()
    await openPast(s, { appSessions: true })
    fireEvent.click(rowEl(A))
    expect(within(panel()).getByRole('heading', { name: 'my review' })).toBeInTheDocument()
    expect(await within(panel()).findByText('what did we do')).toBeInTheDocument()
    expect(within(panel()).getByText('we fixed it')).toBeInTheDocument()
    expect(s.reads()[0].url).toBe(`/api/live/sessions/${A}/transcript`)
    expect(within(panel()).getByText('Past session: read-only')).toBeInTheDocument()
    for (const name of ['Stop', 'Interrupt', 'Close']) expect(within(panel()).queryByRole('button', { name })).toBeNull()
    expect(within(panel()).queryByRole('textbox')).toBeNull()
    for (const [dt, dd] of [['Project', 'alpha'], ['Kind', 'terminal'], ['Session', A], ['Account', 'Claude work'], ['Branch', 'main'], ['Model', 'sonnet']]) expect(within(panel()).getByText(dt, { selector: 'dt' }).nextSibling).toHaveTextContent(dd)
  })

  it('a conversation that cannot be read says so with the server\'s line', async () => {
    const s = pastServer({ past: () => answer([past(A)]), transcriptStatus: { status: 404, body: { error: 'the session file is gone' } } })
    await openPast(s)
    fireEvent.click(rowEl(A))
    expect(await within(panel()).findByText(/the session file is gone/)).toBeInTheDocument()
  })

  it('Resume and Fork start an app session in the app\'s own flow, from the list\'s own facts', async () => {
    const onResumeInApp = vi.fn()
    const s = server()
    const apps = appServer({ sessions: [] }) // the app session list answers (empty): the server has app sessions
    vi.stubGlobal('fetch', (url, init) => (String(url).startsWith('/api/app-sessions') ? apps(url, init) : s(url, init)))
    render(<LiveView appSessions onResumeInApp={onResumeInApp} />)
    stream().emit('snapshot', snapshot([]))
    fireEvent.click(screen.getByRole('button', { name: 'Past' }))
    await waitFor(() => expect(rowEl(A)).not.toBeNull())
    await waitFor(() => expect(within(document.querySelector('.past')).queryByText('Loading past sessions')).toBeNull())
    fireEvent.click(rowEl(A))
    fireEvent.click(within(panel()).getByRole('button', { name: 'Resume in app' }))
    expect(onResumeInApp).toHaveBeenLastCalledWith({ id: A, project: 'alpha', account: 'Claude work', fork: false })
    fireEvent.click(within(panel()).getByRole('button', { name: 'Fork in app' }))
    expect(onResumeInApp).toHaveBeenLastCalledWith({ id: A, project: 'alpha', account: 'Claude work', fork: true })
  })

  it('a server with no app sessions offers neither (there is nothing to start one with)', async () => {
    await openPast(server(), { appSessions: true })
    fireEvent.click(rowEl(A))
    expect(within(panel()).queryByRole('button', { name: /in app$/ })).toBeNull()
  })

  it('Esc and Back close the panel and the row has the focus again', async () => {
    await openPast(server())
    fireEvent.click(rowEl(A))
    fireEvent.keyDown(panel(), { key: 'Escape' })
    expect(screen.queryByRole('region', { name: /^Past session in / })).toBeNull()
    expect(rowEl(A)).toHaveFocus()
    fireEvent.click(rowEl(A))
    fireEvent.click(within(panel()).getByRole('button', { name: 'Back to sessions' }))
    expect(screen.queryByRole('region', { name: /^Past session in / })).toBeNull()
  })

  it('the panel closes when a new filter drops the session from the list', async () => {
    const s = pastServer({ past: (p) => answer(p.get('kind') === 'headless' ? [] : [past(A)]) })
    await openPast(s)
    fireEvent.click(rowEl(A))
    fireEvent.change(screen.getByLabelText('Kind'), { target: { value: 'headless' } })
    await waitFor(() => expect(screen.queryByRole('region', { name: /^Past session in / })).toBeNull())
  })

  it('without a name the heading is the project and the short id', async () => {
    await openPast(pastServer({ past: () => answer([past(A)]) }))
    fireEvent.click(rowEl(A))
    expect(within(panel()).getByRole('heading', { name: 'alpha aaaaaaaa' })).toBeInTheDocument()
  })
})

describe('rename in Past (AC 2)', () => {
  it('Enter saves through the route and the row shows the new name; empty removes it', async () => {
    const s = pastServer({ past: () => answer([past(A)]) })
    await openPast(s)
    fireEvent.click(rowEl(A))
    fireEvent.click(within(panel()).getByRole('button', { name: 'Rename' }))
    const field = within(panel()).getByLabelText('Name')
    fireEvent.change(field, { target: { value: 'yesterday' } })
    fireEvent.keyDown(field, { key: 'Enter' })
    await waitFor(() => expect(within(rowEl(A)).getByText('yesterday')).toBeInTheDocument())
    expect(s.renames()[0].path).toBe(`/api/sessions/${A}/name`)
    expect(s.renames()[0].body).toEqual({ name: 'yesterday', confirmed: true })
    fireEvent.click(within(panel()).getByRole('button', { name: 'Rename' }))
    fireEvent.change(within(panel()).getByLabelText('Name'), { target: { value: '' } })
    fireEvent.keyDown(within(panel()).getByLabelText('Name'), { key: 'Enter' })
    await waitFor(() => expect(within(rowEl(A)).getByText('alpha')).toBeInTheDocument())
    expect(s.renames()[1].body).toEqual({ name: null, confirmed: true })
  })
})

describe('a filter change with a past session open (Stage 4 review)', () => {
  const open = async (over = {}) => {
    const s = pastServer({ past: (p) => answer(p.get('kind') === 'terminal' ? [past(B)] : [past(A, { name: 'my review' }), past(B)]), pages: [page([entryUser('hello')])], ...over })
    await openPast(s)
    fireEvent.click(rowEl(A))
    await within(panel()).findByText('hello')
    return s
  }

  it('keeps the panel mounted, the focus where the owner is, and reads the conversation once', async () => {
    const s = await open()
    const node = panel()
    const reads = s.reads().length
    const select = screen.getByLabelText('Kind')
    select.focus()
    fireEvent.change(select, { target: { value: 'headless' } })
    await waitFor(() => expect(lastParams(s).get('kind')).toBe('headless'))
    await waitFor(() => expect(screen.queryByText('Loading past sessions')).toBeNull())
    expect(panel()).toBe(node)
    expect(select).toHaveFocus()
    expect(s.reads().length).toBe(reads)
  })

  it('keeps an open rename field through a filter change', async () => {
    const s = await open()
    fireEvent.click(within(panel()).getByRole('button', { name: 'Rename' }))
    fireEvent.change(within(panel()).getByLabelText('Name'), { target: { value: 'half typed' } })
    fireEvent.change(screen.getByLabelText('Kind'), { target: { value: 'headless' } })
    await waitFor(() => expect(lastParams(s).get('kind')).toBe('headless'))
    await waitFor(() => expect(screen.queryByText('Loading past sessions')).toBeNull())
    expect(within(panel()).getByLabelText('Name')).toHaveValue('half typed')
  })

  it('keeps the panel and the rows while the next list is loading; the panel closes only when a ready list lacks the session', async () => {
    let release
    await open({ past: (p) => (p.get('kind') === 'terminal' ? new Promise((r) => (release = r)) : answer([past(A), past(B)])) })
    fireEvent.change(screen.getByLabelText('Kind'), { target: { value: 'terminal' } })
    await waitFor(() => expect(screen.getByText('Loading past sessions')).toBeInTheDocument())
    expect(panel()).toBeInTheDocument()
    expect(shown()).toEqual(['aaaaaaaa', 'bbbbbbbb']) // the rows of the last list stay while the next one loads
    await act(async () => release(answer([past(B)])))
    await waitFor(() => expect(screen.queryByRole('region', { name: /^Past session in / })).toBeNull()) // ready and it is not in the list
    expect(shown()).toEqual(['bbbbbbbb'])
  })

  it('an error on the next list keeps the panel', async () => {
    await open({ past: (p) => (p.get('kind') === 'terminal' ? { status: 500, body: {} } : answer([past(A), past(B)])) })
    fireEvent.change(screen.getByLabelText('Kind'), { target: { value: 'terminal' } })
    expect(await screen.findByRole('alert')).toBeInTheDocument()
    expect(panel()).toBeInTheDocument()
  })

  it('a remount of the panel does not take the focus: only opening a row does', async () => {
    await open()
    expect(within(panel()).getByRole('heading', { level: 2 })).toHaveFocus()
    const box = screen.getByLabelText('Project')
    box.focus()
    fireEvent.change(screen.getByLabelText('Kind'), { target: { value: 'headless' } })
    box.focus()
    await waitFor(() => expect(screen.queryByText('Loading past sessions')).toBeNull())
    expect(box).toHaveFocus()
  })
})
