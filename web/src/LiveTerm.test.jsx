import { fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import LiveView from './LiveView.jsx'
import { FakeEventSource, liveRow, snapshot, stream } from './test/liveStream.js'
import { FakeResizeObserver, FakeSocket, Terminal, resetFakes } from './test/fakeXterm.js'
import { appServer, appSession, termSession } from './test/appSessions.js'

vi.mock('@xterm/xterm', async () => ({ Terminal: (await import('./test/fakeXterm.js')).Terminal }))
vi.mock('@xterm/addon-fit', async () => ({ FitAddon: (await import('./test/fakeXterm.js')).FitAddon }))

// The terminal side of the Live view (T134, FR-136, AC 6): a terminal session the app started is listed like the others and opens the terminal
// panel, not the Chat panel. The list is `GET /api/term-sessions`; a row that the live stream also lists (same Claude session id) is one row.
const install = (opts) => {
  const server = appServer(opts)
  vi.stubGlobal('fetch', server)
  return server
}
beforeEach(() => {
  resetFakes()
  window.location.hash = '#secret-token'
  window.localStorage.clear()
  FakeEventSource.instances = []
  vi.stubGlobal('EventSource', FakeEventSource)
  vi.stubGlobal('WebSocket', FakeSocket)
  vi.stubGlobal('ResizeObserver', FakeResizeObserver)
})
afterEach(() => {
  vi.useRealTimers()
  vi.unstubAllGlobals()
})
const rowOf = (key) => document.getElementById(`live-row-${key}`)
const mount = (rows = [], props = {}) => {
  const view = render(<LiveView appSessions {...props} />)
  stream().emit('snapshot', snapshot(rows))
  return view
}

describe('listing (AC 6)', () => {
  it('lists a terminal session beside the others, with its project and state, from the term list', async () => {
    install({ terms: [termSession({ id: 't1', repo: '/work/company' })], sessions: [appSession({ id: 'a1', session_id: null })] })
    mount()
    await waitFor(() => expect(rowOf('term:t1')).not.toBeNull())
    expect(rowOf('app:a1')).not.toBeNull()
    expect(rowOf('term:t1')).toHaveTextContent('company')
    expect(rowOf('term:t1')).toHaveTextContent('Running')
  })

  it('an exited terminal session stays listed as ended with its code', async () => {
    install({ terms: [termSession({ id: 't1', state: 'exited', code: 2 })] })
    mount()
    await waitFor(() => expect(rowOf('term:t1')).not.toBeNull())
    expect(rowOf('term:t1')).toHaveTextContent('Ended (code 2)')
  })

  it('a server with no terminal route lists none, and the rest of Live is as before', async () => {
    install({ sessions: [appSession({ id: 'a1', session_id: null })] })
    mount([liveRow('sess-live')])
    await waitFor(() => expect(rowOf('app:a1')).not.toBeNull())
    expect(document.querySelector('[id^="live-row-term:"]')).toBeNull()
  })

  it('a live row with the same Claude session id is one row, not two', async () => {
    install({ terms: [termSession({ id: 't1', session_id: 'sess-9' })] })
    mount([liveRow('sess-9', { project: 'company' })])
    await waitFor(() => expect(rowOf('term:t1')).not.toBeNull())
    expect(rowOf('sess-9')).toBeNull()
    expect(document.querySelectorAll('.live-session')).toHaveLength(1)
  })

  it('opening a terminal session shows the terminal panel and attaches: no Chat panel, no transcript read', async () => {
    const server = install({ terms: [termSession({ id: 't1', repo: '/work/company' })] })
    mount()
    await waitFor(() => expect(rowOf('term:t1')).not.toBeNull())
    fireEvent.click(rowOf('term:t1'))
    expect(await screen.findByRole('region', { name: 'Terminal session in company' })).toBeInTheDocument()
    expect(screen.queryByRole('region', { name: /^Session in / })).toBeNull()
    expect(FakeSocket.last.url).toBe('ws://localhost:3000/api/term/t1')
    expect(Terminal.instances).toHaveLength(1)
    expect(server.calls.some((c) => c.path.includes('/transcript'))).toBe(false)
  })

  it('Back closes the panel, drops the socket, and the row stays listed', async () => {
    install({ terms: [termSession({ id: 't1' })] })
    mount()
    await waitFor(() => expect(rowOf('term:t1')).not.toBeNull())
    fireEvent.click(rowOf('term:t1'))
    const panel = await screen.findByRole('region', { name: /^Terminal session in / })
    fireEvent.click(within(panel).getByRole('button', { name: 'Back to sessions' }))
    await waitFor(() => expect(screen.queryByRole('region', { name: /^Terminal session in / })).toBeNull())
    expect(FakeSocket.last.closed).toBe(true)
    expect(rowOf('term:t1')).not.toBeNull()
  })

  it('an app session still opens the Chat panel', async () => {
    install({ terms: [termSession({ id: 't1' })], sessions: [appSession({ id: 'a1', session_id: null })] })
    mount()
    await waitFor(() => expect(rowOf('app:a1')).not.toBeNull())
    fireEvent.click(rowOf('app:a1'))
    expect(await screen.findByRole('region', { name: /^Session in / })).toBeInTheDocument()
    expect(screen.queryByRole('region', { name: /^Terminal session in / })).toBeNull()
  })

  it('Stop in the panel ends it: the exit frame makes the row read as ended after the list is read again', async () => {
    const server = install({ terms: [termSession({ id: 't1' })], routes: { 'POST /api/term-sessions/t1/stop': () => ({}) } })
    mount()
    await waitFor(() => expect(rowOf('term:t1')).not.toBeNull())
    fireEvent.click(rowOf('term:t1'))
    await screen.findByRole('region', { name: /^Terminal session in / })
    FakeSocket.last.open()
    server.state.terms = [termSession({ id: 't1', state: 'exited', code: 0 })]
    FakeSocket.last.receive({ t: 'exit', code: 0 })
    await waitFor(() => expect(rowOf('term:t1')).toHaveTextContent('Ended (code 0)'))
  })
})

describe('starting one (AC 1, AC 6)', () => {
  it('a Terminal start from the New session sheet opens its panel in Live', async () => {
    const server = install({ terms: [], routes: { 'POST /api/term-sessions': () => ({ status: 201, body: { id: 't9' } }) } })
    const onOpenAppSession = vi.fn()
    mount([], { onOpenAppSession })
    fireEvent.click(await screen.findByRole('button', { name: 'New session' }))
    fireEvent.click(await screen.findByRole('button', { name: 'Terminal' }))
    fireEvent.click(screen.getByRole('button', { name: 'Review' }))
    server.state.terms = [termSession({ id: 't9' })] // the list the server holds after the start
    fireEvent.click(screen.getByRole('button', { name: 'Start session' }))
    await waitFor(() => expect(onOpenAppSession).toHaveBeenCalledWith('term:t9'))
    await waitFor(() => expect(rowOf('term:t9')).not.toBeNull()) // listed at once, not at the next read
  })

  it('the page asked to open a started terminal session (wantedApp term:<id>): its panel opens once it is listed', async () => {
    install({ terms: [termSession({ id: 't9' })] })
    const onWantedAppUsed = vi.fn()
    mount([], { wantedApp: 'term:t9', onWantedAppUsed })
    expect(await screen.findByRole('region', { name: /^Terminal session in / })).toBeInTheDocument()
    expect(onWantedAppUsed).toHaveBeenCalled()
    expect(FakeSocket.last.url).toBe('ws://localhost:3000/api/term/t9')
  })
})
