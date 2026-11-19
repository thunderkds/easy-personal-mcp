import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import LiveView from './LiveView.jsx'
import { ALREADY, EXPIRED, HANDED_BACK, NOTICE_MS, SENT } from './PendingAnswer.jsx'
import PromptCard, { readPrompt, TOO_LONG } from './PromptCard.jsx'

// The answer card of a terminal session in the Live view (T107, FR-114, FR-115) and the reusable prompt card. Fixtures only.
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

// T106's exact 409 texts (rust/crates/app/src/answer_bridge.rs)
const INCOMPLETE = 'the tool input is too long to show whole, so it cannot be allowed from here; deny it or hand it back to the terminal'
const SECRET = '[hidden: looks like a secret]'
const question = (over = {}) => ({
  id: 'p1',
  kind: 'question',
  questions: [
    { question: 'Which database?', header: 'Database', multi_select: false, options: [{ label: 'Postgres', description: 'Server' }, { label: 'SQLite', description: 'File' }] },
  ],
  ...over,
})
const permission = (over = {}) => ({ id: 'p2', kind: 'permission', tool: 'Bash', input: 'git status', complete: true, ...over })
const session = (id, pending, over = {}) => ({
  session_id: id, account: 'Claude work', project: 'alpha', last_ts: new Date().toISOString(),
  tool_calls: 0, tokens_in: 0, tokens_out: 0, cost_usd: 0, ...(pending ? { pending_answer: pending } : {}), ...over,
})
const snapshot = (sessions) => ({ active_window_s: 300, last_id: 0, sessions })
/** The card is in the session's panel (T118): the first session that holds a prompt is opened. */
const openCard = (sessions) => {
  const first = sessions.find((s) => s.pending_answer)
  if (first) fireEvent.click(document.getElementById(`live-row-${first.session_id}`))
}
const show = (sessions) => {
  render(<LiveView />)
  stream().emit('snapshot', snapshot(sessions))
  openCard(sessions)
}

/** `routes`: answer / release -> a status (200 default). Records every request. */
function stubFetch(routes = {}) {
  const calls = []
  const f = vi.fn(async (url, init = {}) => {
    if (String(url).endsWith('/transcript')) return { ok: false, status: 404, json: async () => ({ error: 'no conversation in this test' }) } // the panel's own read: not one of the answer calls
    calls.push({ url: String(url), init, body: init.body ? JSON.parse(init.body) : undefined })
    const status = (url === '/api/answer' ? routes.answer : routes.release) ?? 200
    const error = { 400: 'choose an option or type an answer for every question', 404: 'no such question or permission is waiting', 409: 'already answered' }[status]
    return { ok: status < 400, status, json: async () => (status < 400 ? { ok: true } : { error }) }
  })
  vi.stubGlobal('fetch', f)
  return calls
}
const click = fireEvent.click

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

describe('readPrompt', () => {
  it('keeps a well-formed question and permission, and drops anything else', () => {
    expect(readPrompt(question()).questions[0].options.map((o) => o.label)).toEqual(['Postgres', 'SQLite'])
    expect(readPrompt(permission())).toMatchObject({ kind: 'permission', tool: 'Bash', input: 'git status', complete: true })
    for (const bad of [undefined, null, 'x', {}, { id: 'a', kind: 'other' }, { id: 'a', kind: 'question', questions: [] }, { id: 'a', kind: 'question', questions: [{ question: 'q', options: [] }] }, { kind: 'permission', tool: 'Bash' }]) {
      expect(readPrompt(bad)).toBeNull()
    }
  })
  it('fails closed: only complete: true is complete; a missing, false or non-boolean complete is not', () => {
    expect(readPrompt(permission())).toMatchObject({ complete: true })
    for (const complete of [false, undefined, 'true', 1, null]) expect(readPrompt(permission({ complete }))).toMatchObject({ complete: false })
    expect(readPrompt({ id: 'p', kind: 'permission', tool: 'Bash', input: 'x' })).toMatchObject({ complete: false })
  })
})

