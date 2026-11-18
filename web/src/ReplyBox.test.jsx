import { act, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import LiveView from './LiveView.jsx'
import ReplyBox, { GONE, MAX_CHARS, MARKED_NOTE } from './ReplyBox.jsx'
import { holdSession, replyHold, snapshotOf } from './test/fixtures.js'

// The reply box and *Reply from the app* in the Live view (T112, FR-122). Fixtures only: no request leaves the test.
class FakeEventSource {
  static instances = []
  constructor(url) {
    this.url = url
    this.listeners = {}
    FakeEventSource.instances.push(this)
  }
  addEventListener(type, fn) {
    ;(this.listeners[type] ??= []).push(fn)
  }
  close() {}
  emit(type, data) {
    act(() => (this.listeners[type] ?? []).forEach((fn) => fn({ type, data: JSON.stringify(data), lastEventId: '' })))
  }
}
const stream = () => FakeEventSource.instances.at(-1)

/** `routes`: path -> status number or a function (body) -> status. GET /api/hooks answers `hooks`. Records every request. */
function stubFetch({ routes = {}, hooks = { accounts: [{ id: 1, label: 'Claude work', status: 'on' }] } } = {}) {
  const calls = []
  vi.stubGlobal(
    'fetch',
    vi.fn(async (url, init = {}) => {
      const body = init.body ? JSON.parse(init.body) : undefined
      calls.push({ url: String(url), method: init.method ?? 'GET', body, headers: init.headers })
      if (url === '/api/hooks') return { ok: true, status: 200, json: async () => hooks }
      const route = routes[url]
      const status = typeof route === 'function' ? route(body) : (route ?? 200)
      const error = { 400: 'the reply is empty or too long', 404: 'no such session', 409: 'no session is waiting for a reply with that id', 500: 'boom' }[status]
      return { ok: status < 400, status, json: async () => (status < 400 ? { ok: true } : { error }) }
    }),
  )
  return calls
}
const posted = (calls, path) => calls.filter((c) => c.url === path)
const box = () => screen.getByRole('textbox', { name: /reply/i })
const type = (text) => fireEvent.change(box(), { target: { value: text } })
const reply = (over = {}) => replyHold(over)

beforeEach(() => {
  FakeEventSource.instances = []
  window.location.hash = '#secret-token'
  vi.stubGlobal('EventSource', FakeEventSource)
})
afterEach(() => {
  vi.useRealTimers()
  vi.restoreAllMocks()
  vi.unstubAllGlobals()
})

describe('AC1: the box of a session with pending_reply', () => {
  it('shows the last text as plain text, a message box, Send, Hand back and the time left', () => {
    stubFetch()
    render(<ReplyBox sessionId="s1" reply={reply({ last: 'Done. <b>bold</b>\nNext?' })} open={false} />)
    expect(screen.getByText(/Done\. <b>bold<\/b>/)).toBeInTheDocument()
    expect(document.querySelector('.reply-last b')).toBeNull()
    expect(box()).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Send' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Hand back to terminal' })).toBeInTheDocument()
    expect(screen.getByText('waits 1m 20s more')).toBeInTheDocument()
  })

  it('counts down each second', () => {
    vi.useFakeTimers()
    stubFetch()
    render(<ReplyBox sessionId="s1" reply={reply({ until: Date.now() / 1000 + 65 })} open={false} />)
    expect(screen.getByText('waits 1m 5s more')).toBeInTheDocument()
    act(() => vi.advanceTimersByTime(1000))
    expect(screen.getByText('waits 1m 4s more')).toBeInTheDocument()
    act(() => vi.advanceTimersByTime(10_000))
    expect(screen.getByText('waits 54s more')).toBeInTheDocument()
  })

  it('is drawn in the Live row of a session with a hold', () => {
    stubFetch()
    render(<LiveView />)
    stream().emit('snapshot', snapshotOf([holdSession('s1')]))
    expect(screen.getByRole('button', { name: 'Hand back to terminal' })).toBeInTheDocument()
    expect(screen.getByText('Waiting on you for the next step.')).toBeInTheDocument()
  })
})

