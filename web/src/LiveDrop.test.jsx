import { act, fireEvent, render, screen, within } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import LiveView from './LiveView.jsx'
import { FILTERS_KEY } from './LiveFilters.jsx'
import { FakeEventSource, liveRow, liveServer, page, snapshot, stream } from './test/liveStream.js'

// T121 AC 3: nothing drops when a session changes state. Each test drives the states through the mocked stream (working -> idle -> needs you ->
// ended, rows added, removed and re-sorted) and checks that the open panel, the typed text, the focus and the rows under the pointer stay put.
const A = 'aaaaaaaa-1111-2222'
const B = 'bbbbbbbb-3333-4444'
const panel = () => screen.getByRole('region', { name: /^Session in / })
const rowEl = (id) => document.getElementById(`live-row-${id}`)
const order = () => [...document.querySelectorAll('.live-row')].map((b) => b.id.replace('live-row-', ''))
const rows = () => document.querySelector('.live-rows')
const ts = (secondsAgo) => new Date(Date.now() - secondsAgo * 1000).toISOString()
const until = () => Math.floor(Date.now() / 1000) + 600

// The states of one terminal session with the answer bridge on.
const busy = (over = {}) => ({ reply_bridge: true, proc: { busy: true }, ...over })
const idle = (over = {}) => ({ reply_bridge: true, proc: { busy: false }, ...over })
const yourTurn = (over = {}) => ({ reply_bridge: true, proc: { busy: false }, attention: [{ state: 'your_turn' }], pending_reply: { id: 'r1', until: until(), last: 'Done.' }, ...over })

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

const show = (...list) => stream().emit('snapshot', snapshot(list))

async function openOne(over) {
  vi.stubGlobal('fetch', liveServer({ pages: [page([])] }))
  render(<LiveView />)
  show(liveRow(A, over))
  fireEvent.click(rowEl(A))
  await screen.findByRole('log', { name: 'Conversation' })
}

describe('the open panel stays put (F5, F6)', () => {
  it('working -> idle -> needs you -> ended: the same panel node; the typed text and the focus are kept; ended says no longer active', async () => {
    await openOne(busy())
    const node = panel()
    const queue = within(node).getByLabelText(/Queue a message/)
    queue.focus()
    fireEvent.change(queue, { target: { value: 'run the tests' } })

    show(liveRow(A, idle())) // idle: no box, the text is kept and the focus stays in the composer
    expect(panel()).toBe(node)
    expect(within(node).getByText(/Your text is kept/)).toBeInTheDocument()
    expect(document.activeElement).not.toBe(document.body)
    expect(node.contains(document.activeElement)).toBe(true)

    show(liveRow(A, yourTurn())) // needs you: the Reply box has the same text and the focus
    expect(panel()).toBe(node)
    const reply = within(node).getByLabelText(/Reply to session/)
    expect(reply).toHaveValue('run the tests')
    expect(reply).toHaveFocus()

    show(liveRow(A, busy())) // working again: back in the Queue box, text and focus kept
    const again = within(node).getByLabelText(/Queue a message/)
    expect(again).toHaveValue('run the tests')
    expect(again).toHaveFocus()

    show() // ended: it left the list; the panel stays and says so
    expect(panel()).toBe(node)
    expect(within(node).getByText(/no longer active/)).toBeInTheDocument()
    expect(within(node).queryByRole('textbox')).toBeNull() // no composer
    expect(within(node).queryByRole('button', { name: 'Stop' })).toBeNull() // no controls
    fireEvent.click(within(node).getByRole('button', { name: 'Back to sessions' }))
    expect(document.getElementById('live-panel')).toBeNull()
  })

  it('another session changing state does not close or redraw the open panel of a quiet one', async () => {
    vi.stubGlobal('fetch', liveServer({ pages: [page([])] }))
    render(<LiveView />)
    show(liveRow(A, busy()), liveRow(B, busy()))
    fireEvent.click(rowEl(A))
    const node = panel()
    show(liveRow(B, idle())) // A went quiet past the window and the snapshot was re-sent for B
    expect(panel()).toBe(node)
    show(liveRow(A, busy()), liveRow(B, idle())) // A back: the same panel, live again
    expect(panel()).toBe(node)
    expect(within(node).queryByText(/no longer active/)).toBeNull()
    expect(within(node).getByRole('button', { name: 'Stop' })).toBeInTheDocument()
  })
})

describe('the focus is never taken back (F5, negative)', () => {
  it('the owner moved the focus out of the composer: a change of state leaves it where the owner put it', async () => {
    await openOne(busy())
    const back = within(panel()).getByRole('button', { name: 'Back to sessions' })
    within(panel()).getByLabelText(/Queue a message/).focus()
    back.focus()
    show(liveRow(A, yourTurn()))
    expect(back).toHaveFocus()
  })

  it('no text typed: the idle line does not claim a text is kept', async () => {
    await openOne(busy())
    show(liveRow(A, idle()))
    expect(within(panel()).queryByText(/Your text is kept/)).toBeNull()
  })
})