describe('AC2: the card on a session with pending_answer', () => {
  it('shows a question as options plus Other, Answer in terminal, and no card for a session with none', () => {
    show([session('aaaaaaaa-1', question()), session('bbbbbbbb-2')])
    const card = screen.getByRole('region', { name: 'Question from a terminal session' })
    expect(within(card).getByText('Which database?')).toBeInTheDocument()
    expect(within(card).getByText('Database')).toBeInTheDocument()
    expect(within(card).getAllByRole('radio').map((r) => r.closest('label').textContent)).toEqual(['PostgresServer', 'SQLiteFile', 'Other'])
    expect(within(card).getByRole('button', { name: 'Answer in terminal' })).toBeInTheDocument()
    expect(within(card).getByRole('button', { name: 'Send answer' })).toBeDisabled()
    expect(document.querySelectorAll('.prompt-card')).toHaveLength(1)
  })

  it('answers a single-select by option position', async () => {
    const calls = stubFetch()
    show([session('aaaaaaaa-1', question())])
    click(screen.getByRole('radio', { name: /SQLite/ }))
    click(screen.getByRole('button', { name: 'Send answer' }))
    await screen.findByText(SENT)
    expect(calls).toHaveLength(1)
    expect(calls[0].url).toBe('/api/answer')
    expect(calls[0].init.method).toBe('POST')
    expect(calls[0].body).toEqual({ id: 'p1', confirmed: true, answers: [{ selected: [1] }] })
    expect(calls[0].init.headers['X-Dashboard-Token']).toBe('secret-token')
    expect(calls[0].url).not.toContain('secret-token') // the token is never in a URL
  })

  it('answers with Other text, and a multi-select with several positions plus Other', async () => {
    const calls = stubFetch()
    const multi = question({
      questions: [
        { question: 'Which?', header: '', multi_select: true, options: [{ label: 'A', description: '' }, { label: 'B', description: '' }, { label: 'C', description: '' }] },
        { question: 'Why?', header: '', multi_select: false, options: [{ label: 'Because', description: '' }] },
      ],
    })
    show([session('aaaaaaaa-1', multi)])
    click(screen.getByRole('checkbox', { name: 'C' }))
    click(screen.getByRole('checkbox', { name: 'A' }))
    click(screen.getByRole('checkbox', { name: 'Other' }))
    fireEvent.change(screen.getByLabelText('Your own answer to question 1'), { target: { value: '  my own  ' } })
    expect(screen.getByRole('button', { name: 'Send answer' })).toBeDisabled() // question 2 is open
    click(screen.getAllByRole('radio', { name: 'Other' })[0])
    expect(screen.getByRole('button', { name: 'Send answer' })).toBeDisabled() // Other chosen but empty
    fireEvent.change(screen.getByLabelText('Your own answer to question 2'), { target: { value: 'because I say' } })
    click(screen.getByRole('button', { name: 'Send answer' }))
    await screen.findByText(SENT)
    expect(calls[0].body.answers).toEqual([{ selected: [0, 2], other: 'my own' }, { selected: [], other: 'because I say' }])
  })

  it('a question hidden as a secret is shown as the server sent it and is still answerable by position', async () => {
    const calls = stubFetch()
    const hidden = question({ questions: [{ question: SECRET, header: '', multi_select: false, options: [{ label: 'Yes', description: '' }, { label: 'No', description: '' }] }] })
    show([session('aaaaaaaa-1', hidden)])
    expect(screen.getAllByText(SECRET).length).toBeGreaterThan(0)
    expect(screen.getByText(/looks like a secret, so it is hidden here/)).toBeInTheDocument()
    click(screen.getByRole('radio', { name: 'No' }))
    click(screen.getByRole('button', { name: 'Send answer' }))
    await screen.findByText(SENT)
    expect(calls[0].body.answers).toEqual([{ selected: [1] }])
  })

  it('a permission shows the tool and its whole input, and Allow / Deny send the decision; Deny carries the note', async () => {
    const calls = stubFetch()
    show([session('aaaaaaaa-1', permission({ input: 'rm -rf build' }))])
    const card = screen.getByRole('region', { name: 'Permission request from a terminal session' })
    expect(within(card).getByText('Bash asks for permission')).toBeInTheDocument()
    expect(within(card).getByText('rm -rf build')).toBeInTheDocument()
    click(within(card).getByRole('button', { name: 'Allow' }))
    await screen.findByText(SENT)
    expect(calls[0].body).toEqual({ id: 'p2', confirmed: true, decision: 'allow' })
  })

  it('Deny sends the note as the message', async () => {
    const calls = stubFetch()
    show([session('aaaaaaaa-1', permission())])
    fireEvent.change(screen.getByLabelText('Note to Claude (sent with Deny)'), { target: { value: 'not that one' } })
    click(screen.getByRole('button', { name: 'Deny' }))
    await screen.findByText(SENT)
    expect(calls[0].body).toEqual({ id: 'p2', confirmed: true, decision: 'deny', message: 'not that one' })
  })

  it('Answer in terminal hands the prompt back and says the terminal asks now', async () => {
    const calls = stubFetch()
    show([session('aaaaaaaa-1', permission())])
    click(screen.getByRole('button', { name: 'Answer in terminal' }))
    await screen.findByText(HANDED_BACK)
    expect(calls[0].url).toBe('/api/answer/release')
    expect(calls[0].body).toEqual({ id: 'p2', confirmed: true })
  })

  it('409 says already answered and 404 says the terminal asks now; a second click while a request is out is dropped', async () => {
    const calls = stubFetch({ answer: 409 })
    show([session('aaaaaaaa-1', permission())])
    const allow = screen.getByRole('button', { name: 'Allow' })
    click(allow)
    click(allow)
    await screen.findByText(ALREADY)
    expect(calls).toHaveLength(1)
    vi.unstubAllGlobals()
    vi.stubGlobal('EventSource', FakeEventSource)
    stubFetch({ answer: 404 })
    act(() => stream().emit('snapshot', snapshot([session('aaaaaaaa-1', permission({ id: 'p3' }))])))
    click(screen.getByRole('button', { name: 'Allow' }))
    await screen.findByText(EXPIRED)
  })

  it('a 409 that is not "already answered" (an allow the server refuses) keeps the card and shows the server\'s note', async () => {
    vi.stubGlobal('fetch', vi.fn(async () => ({ ok: false, status: 409, json: async () => ({ error: INCOMPLETE }) })))
    show([session('aaaaaaaa-1', permission())])
    click(screen.getByRole('button', { name: 'Allow' }))
    expect(await screen.findByRole('alert')).toHaveTextContent(INCOMPLETE)
    expect(screen.queryByText(ALREADY)).toBeNull()
  })

  it('a refused answer (400) shows the server\'s reason in the card and keeps the card', async () => {
    stubFetch({ answer: 400 })
    show([session('aaaaaaaa-1', question())])
    click(screen.getByRole('radio', { name: /Postgres/ }))
    click(screen.getByRole('button', { name: 'Send answer' }))
    expect(await screen.findByRole('alert')).toHaveTextContent('choose an option or type an answer for every question')
    await waitFor(() => expect(screen.getByRole('button', { name: 'Send answer' })).toBeEnabled())
  })

  it('a failed request (network) asks to try again and keeps the card', async () => {
    vi.stubGlobal('fetch', vi.fn(async () => { throw new Error('offline') }))
    show([session('aaaaaaaa-1', permission())])
    click(screen.getByRole('button', { name: 'Allow' }))
    expect(await screen.findByRole('alert')).toHaveTextContent('Could not send the answer. Try again.')
  })

  it('a prompt that leaves the stream without our answer says the terminal asks now, and the line ends after a while', () => {
    vi.useFakeTimers()
    show([session('aaaaaaaa-1', permission())])
    expect(screen.queryByText(EXPIRED)).toBeNull()
    stream().emit('snapshot', snapshot([session('aaaaaaaa-1')]))
    expect(screen.getByText(EXPIRED)).toBeInTheDocument()
    expect(document.querySelector('.prompt-card')).toBeNull()
    act(() => vi.advanceTimersByTime(NOTICE_MS))
    expect(screen.queryByText(EXPIRED)).toBeNull()
  })

  it('a new prompt replaces the line of the last one', () => {
    show([session('aaaaaaaa-1', permission())])
    stream().emit('snapshot', snapshot([session('aaaaaaaa-1')]))
    expect(screen.getByText(EXPIRED)).toBeInTheDocument()
    stream().emit('snapshot', snapshot([session('aaaaaaaa-1', permission({ id: 'p9' }))]))
    expect(screen.queryByText(EXPIRED)).toBeNull()
    expect(screen.getByRole('button', { name: 'Allow' })).toBeInTheDocument()
  })

  it('a session that leaves the list with its prompt shows no line and no card', () => {
    show([session('aaaaaaaa-1', permission())])
    stream().emit('snapshot', snapshot([]))
    expect(screen.queryByText(EXPIRED)).toBeNull()
    expect(document.querySelector('.prompt-card')).toBeNull()
  })

  it('a pending_answer of a shape it does not know draws no card', () => {
    show([session('aaaaaaaa-1', { id: 'x', kind: 'unknown', text: 'x' })])
    expect(document.querySelector('.prompt-card')).toBeNull()
  })

  it('draws the prompt text as text, never as HTML, and keeps nothing in localStorage', () => {
    const setItem = vi.spyOn(Storage.prototype, 'setItem')
    show([session('aaaaaaaa-1', permission({ input: '<img src=x onerror=alert(1)> <b>bold</b>' }))])
    expect(document.querySelector('.prompt-summary img')).toBeNull()
    expect(document.querySelector('.prompt-summary b')).toBeNull()
    expect(screen.getByText('<img src=x onerror=alert(1)> <b>bold</b>')).toBeInTheDocument()
    expect(setItem).not.toHaveBeenCalled()
  })
})