describe('AC2: keys and limits', () => {
  it('Enter sends, Shift+Enter does not, IME composition (keyCode 229) does not', async () => {
    const calls = stubFetch()
    render(<ReplyBox sessionId="s1" reply={reply()} open={false} />)
    type('go on')
    fireEvent.keyDown(box(), { key: 'Enter', shiftKey: true })
    fireEvent.keyDown(box(), { key: 'Enter', keyCode: 229 })
    expect(posted(calls, '/api/reply')).toHaveLength(0)
    fireEvent.keyDown(box(), { key: 'Enter' })
    await waitFor(() => expect(posted(calls, '/api/reply')).toHaveLength(1))
  })

  it('Send is disabled for an empty or blank box and for more than 8,000 characters, and the count shows near the limit', () => {
    stubFetch()
    render(<ReplyBox sessionId="s1" reply={reply()} open={false} />)
    const send = screen.getByRole('button', { name: 'Send' })
    expect(send).toBeDisabled()
    type('   ')
    expect(send).toBeDisabled()
    expect(screen.queryByText(/ \/ 8,000/)).toBeNull()
    type('a'.repeat(MAX_CHARS))
    expect(send).toBeEnabled()
    expect(screen.getByText('8,000 / 8,000')).toBeInTheDocument()
    type('a'.repeat(MAX_CHARS + 1))
    expect(send).toBeDisabled()
    expect(screen.getByRole('alert')).toHaveTextContent('8,001 / 8,000')
  })

  it('counts characters as typed: 8,000 emoji are within the limit', () => {
    stubFetch()
    render(<ReplyBox sessionId="s1" reply={reply()} open={false} />)
    type('😀'.repeat(MAX_CHARS))
    expect(screen.getByRole('button', { name: 'Send' })).toBeEnabled()
  })
})

describe('AC3: Send', () => {
  it('posts {id, text, confirmed: true} with the token in the header only, then says "sent"', async () => {
    const calls = stubFetch()
    render(<ReplyBox sessionId="s1" reply={reply({ id: 'r9' })} open={false} />)
    type('  keep going\nplease ')
    fireEvent.click(screen.getByRole('button', { name: 'Send' }))
    expect(await screen.findByText('sent')).toBeInTheDocument()
    const call = posted(calls, '/api/reply')[0]
    expect(call.body).toEqual({ id: 'r9', text: '  keep going\nplease ', confirmed: true })
    expect(call.headers['X-Dashboard-Token']).toBe('secret-token')
    expect(call.url).not.toContain('secret-token')
    expect(box()).toBeDisabled()
  })

  it('409 says the session is no longer waiting and keeps the text', async () => {
    stubFetch({ routes: { '/api/reply': 409 } })
    render(<ReplyBox sessionId="s1" reply={reply()} open={false} />)
    type('hello')
    fireEvent.click(screen.getByRole('button', { name: 'Send' }))
    expect(await screen.findByRole('alert')).toHaveTextContent(GONE)
    expect(GONE).toBe('This session is no longer waiting')
    expect(box()).toHaveValue('hello')
  })

  it('a 400 and a 500 both show the server line, and both keep the text', async () => {
    stubFetch({ routes: { '/api/reply': 400 } })
    render(<ReplyBox sessionId="s1" reply={reply()} open={false} />)
    type('hello')
    fireEvent.click(screen.getByRole('button', { name: 'Send' }))
    expect(await screen.findByRole('alert')).toHaveTextContent('the reply is empty or too long')
    expect(box()).toHaveValue('hello')
    vi.mocked(fetch).mockImplementationOnce(async () => ({ ok: false, status: 500, json: async () => ({ error: 'boom' }) }))
    fireEvent.click(screen.getByRole('button', { name: 'Send' }))
    await waitFor(() => expect(screen.getByRole('alert')).toHaveTextContent('boom'))
    expect(box()).toHaveValue('hello')
    expect(fetch).toHaveBeenCalledTimes(2)
  })

  it('text typed while the request is out is not sent and not kept: a 200 clears the box and says "sent"', async () => {
    let release
    const bodies = []
    vi.stubGlobal('fetch', vi.fn((url, init) => {
      bodies.push(JSON.parse(init.body))
      return new Promise((resolve) => (release = () => resolve({ ok: true, status: 200, json: async () => ({ ok: true }) })))
    }))
    render(<ReplyBox sessionId="s1" reply={reply()} open={false} />)
    type('first')
    fireEvent.click(screen.getByRole('button', { name: 'Send' }))
    await waitFor(() => expect(release).toBeTypeOf('function'))
    type('first and a late thought') // typed while the request is out
    expect(box()).toHaveValue('first and a late thought')
    await act(async () => release())
    expect(await screen.findByText('sent')).toBeInTheDocument()
    expect(bodies).toEqual([{ id: 'r1', text: 'first', confirmed: true }])
    expect(box()).toHaveValue('')
  })

  it('the row returns to normal when pending_reply goes away', async () => {
    stubFetch()
    render(<LiveView />)
    stream().emit('snapshot', snapshotOf([holdSession('s1')]))
    type('go')
    fireEvent.click(screen.getByRole('button', { name: 'Send' }))
    expect(await screen.findByText('sent')).toBeInTheDocument()
    stream().emit('snapshot', snapshotOf([holdSession('s1', { pending_reply: undefined })]))
    expect(screen.queryByText('sent')).toBeNull()
    expect(screen.queryByRole('button', { name: 'Send' })).toBeNull()
  })
})

