import { fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import LiveView from './LiveView.jsx'
import { FakeEventSource, liveRow, snapshot, stream } from './test/liveStream.js'
import { FakeResizeObserver, FakeSocket, Terminal, resetFakes } from './test/fakeXterm.js'
import { appServer, appSession, termSession } from './test/appSessions.js'
import { FakeChannel, resetChannels } from './test/fakeChannel.js'

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
  resetChannels()
  vi.stubGlobal('BroadcastChannel', FakeChannel)
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

describe('Changes of a terminal session (T137, FR-141)', () => {
  const openTerm = async (term, props) => {
    install({ terms: [termSession({ id: 't1', repo: '/work/company', ...term })], sessions: [] })
    mount([], props)
    await waitFor(() => expect(rowOf('term:t1')).not.toBeNull())
    fireEvent.click(rowOf('term:t1'))
    return within(await screen.findByRole('region', { name: 'Terminal session in company' }))
  }

  it('asks for the changes of the session folder, named by the panel title', async () => {
    const onOpenChanges = vi.fn()
    const panel = await openTerm({ session_id: 'abcdef12-3456' }, { onOpenChanges })
    fireEvent.click(await panel.findByRole('button', { name: 'Changes' }))
    expect(onOpenChanges).toHaveBeenCalledWith({ session: 'abcdef12-3456', title: 'company · t1' })
  })

  it('has no button before the session has its Claude id', async () => {
    const panel = await openTerm({ session_id: null }, { onOpenChanges: vi.fn() })
    expect(panel.queryByRole('button', { name: 'Changes' })).toBeNull()
  })
})

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

describe('room and Pop out (T135)', () => {
  it('a terminal panel takes the whole width: the split is marked (the list and the rail step aside)', async () => {
    install({ terms: [termSession({ id: 't1' })] })
    mount()
    await waitFor(() => expect(rowOf('term:t1')).not.toBeNull())
    fireEvent.click(rowOf('term:t1'))
    await screen.findByRole('region', { name: /^Terminal session in / })
    expect(document.querySelector('.live-split').dataset.term).toBe('true')
  })

  it('a Chat panel keeps the split: the list beside it, no terminal mark', async () => {
    install({ terms: [termSession({ id: 't1' })], sessions: [appSession({ id: 'a1', session_id: null })] })
    mount()
    await waitFor(() => expect(rowOf('app:a1')).not.toBeNull())
    fireEvent.click(rowOf('app:a1'))
    await screen.findByRole('region', { name: /^Session in / })
    expect(document.querySelector('.live-split').dataset.open).toBe('true')
    expect(document.querySelector('.live-split').dataset.term).toBeUndefined()
  })

  it('Pop out closes the panel in the app and opens the window; the row stays listed', async () => {
    install({ terms: [termSession({ id: 't1' })] })
    const open = vi.fn(() => ({ opener: {} }))
    vi.stubGlobal('open', open)
    mount()
    await waitFor(() => expect(rowOf('term:t1')).not.toBeNull())
    fireEvent.click(rowOf('term:t1'))
    const panel = await screen.findByRole('region', { name: /^Terminal session in / })
    fireEvent.click(within(panel).getByRole('button', { name: 'Pop out' }))
    expect(open).toHaveBeenCalledWith('/?term=t1#secret-token', '_blank')
    await waitFor(() => expect(screen.queryByRole('region', { name: /^Terminal session in / })).toBeNull())
    expect(document.querySelector('.live-split').dataset.term).toBeUndefined()
    expect(rowOf('term:t1')).not.toBeNull()
  })
})

describe('terminals in their own window (T135, owner 2026-11-23)', () => {
  it('asks which windows are open; a row whose terminal is open in its own window says so until that window closes', async () => {
    install({ terms: [termSession({ id: 't1' }), termSession({ id: 't2', repo: '/work/other' })] })
    mount()
    await waitFor(() => expect(rowOf('term:t2')).not.toBeNull())
    expect(FakeChannel.sent).toContainEqual({ t: 'who' })
    FakeChannel.from({ t: 'open', id: 't1' })
    await waitFor(() => expect(rowOf('term:t1')).toHaveTextContent('in its own window'))
    expect(rowOf('term:t2')).not.toHaveTextContent('in its own window')
    FakeChannel.from({ t: 'closed', id: 't1' })
    await waitFor(() => expect(rowOf('term:t1')).not.toHaveTextContent('in its own window'))
  })

  it('Dock back from a window opens that terminal in the panel here', async () => {
    install({ terms: [termSession({ id: 't1', repo: '/work/company' })] })
    mount()
    await waitFor(() => expect(rowOf('term:t1')).not.toBeNull())
    FakeChannel.from({ t: 'dock', id: 't1' })
    expect(await screen.findByRole('region', { name: 'Terminal session in company' })).toBeInTheDocument()
    expect(FakeChannel.sent).toContainEqual({ t: 'docked', id: 't1' }) // the window may close now
  })

  it('Dock back for a terminal this page does not list gets no answer and opens nothing', async () => {
    install({ terms: [termSession({ id: 't1' })] })
    mount()
    await waitFor(() => expect(rowOf('term:t1')).not.toBeNull())
    FakeChannel.from({ t: 'dock', id: 'zz' })
    expect(FakeChannel.sent).not.toContainEqual({ t: 'docked', id: 'zz' })
    expect(screen.queryByRole('region', { name: /^Terminal session in / })).toBeNull()
  })

  it('when one of two windows of a terminal closes, Live asks again so the other keeps the mark', async () => {
    install({ terms: [termSession({ id: 't1' })] })
    mount()
    await waitFor(() => expect(rowOf('term:t1')).not.toBeNull())
    const second = new FakeChannel('easy-personal-term') // the window still open
    second.onmessage = ({ data }) => data.t === 'who' && second.postMessage({ t: 'open', id: 't1' })
    FakeChannel.from({ t: 'open', id: 't1' })
    FakeChannel.from({ t: 'closed', id: 't1' })
    await waitFor(() => expect(rowOf('term:t1')).toHaveTextContent('in its own window'))
  })

  it('a message of another shape is ignored', async () => {
    install({ terms: [termSession({ id: 't1' })] })
    mount()
    await waitFor(() => expect(rowOf('term:t1')).not.toBeNull())
    FakeChannel.from({ t: 'open', id: 7 })
    FakeChannel.from({ t: 'dock' })
    FakeChannel.from('open')
    expect(rowOf('term:t1')).not.toHaveTextContent('in its own window')
    expect(screen.queryByRole('region', { name: /^Terminal session in / })).toBeNull()
  })
})
