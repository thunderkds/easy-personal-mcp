import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import TermPanel from './TermPanel.jsx'
import { FakeResizeObserver, FakeSocket, FitAddon, Terminal, resetFakes } from './test/fakeXterm.js'

vi.mock('@xterm/xterm', async () => ({ Terminal: (await import('./test/fakeXterm.js')).Terminal }))
vi.mock('@xterm/addon-fit', async () => ({ FitAddon: (await import('./test/fakeXterm.js')).FitAddon }))

// The terminal panel (T134, FR-136..FR-138): xterm draws, a WebSocket carries the bytes. The contract is T133's: the first frame is the auth frame
// (token never in the URL), binary frames are stdin / screen bytes, text frames are `resize`, `exit`, `pages`, `notice`.
const bytes = (text) => new TextEncoder().encode(text)
const TERM = { id: 't1', repo: '/work/company', account: { id: '2', label: 'Claude personal' }, state: 'running', code: null, pages: 1 }
const fetchOk = () => vi.fn(async () => ({ ok: true, status: 200, json: async () => ({}) }))
let onChanged
const mount = (term = {}, props = {}) => {
  onChanged = vi.fn()
  return render(<TermPanel term={{ ...TERM, ...term }} onBack={() => {}} onChanged={onChanged} {...props} />)
}
const attach = () => {
  const socket = FakeSocket.last
  socket.open()
  return socket
}

beforeEach(() => {
  resetFakes()
  window.location.hash = '#secret-token'
  vi.stubGlobal('WebSocket', FakeSocket)
  vi.stubGlobal('ResizeObserver', FakeResizeObserver)
  vi.stubGlobal('fetch', fetchOk())
})
afterEach(() => {
  vi.useRealTimers()
  vi.unstubAllGlobals()
})

describe('the socket (AC 2)', () => {
  it('opens /api/term/{id} with no token in the URL and sends the auth frame first, with the fitted size', () => {
    mount()
    const socket = FakeSocket.last
    expect(socket.url).toBe('ws://localhost:3000/api/term/t1')
    expect(socket.url).not.toContain('secret-token')
    expect(socket.sent).toEqual([]) // nothing before the socket is open
    FitAddon.size = { cols: 100, rows: 30 }
    socket.open()
    expect(socket.sent).toHaveLength(1)
    expect(socket.texts[0]).toEqual({ t: 'auth', token: 'secret-token', cols: 100, rows: 30 })
  })

  it('encodes the id into the path and uses wss on an https page', () => {
    const { location } = window
    vi.stubGlobal('location', { ...location, protocol: 'https:', host: 'app.test' })
    mount({ id: 'a/b' })
    expect(FakeSocket.last.url).toBe('wss://app.test/api/term/a%2Fb')
  })

  it('draws with xterm: the fit runs once the terminal is open, binary frames from the server are written as bytes', () => {
    mount()
    const term = Terminal.last
    expect(term.element).not.toBeNull()
    expect(FitAddon.fits).toBeGreaterThan(0)
    const socket = attach()
    socket.receive(bytes('hello \x1b[31mred\x1b[0m'))
    expect(term.written).toHaveLength(1)
    expect(ArrayBuffer.isView(term.written[0])).toBe(true)
    expect(new TextDecoder().decode(term.written[0])).toBe('hello \x1b[31mred\x1b[0m')
  })

  it('sends typed text as one binary frame of UTF-8 bytes, in order, after the auth frame', () => {
    mount()
    const socket = attach()
    Terminal.last.type('ls\r')
    Terminal.last.type('é')
    expect(socket.sent[0]).toEqual(expect.any(String)) // auth first
    expect(socket.binaries.map((b) => [...b])).toEqual([[...bytes('ls\r')], [...bytes('é')]])
    expect(ArrayBuffer.isView(socket.binaries[0])).toBe(true)
  })

  it('sends nothing typed before the socket is open', () => {
    mount()
    Terminal.last.type('x')
    FakeSocket.last.open()
    expect(FakeSocket.last.binaries).toEqual([])
  })

  it('sends a resize when the box changes size, debounced: a burst is one frame, an unchanged size is none', () => {
    vi.useFakeTimers()
    mount()
    const socket = attach()
    const observer = FakeResizeObserver.instances[0]
    FitAddon.size = { cols: 90, rows: 20 }
    observer.fire()
    observer.fire()
    observer.fire()
    expect(socket.texts.filter((m) => m.t === 'resize')).toEqual([]) // not yet
    act(() => vi.advanceTimersByTime(300))
    expect(socket.texts.filter((m) => m.t === 'resize')).toEqual([{ t: 'resize', cols: 90, rows: 20 }])
    observer.fire()
    act(() => vi.advanceTimersByTime(300))
    expect(socket.texts.filter((m) => m.t === 'resize')).toHaveLength(1) // same size: nothing to say
  })

  it('closes the socket and frees the terminal when the panel goes', () => {
    const { unmount } = mount()
    const socket = attach()
    unmount()
    expect(socket.closed).toBe(true)
    expect(Terminal.last.disposed).toBe(true)
    expect(FakeResizeObserver.instances[0].disconnected).toBe(true)
  })

  it('a malformed or unknown text frame is ignored', () => {
    mount()
    const socket = attach()
    act(() => socket.onmessage({ data: 'not json' }))
    socket.receive({ t: 'whatever' })
    expect(screen.queryByRole('alert')).toBeNull()
  })
})