describe('AC4: Hand back to terminal', () => {
  it('posts /api/reply/release {id, confirmed}', async () => {
    const calls = stubFetch()
    render(<ReplyBox sessionId="s1" reply={reply({ id: 'r9' })} open={false} />)
    fireEvent.click(screen.getByRole('button', { name: 'Hand back to terminal' }))
    await waitFor(() => expect(posted(calls, '/api/reply/release')).toHaveLength(1))
    expect(posted(calls, '/api/reply/release')[0].body).toEqual({ id: 'r9', confirmed: true })
  })

  it('a 409 (it ended) says the session is no longer waiting', async () => {
    stubFetch({ routes: { '/api/reply/release': 409 } })
    render(<ReplyBox sessionId="s1" reply={reply()} open={false} />)
    fireEvent.click(screen.getByRole('button', { name: 'Hand back to terminal' }))
    expect(await screen.findByRole('alert')).toHaveTextContent(GONE)
  })
})

describe('AC5: the watch call', () => {
  const watches = (calls) => posted(calls, '/api/reply/watch')

  it('is sent every 20 s while the row is open, and stops when it closes', async () => {
    vi.useFakeTimers()
    const calls = stubFetch()
    const { rerender } = render(<ReplyBox sessionId="s1" reply={reply()} open />)
    await act(async () => {})
    const first = watches(calls).length
    expect(first).toBeGreaterThanOrEqual(1)
    expect(watches(calls)[0].body).toEqual({ session_id: 's1', confirmed: true })
    await act(async () => vi.advanceTimersByTime(20_000))
    expect(watches(calls)).toHaveLength(first + 1)
    await act(async () => vi.advanceTimersByTime(40_000))
    expect(watches(calls)).toHaveLength(first + 3)
    rerender(<ReplyBox sessionId="s1" reply={reply()} open={false} />)
    await act(async () => vi.advanceTimersByTime(60_000))
    expect(watches(calls)).toHaveLength(first + 3)
  })

  it('is sent while the box has focus, and stops on blur when the row is closed', async () => {
    vi.useFakeTimers()
    const calls = stubFetch()
    render(<ReplyBox sessionId="s1" reply={reply()} open={false} />)
    await act(async () => vi.advanceTimersByTime(60_000))
    expect(watches(calls)).toHaveLength(0)
    fireEvent.focus(box())
    await act(async () => vi.advanceTimersByTime(20_000))
    const n = watches(calls).length
    expect(n).toBeGreaterThanOrEqual(1)
    fireEvent.blur(box())
    await act(async () => vi.advanceTimersByTime(60_000))
    expect(watches(calls)).toHaveLength(n)
  })

  it('keeps going on blur while the row is open', async () => {
    vi.useFakeTimers()
    const calls = stubFetch()
    render(<ReplyBox sessionId="s1" reply={reply()} open />)
    await act(async () => {})
    fireEvent.focus(box())
    fireEvent.blur(box())
    const n = watches(calls).length
    await act(async () => vi.advanceTimersByTime(20_000))
    expect(watches(calls).length).toBeGreaterThan(n)
  })

  it('a session without pending_reply never gets a watch call', async () => {
    vi.useFakeTimers()
    const calls = stubFetch()
    render(<LiveView />)
    stream().emit('snapshot', snapshotOf([holdSession('s1', { pending_reply: undefined })]))
    fireEvent.click(document.querySelector('.live-row'))
    await act(async () => vi.advanceTimersByTime(60_000))
    expect(watches(calls)).toHaveLength(0)
  })

  it('opening a row of the Live view starts it', async () => {
    const calls = stubFetch()
    render(<LiveView />)
    stream().emit('snapshot', snapshotOf([holdSession('s1')]))
    expect(watches(calls)).toHaveLength(0)
    fireEvent.click(document.querySelector('.live-row'))
    await waitFor(() => expect(watches(calls)).toHaveLength(1))
  })
})