describe('the permission card shows what is allowed (T106 security review)', () => {
  const css = () => readFileSync(resolve(process.cwd(), 'src/appsession.css'), 'utf8')
  const card = (over) => render(<PromptCard prompt={readPrompt(permission(over))} onAnswer={() => {}} onRelease={() => {}} />)

  it('puts the summary in a monospace pre-wrap block in the page flow: no max-height and no inner scroll, in any CSS file', () => {
    const rule = css().match(/\.prompt-summary \{[^}]*\}/)[0]
    expect(rule).toMatch(/font-family: var\(--font-mono\)[^}]*white-space: pre-wrap/)
    for (const file of ['appsession.css', 'app.css', 'watch.css']) {
      const all = readFileSync(resolve(process.cwd(), 'src', file), 'utf8')
      const own = all.split('\n').filter((l) => /\.prompt-(summary|card|question|options|option|text)\b/.test(l)).join('\n')
      expect(own, file).not.toMatch(/max-height|overflow-y|overflow: (auto|scroll)/)
    }
    card({ input: 'echo a      b' })
    expect(document.querySelector('pre.prompt-summary').textContent).toBe('echo a      b') // every space reaches the block
  })

  it('keeps line breaks, visible escapes and a long pretty-printed input as one text', () => {
    const input = '{\n  "plan": "line \\u{202e} two"\n}'
    card({ tool: 'ExitPlanMode', input })
    expect(document.querySelector('pre.prompt-summary').textContent).toBe(input)
  })

  it('shows the fixed banner when the input was cut, and none otherwise', () => {
    const { unmount } = card({ complete: false })
    expect(screen.getByText(TOO_LONG)).toBeInTheDocument()
    unmount()
    card({})
    expect(screen.queryByText(TOO_LONG)).toBeNull()
  })

  it('an incomplete input offers Deny and Answer in terminal, and no Allow', () => {
    card({ complete: false })
    expect(screen.queryByRole('button', { name: 'Allow' })).toBeNull()
    expect(screen.getByRole('button', { name: 'Deny' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Answer in terminal' })).toBeInTheDocument()
  })

  it('fails closed: no Allow when complete is missing, or the input is empty, even with complete: true', () => {
    for (const over of [{ complete: undefined }, { input: '' }, { input: undefined }]) {
      const { unmount } = card(over)
      expect(screen.queryByRole('button', { name: 'Allow' }), JSON.stringify(over)).toBeNull()
      expect(screen.getByRole('button', { name: 'Deny' })).toBeInTheDocument()
      unmount()
    }
  })
})

describe('sorting and the rest of the row', () => {
  it('lists a session with a card before one with none, and hides the plain question text while the card shows it', () => {
    show([
      session('aaaaaaaa-1', null, { last_ts: new Date().toISOString() }),
      session('bbbbbbbb-2', permission(), { last_ts: new Date(Date.now() - 60_000).toISOString() }),
    ])
    const ids = [...document.querySelectorAll('.live-id')].map((e) => e.textContent)
    expect(ids).toEqual(['bbbbbbbb', 'aaaaaaaa'])
    const waiting = session('cccccccc-3', question(), { attention: [{ state: 'waiting', questions: ['Which database?'] }] })
    act(() => stream().emit('snapshot', snapshot([waiting])))
    expect(document.querySelectorAll('.live-question')).toHaveLength(1) // the panel is closed: the row shows the plain text
    openCard([waiting])
    expect(document.querySelectorAll('.live-question')).toHaveLength(0) // open: the card shows it, once
    expect(screen.getAllByText('Which database?')).toHaveLength(1)
  })
})

describe('a new prompt starts clean (key by prompt id)', () => {
  it('an error on prompt A is gone when prompt B arrives in the same session', async () => {
    stubFetch({ answer: 400 })
    show([session('aaaaaaaa-1', permission({ id: 'pA' }))])
    click(screen.getByRole('button', { name: 'Allow' }))
    expect(await screen.findByRole('alert')).toHaveTextContent('choose an option or type an answer for every question')
    act(() => stream().emit('snapshot', snapshot([session('aaaaaaaa-1', permission({ id: 'pB', input: 'ls' }))])))
    expect(screen.getByText('ls')).toBeInTheDocument()
    expect(screen.queryByRole('alert')).toBeNull()
    await waitFor(() => expect(screen.getByRole('button', { name: 'Allow' })).toBeEnabled())
  })
})
