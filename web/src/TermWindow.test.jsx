import { act, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import TermWindow from './TermWindow.jsx'
import { FakeResizeObserver, FakeSocket, resetFakes } from './test/fakeXterm.js'
import { appServer, termSession } from './test/appSessions.js'
import { FakeChannel, resetChannels } from './test/fakeChannel.js'

vi.mock('@xterm/xterm', async () => ({ Terminal: (await import('./test/fakeXterm.js')).Terminal }))
vi.mock('@xterm/addon-fit', async () => ({ FitAddon: (await import('./test/fakeXterm.js')).FitAddon }))

// The pop-out window (T135, AC 5): `/?term=<id>#<token>` shows only that terminal, filling the window.
beforeEach(() => {
  resetFakes()
  resetChannels()
  vi.stubGlobal('BroadcastChannel', FakeChannel)
  window.location.hash = '#secret-token'
  vi.stubGlobal('WebSocket', FakeSocket)
  vi.stubGlobal('ResizeObserver', FakeResizeObserver)
})
afterEach(() => vi.unstubAllGlobals())

describe('the pop-out window (AC 5)', () => {
  it('reads the list, finds the id and shows only its terminal: popped, attached, titled', async () => {
    vi.stubGlobal('fetch', appServer({ terms: [termSession({ id: 'x0' }), termSession({ id: 't1', repo: '/work/company' })] }))
    render(<TermWindow id="t1" />)
    expect(await screen.findByRole('region', { name: 'Terminal session in company' })).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Back to sessions' })).toBeNull()
    expect(screen.queryByRole('button', { name: 'Pop out' })).toBeNull()
    await waitFor(() => expect(FakeSocket.last?.url).toBe('ws://localhost:3000/api/term/t1')) // the panel's effect opens it after the render
    expect(document.title).toBe('company · terminal · Easy Personal')
  })

  it('an id the server does not list says so, with no socket', async () => {
    vi.stubGlobal('fetch', appServer({ terms: [termSession({ id: 'x0' })] }))
    render(<TermWindow id="gone" />)
    expect(await screen.findByText(/not running here any more/)).toBeInTheDocument()
    expect(FakeSocket.instances).toHaveLength(0)
  })

  it('a list that cannot be read says so', async () => {
    vi.stubGlobal('fetch', vi.fn(async () => ({ ok: false, status: 500, json: async () => ({}) })))
    render(<TermWindow id="t1" />)
    await waitFor(() => expect(screen.getByRole('alert')).toHaveTextContent(/Could not read the terminal sessions/))
  })

  it('a refresh that fails after the terminal is shown (Stop, exit) keeps the terminal and its socket', async () => {
    const server = appServer({ terms: [termSession({ id: 't1', repo: '/work/company' })] })
    let fail = false
    let failed = 0
    vi.stubGlobal('fetch', vi.fn(async (...args) => (fail ? (failed++, { ok: false, status: 500, json: async () => ({}) }) : server(...args))))
    render(<TermWindow id="t1" />)
    await screen.findByRole('region', { name: 'Terminal session in company' })
    await waitFor(() => expect(FakeSocket.last).toBeDefined())
    const socket = FakeSocket.last
    socket.open()
    fail = true
    socket.receive({ t: 'exit', code: 0 }) // the panel asks the window to read the list again
    await waitFor(() => expect(failed).toBe(1), { timeout: 5000 })
    await act(async () => {}) // let the failed read settle
    expect(screen.getByRole('region', { name: 'Terminal session in company' })).toHaveTextContent('Ended (code 0)')
    expect(screen.queryByText(/Could not read the terminal sessions/)).toBeNull()
    expect(socket.closed).toBe(false)
  })
})

describe('it belongs to the app (T135, owner 2026-11-23)', () => {
  it('a bar names the app and says closing the window does not stop the session', async () => {
    vi.stubGlobal('fetch', appServer({ terms: [termSession({ id: 't1', repo: '/work/company' })] }))
    render(<TermWindow id="t1" />)
    await screen.findByRole('region', { name: 'Terminal session in company' })
    const bar = document.querySelector('.term-window-bar')
    expect(bar).toHaveTextContent('Easy Personal · Terminal')
    expect(bar).toHaveTextContent('Closing this window does not stop the session.')
  })

  it('tells Live it is open, answers who, and says closed when it goes', async () => {
    vi.stubGlobal('fetch', appServer({ terms: [termSession({ id: 't1' })] }))
    const view = render(<TermWindow id="t1" />)
    expect(FakeChannel.sent).toContainEqual({ t: 'open', id: 't1' })
    FakeChannel.sent = []
    FakeChannel.from({ t: 'who' })
    expect(FakeChannel.sent).toContainEqual({ t: 'open', id: 't1' })
    view.unmount()
    expect(FakeChannel.sent.at(-1)).toEqual({ t: 'closed', id: 't1' })
  })

  it('Dock back asks Live to show the terminal, closes the window, and says so when the window stays', async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true })
    const close = vi.spyOn(window, 'close').mockImplementation(() => {})
    vi.stubGlobal('fetch', appServer({ terms: [termSession({ id: 't1' })] }))
    render(<TermWindow id="t1" />)
    fireEvent.click(await screen.findByRole('button', { name: 'Dock back' }))
    expect(FakeChannel.sent).toContainEqual({ t: 'dock', id: 't1' })
    expect(close).toHaveBeenCalled()
    act(() => vi.advanceTimersByTime(400))
    expect(document.querySelector('.term-window-bar')).toHaveTextContent('You can close this window')
    expect(screen.queryByRole('button', { name: 'Dock back' })).toBeNull()
    close.mockRestore()
    vi.useRealTimers()
  })
})