describe('AC6: Reply from the app', () => {
  const idle = (over = {}) => holdSession('s1', { pending_reply: undefined, reply_bridge: true, ...over }) // the server's flag: all three hook entries of its account are on
  const toggle = () => screen.getByRole('button', { name: 'Reply from the app' })

  it('a bridge-enabled session that is not your turn shows the toggle, and a click posts /api/reply/mark', async () => {
    const calls = stubFetch()
    render(<LiveView />)
    stream().emit('snapshot', snapshotOf([idle()]))
    await screen.findByRole('button', { name: 'Reply from the app' })
    expect(toggle()).toHaveAttribute('aria-pressed', 'false')
    fireEvent.click(toggle())
    await waitFor(() => expect(posted(calls, '/api/reply/mark')).toHaveLength(1))
    expect(posted(calls, '/api/reply/mark')[0].body).toEqual({ session_id: 's1', on: true, confirmed: true })
  })

  it('shows what the server says: reply_marked sets aria-pressed and the note; a click then unmarks', async () => {
    const calls = stubFetch()
    render(<LiveView />)
    stream().emit('snapshot', snapshotOf([idle({ reply_marked: true })]))
    await screen.findByRole('button', { name: 'Reply from the app' })
    expect(toggle()).toHaveAttribute('aria-pressed', 'true')
    expect(screen.getByText(MARKED_NOTE)).toBeInTheDocument()
    expect(MARKED_NOTE).toBe('marked: it waits for you at its next turn end')
    fireEvent.click(toggle())
    await waitFor(() => expect(posted(calls, '/api/reply/mark')[0].body.on).toBe(false))
  })

  it('a marked session that has a hold shows the box, not the "marked" note', async () => {
    stubFetch()
    render(<LiveView />)
    stream().emit('snapshot', snapshotOf([holdSession('s1', { reply_marked: true, reply_bridge: true })]))
    await screen.findByRole('button', { name: 'Reply from the app' })
    expect(screen.queryByText(MARKED_NOTE)).toBeNull()
    expect(screen.getByRole('button', { name: 'Send' })).toBeInTheDocument()
  })

  it('no toggle without the server flag reply_bridge, and no /api/hooks read to find out', async () => {
    const calls = stubFetch()
    render(<LiveView />)
    stream().emit('snapshot', snapshotOf([idle({ reply_bridge: undefined })]))
    await act(async () => {})
    expect(screen.queryByRole('button', { name: 'Reply from the app' })).toBeNull()
    expect(calls.filter((c) => c.url === '/api/hooks')).toHaveLength(0)
  })

  it('the toggle follows the flag: it shows when a later snapshot carries reply_bridge', async () => {
    stubFetch()
    render(<LiveView />)
    stream().emit('snapshot', snapshotOf([idle({ reply_bridge: undefined })]))
    expect(screen.queryByRole('button', { name: 'Reply from the app' })).toBeNull()
    stream().emit('snapshot', snapshotOf([idle()]))
    expect(screen.getByRole('button', { name: 'Reply from the app' })).toBeInTheDocument()
  })

  it('no toggle for a session at your turn, even with the flag', async () => {
    stubFetch()
    render(<LiveView />)
    stream().emit('snapshot', snapshotOf([idle({ attention: [{ state: 'your_turn', count: 1 }] })]))
    await act(async () => {})
    expect(screen.queryByRole('button', { name: 'Reply from the app' })).toBeNull()
  })

  it('a refused mark (404) shows the server line', async () => {
    stubFetch({ routes: { '/api/reply/mark': 404 } })
    render(<LiveView />)
    stream().emit('snapshot', snapshotOf([idle()]))
    await screen.findByRole('button', { name: 'Reply from the app' })
    fireEvent.click(toggle())
    expect(await screen.findByRole('alert')).toHaveTextContent('no such session')
  })
})

describe('AC7: nothing kept, token never in a URL', () => {
  it('writes nothing to localStorage and puts the token in no URL', async () => {
    const setItem = vi.spyOn(Storage.prototype, 'setItem')
    const calls = stubFetch()
    render(<LiveView />)
    stream().emit('snapshot', snapshotOf([holdSession('s1')]))
    type('a private request')
    fireEvent.click(screen.getByRole('button', { name: 'Send' }))
    await screen.findByText('sent')
    expect(setItem).not.toHaveBeenCalled()
    expect(JSON.stringify(Object.entries(localStorage))).not.toContain('private')
    for (const c of calls) expect(c.url).not.toContain('secret-token')
  })
})