describe('the panel is not remounted when its row key changes (F10)', () => {
  it('the app list answering after the stream (id -> app:<id>) keeps the same panel node and text', async () => {
    vi.useFakeTimers()
    const view = { id: 'a1', session_id: A, state: 'working', folder: '/w/alpha', account: { label: 'Claude work' }, items: [], model: 'm', mode: 'default', worktree: false }
    let apps = []
    const inner = liveServer({ pages: [page([])] })
    const server = async (url, init = {}) =>
      new URL(String(url), 'http://127.0.0.1').pathname === '/api/app-sessions' ? { ok: true, status: 200, json: async () => ({ sessions: apps }) } : inner(url, init)
    vi.stubGlobal('fetch', server)
    render(<LiveView appSessions />)
    await act(async () => {
      await vi.advanceTimersByTimeAsync(0)
    })
    show(liveRow(A, busy()))
    fireEvent.click(rowEl(A))
    const node = panel()
    fireEvent.change(within(node).getByLabelText(/Queue a message/), { target: { value: 'keep me' } })
    apps = [view]
    await act(async () => {
      await vi.advanceTimersByTimeAsync(5100)
    })
    expect(rowEl(`app:a1`)).not.toBeNull()
    expect(panel()).toBe(node)
    expect(within(node).getByLabelText('Message to Claude')).toHaveValue('keep me') // the composer follows the kind; the text stays
  })
})

describe('rows do not move or vanish under the pointer (F7, F8)', () => {
  it('while the pointer is on the list a re-sort or a new state keeps the order; leaving it lets the list sort again', () => {
    vi.stubGlobal('fetch', liveServer())
    render(<LiveView />)
    show(liveRow(A, { last_ts: ts(60) }), liveRow(B, { last_ts: ts(10) }))
    expect(order()).toEqual([B, A])
    const hovered = rowEl(A)
    fireEvent.pointerEnter(rows())
    show(liveRow(A, { last_ts: ts(1) }), liveRow(B, { last_ts: ts(10) })) // A is now the newest
    expect(order()).toEqual([B, A])
    show(liveRow(A, { last_ts: ts(1), attention: [{ state: 'your_turn' }] }), liveRow(B, { last_ts: ts(10) })) // and needs you
    expect(order()).toEqual([B, A])
    expect(rowEl(A)).toBe(hovered)
    fireEvent.pointerLeave(rows())
    expect(order()).toEqual([A, B])
  })

  it('the keyboard focus on a row holds the order too, until the focus leaves the list', () => {
    vi.stubGlobal('fetch', liveServer())
    render(<LiveView />)
    show(liveRow(A, { last_ts: ts(60) }), liveRow(B, { last_ts: ts(10) }))
    act(() => rowEl(A).focus())
    show(liveRow(A, { last_ts: ts(1) }), liveRow(B, { last_ts: ts(10) }))
    expect(order()).toEqual([B, A])
    act(() => rowEl(A).blur())
    expect(order()).toEqual([A, B])
  })

  it('with Working set, a hovered row whose session goes idle stays listed until the pointer leaves', () => {
    window.localStorage.setItem(FILTERS_KEY, JSON.stringify({ state: 'working', project: '', kind: '' }))
    vi.stubGlobal('fetch', liveServer())
    render(<LiveView />)
    show(liveRow(A, busy()), liveRow(B, busy()))
    fireEvent.pointerEnter(rows())
    show(liveRow(A, idle()), liveRow(B, busy()))
    expect(rowEl(A)).not.toBeNull()
    fireEvent.pointerLeave(rows())
    expect(rowEl(A)).toBeNull()
  })

  it('a row added while the pointer is on the list goes at the end, the others keep their place', () => {
    vi.stubGlobal('fetch', liveServer())
    render(<LiveView />)
    show(liveRow(A, { last_ts: ts(60) }))
    fireEvent.pointerEnter(rows())
    show(liveRow(A, { last_ts: ts(60) }), liveRow(B, { last_ts: ts(1) }))
    expect(order()).toEqual([A, B])
  })
})

describe('the head and the list do not jump (F9, F11, F20)', () => {
  it('the "need you" line is always drawn: the same node with 0 and with 1', () => {
    vi.stubGlobal('fetch', liveServer())
    render(<LiveView />)
    show(liveRow(A))
    const line = document.querySelector('.live-need')
    expect(line).not.toBeNull()
    expect(line).toHaveTextContent('')
    show(liveRow(A, { attention: [{ state: 'your_turn' }] }))
    expect(document.querySelector('.live-need')).toBe(line)
    expect(line).toHaveTextContent('1 session needs you')
  })

  it('the footnote of the totals is folded in a closed details', () => {
    vi.stubGlobal('fetch', liveServer())
    render(<LiveView />)
    show(liveRow(A))
    const about = screen.getByText(/API-equivalent cost is tokens times/).closest('details')
    expect(about).not.toBeNull()
    expect(about.open).toBe(false)
  })

  it('a click anywhere on the row card opens it, not only on its first line', () => {
    vi.stubGlobal('fetch', liveServer())
    render(<LiveView />)
    show(liveRow(A))
    fireEvent.click(rowEl(A).closest('li').querySelector('.live-bars'))
    expect(rowEl(A)).toHaveAttribute('aria-expanded', 'true')
  })
})