describe('the key row (AC 3)', () => {
  const KEYS = [
    ['Esc', '\x1b'],
    ['Tab', '\t'],
    ['Ctrl-C', '\x03'],
    ['Enter', '\r'],
    ['Up', '\x1b[A'],
    ['Down', '\x1b[B'],
    ['Left', '\x1b[D'],
    ['Right', '\x1b[C'],
  ]
  it.each(KEYS)('%s sends the right bytes as one binary frame and gives the focus back to the terminal', (name, sequence) => {
    mount()
    const socket = attach()
    const before = Terminal.last.focused
    fireEvent.click(within(screen.getByRole('group', { name: 'Terminal keys' })).getByRole('button', { name }))
    expect(socket.binaries.map((b) => [...b])).toEqual([[...bytes(sequence)]])
    expect(Terminal.last.focused).toBeGreaterThan(before)
  })

  it('has exactly these eight keys, each a labelled button', () => {
    mount()
    const buttons = within(screen.getByRole('group', { name: 'Terminal keys' })).getAllByRole('button')
    expect(buttons.map((b) => b.getAttribute('aria-label') ?? b.textContent)).toEqual(KEYS.map(([name]) => name))
  })

  it('sends nothing while the socket is not open', () => {
    mount()
    fireEvent.click(screen.getByRole('button', { name: 'Esc' }))
    expect(FakeSocket.last.binaries).toEqual([])
  })
})

