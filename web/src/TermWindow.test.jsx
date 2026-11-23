import { render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import TermWindow from './TermWindow.jsx'
import { FakeResizeObserver, FakeSocket, resetFakes } from './test/fakeXterm.js'
import { appServer, termSession } from './test/appSessions.js'

vi.mock('@xterm/xterm', async () => ({ Terminal: (await import('./test/fakeXterm.js')).Terminal }))
vi.mock('@xterm/addon-fit', async () => ({ FitAddon: (await import('./test/fakeXterm.js')).FitAddon }))

// The pop-out window (T135, AC 5): `/?term=<id>#<token>` shows only that terminal, filling the window.
beforeEach(() => {
  resetFakes()
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
    expect(FakeSocket.last.url).toBe('ws://localhost:3000/api/term/t1')
    expect(document.title).toBe('company · terminal')
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
})