describe('the end and the pages (AC 4)', () => {
  it('an exit frame says "Ended (code N)", turns the input off and leaves the screen to read', () => {
    mount()
    const socket = attach()
    socket.receive(bytes('last words'))
    socket.receive({ t: 'exit', code: 3 })
    expect(screen.getByRole('status')).toHaveTextContent('Ended (code 3)')
    Terminal.last.type('x')
    expect(socket.binaries).toEqual([])
    for (const name of ['Esc', 'Tab', 'Ctrl-C', 'Up', 'Down', 'Left', 'Right', 'Enter']) expect(screen.getByRole('button', { name })).toBeDisabled()
    expect(Terminal.last.disposed).toBe(false)
    expect(Terminal.last.written).toHaveLength(1) // nothing was cleared
    expect(Terminal.last.options.disableStdin).toBe(true)
    expect(screen.queryByRole('button', { name: 'Stop' })).toBeNull()
    expect(onChanged).toHaveBeenCalled() // Live reads the list again
  })

  it('a socket that closes after the exit frame is not a lost connection', () => {
    mount()
    const socket = attach()
    socket.receive({ t: 'exit', code: 0 })
    socket.drop()
    expect(screen.getByRole('status')).toHaveTextContent('Ended (code 0)')
    expect(screen.queryByRole('button', { name: 'Reconnect' })).toBeNull()
  })

  it('a session the list already shows as exited reads as ended before any frame', () => {
    mount({ state: 'exited', code: 1 })
    expect(screen.getByRole('status')).toHaveTextContent('Ended (code 1)')
    expect(screen.getByRole('button', { name: 'Enter' })).toBeDisabled()
  })

  it('a pages frame shows "N pages attached" when N > 1 and nothing for one page', () => {
    mount()
    const socket = attach()
    expect(screen.queryByText(/pages attached/)).toBeNull()
    socket.receive({ t: 'pages', n: 3 })
    expect(screen.getByText('3 pages attached')).toBeInTheDocument()
    socket.receive({ t: 'pages', n: 1 })
    expect(screen.queryByText(/pages attached/)).toBeNull()
  })

  it('a notice frame says input was dropped, and typing still works', () => {
    mount()
    const socket = attach()
    socket.receive({ t: 'notice' })
    expect(screen.getByText(/Some input was dropped/)).toBeInTheDocument()
    Terminal.last.type('a')
    expect(socket.binaries).toHaveLength(1)
  })

  it('Stop posts /stop with confirmed and the token in the header only, then asks Live to read again', async () => {
    const fetchSpy = fetchOk()
    vi.stubGlobal('fetch', fetchSpy)
    mount()
    attach()
    fireEvent.click(screen.getByRole('button', { name: 'Stop' }))
    await waitFor(() => expect(fetchSpy).toHaveBeenCalled())
    const [url, init] = fetchSpy.mock.calls[0]
    expect(url).toBe('/api/term-sessions/t1/stop')
    expect(init.method).toBe('POST')
    expect(JSON.parse(init.body)).toEqual({ confirmed: true })
    expect(init.headers['X-Dashboard-Token']).toBe('secret-token')
    await waitFor(() => expect(onChanged).toHaveBeenCalled())
  })

  it('a refused Stop shows the server line and keeps the panel', async () => {
    vi.stubGlobal('fetch', async () => ({ ok: false, status: 404, json: async () => ({ error: 'no such session' }) }))
    mount()
    attach()
    fireEvent.click(screen.getByRole('button', { name: 'Stop' }))
    expect(await screen.findByRole('alert')).toHaveTextContent('no such session')
    expect(screen.getByRole('button', { name: 'Stop' })).toBeEnabled()
  })
})

describe('re-attach (AC 5)', () => {
  it('a panel opened again (reload, another page) opens its own socket, authenticates, and draws the replay the server sends', () => {
    const first = mount()
    attach().receive(bytes('screen'))
    first.unmount()
    mount()
    const second = attach()
    expect(FakeSocket.instances).toHaveLength(2)
    expect(second.texts[0].t).toBe('auth')
    second.receive(bytes('screen again'))
    expect(new TextDecoder().decode(Terminal.last.written[0])).toBe('screen again')
  })

  it('a lost connection says so and Reconnect opens a fresh socket on a cleared screen, so the replay is not drawn twice', () => {
    mount()
    const socket = attach()
    socket.receive(bytes('old screen'))
    socket.drop()
    expect(screen.getByRole('alert')).toHaveTextContent('Disconnected')
    fireEvent.click(screen.getByRole('button', { name: 'Reconnect' }))
    expect(Terminal.last.resets).toBe(1)
    expect(FakeSocket.instances).toHaveLength(2)
    const next = attach()
    expect(next.texts[0].t).toBe('auth')
    expect(screen.queryByRole('alert')).toBeNull()
  })

  it('shows a connecting line until the socket is open', () => {
    mount()
    expect(screen.getByRole('status')).toHaveTextContent('Connecting')
    attach()
    expect(screen.getByRole('status')).not.toHaveTextContent('Connecting')
  })
})

describe('the panel', () => {
  it('names the project and account, takes the Back button, and keeps the terminal in a labelled group', () => {
    const onBack = vi.fn()
    mount({}, { onBack })
    expect(screen.getByRole('region', { name: 'Terminal session in company' })).toBeInTheDocument()
    expect(screen.getByText('Claude personal')).toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: 'Back to sessions' }))
    expect(onBack).toHaveBeenCalled()
    expect(screen.getByRole('group', { name: 'Terminal' })).toBeInTheDocument()
  })

  it('keeps nothing: no storage and no console output from what the terminal shows', () => {
    const log = vi.spyOn(console, 'log').mockImplementation(() => {})
    window.localStorage.clear()
    mount()
    attach().receive(bytes('secret screen text'))
    expect(window.localStorage.length).toBe(0)
    expect(window.sessionStorage.length).toBe(0)
    expect(log).not.toHaveBeenCalled()
    log.mockRestore()
  })
})

describe('the roomy panel (T135)', () => {
  it('has one compact head: Back, the title, the status, Pop out and Stop; account and folder are one line, the folder in full on hover', () => {
    mount()
    const head = document.querySelector('.term-head')
    expect(within(head).getByRole('button', { name: 'Back to sessions' })).toBeInTheDocument()
    expect(within(head).getByRole('heading', { name: /company/ })).toBeInTheDocument()
    expect(within(head).getByRole('status')).toHaveTextContent('Connecting')
    expect(within(head).getByRole('button', { name: 'Pop out' })).toBeInTheDocument()
    expect(within(head).getByRole('button', { name: 'Stop' })).toBeInTheDocument()
    const meta = document.querySelector('.term-meta')
    expect(meta).toHaveTextContent('Claude personal')
    expect(within(meta).getByText('/work/company')).toHaveAttribute('title', '/work/company')
    expect(document.querySelector('dl')).toBeNull() // no three-row facts list
  })

  it('Pop out opens /?term=<id>#<token> in a new window with no opener, and tells Live', () => {
    const win = { opener: {} }
    const open = vi.fn(() => win)
    vi.stubGlobal('open', open)
    const onPoppedOut = vi.fn()
    mount({ id: 'a/b' }, { onPoppedOut })
    fireEvent.click(screen.getByRole('button', { name: 'Pop out' }))
    expect(open).toHaveBeenCalledWith('/?term=a%2Fb#secret-token', '_blank')
    expect(win.opener).toBeNull()
    expect(onPoppedOut).toHaveBeenCalled()
    expect(screen.queryByRole('alert')).toBeNull()
  })

  it('a blocked pop-up says so and the panel stays', () => {
    vi.stubGlobal('open', vi.fn(() => null))
    const onPoppedOut = vi.fn()
    mount({}, { onPoppedOut })
    fireEvent.click(screen.getByRole('button', { name: 'Pop out' }))
    expect(screen.getByRole('alert')).toHaveTextContent('blocked the new window')
    expect(onPoppedOut).not.toHaveBeenCalled()
  })

  it('in the desktop app window the app opens the window itself (window.open answers null there): no blocked alert', () => {
    vi.stubGlobal('open', vi.fn(() => null))
    window.__EASY_PERSONAL_APP__ = true
    try {
      const onPoppedOut = vi.fn()
      mount({}, { onPoppedOut })
      fireEvent.click(screen.getByRole('button', { name: 'Pop out' }))
      expect(screen.queryByRole('alert')).toBeNull()
      expect(onPoppedOut).toHaveBeenCalled()
    } finally {
      delete window.__EASY_PERSONAL_APP__
    }
  })

  it('popped: no Back and no Pop out, and the screen keeps its keys and Stop', () => {
    mount({}, { popped: true, onBack: undefined })
    expect(screen.queryByRole('button', { name: 'Back to sessions' })).toBeNull()
    expect(screen.queryByRole('button', { name: 'Pop out' })).toBeNull()
    expect(screen.getByRole('button', { name: 'Stop' })).toBeInTheDocument()
    expect(screen.getByRole('group', { name: 'Terminal keys' })).toBeInTheDocument()
    expect(document.querySelector('.term-panel').dataset.popped).toBe('true')
    expect(Terminal.last.focused).toBe(1) // the keys go to the terminal at once
  })

  it('a plain refit right after a focus keeps the forced send', () => {
    vi.useFakeTimers()
    mount()
    const socket = attach()
    act(() => Terminal.last.textarea.dispatchEvent(new FocusEvent('focus')))
    FakeResizeObserver.instances[0].fire() // the box settles within the same 150 ms, at the same size
    act(() => vi.advanceTimersByTime(300))
    expect(socket.texts.filter((m) => m.t === 'resize')).toEqual([{ t: 'resize', cols: 100, rows: 30 }])
  })

  it('focusing the screen sends its size even when unchanged, so the page you type in wins (question 47)', () => {
    vi.useFakeTimers()
    mount()
    const socket = attach()
    expect(socket.texts.filter((m) => m.t === 'resize')).toEqual([])
    act(() => Terminal.last.textarea.dispatchEvent(new FocusEvent('focus')))
    act(() => vi.advanceTimersByTime(300))
    expect(socket.texts.filter((m) => m.t === 'resize')).toEqual([{ t: 'resize', cols: 100, rows: 30 }])
  })
})
