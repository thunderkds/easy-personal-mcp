import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import LiveView from './LiveView.jsx'
import { answerBody } from './SessionPanel.jsx'
import { readPrompt } from './PromptCard.jsx'
import { FakeEventSource, snapshot, stream } from './test/liveStream.js'
import { appServer, appSession as baseSession, assistant, planPrompt, questionPrompt, tool, toolPrompt, user } from './test/appSessions.js'

// The app session's side of the one Live panel (T118, FR-128): what the App sessions page held before (T104: header, conversation, message box,
// Interrupt / Stop / Close, prompt cards) now lives in the panel of a Live row. A session with no Claude session id yet shows its own conversation.
const APPS_FAST_MS = 1000
const appSession = (over = {}) => baseSession({ session_id: null, ...over })

const install = (opts) => {
  const server = appServer(opts)
  vi.stubGlobal('fetch', server)
  return server
}
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
const rowOf = (id = 'a1') => document.getElementById(`live-row-app:${id}`)
/** The Live view with no terminal session; the app sessions are read from the server and listed. */
const mount = (props = {}) => {
  render(<LiveView appSessions {...props} />)
  stream().emit('snapshot', snapshot([]))
}
const mountFake = async (id = 'a1') => {
  mount()
  await act(async () => {
    await vi.advanceTimersByTimeAsync(0)
  })
  fireEvent.click(rowOf(id))
}
const open = async (id = 'a1') => {
  mount()
  await waitFor(() => expect(rowOf(id)).not.toBeNull())
  fireEvent.click(rowOf(id))
  await screen.findByRole('region', { name: /^Session in / })
}
const panel = () => screen.getByRole('region', { name: /^Session in / })
const log = () => screen.getByRole('log', { name: 'Conversation' })
/** `server` with one kind of call held: the answer is computed now (the server's state at that moment) and handed back on `release()`. */
const hold = (server, match) => {
  const held = { release: () => {}, on: false }
  const fn = (url, init = {}) => {
    const call = { method: init.method ?? 'GET', path: new URL(String(url), 'http://127.0.0.1').pathname }
    if (!held.on || !match(call)) return server(url, init)
    held.on = false
    const answer = server(url, init)
    return new Promise((resolve) => {
      held.release = () => resolve(answer)
    })
  }
  Object.assign(fn, { calls: server.calls, state: server.state, posts: server.posts })
  vi.stubGlobal('fetch', fn)
  return held
}

describe('the panel header (AC 4)', () => {
  it('shows the session id, account, model, mode and folder', async () => {
    install({ sessions: [appSession({ session_id: 'sess-1111', model: 'opus', mode: 'acceptEdits' })] })
    await open()
    const facts = within(panel())
    for (const [name, value] of [['Session', 'sess-1111'], ['Account', 'Claude personal'], ['Model', 'opus'], ['Mode', 'acceptEdits'], ['Folder', '/work/company']])
      expect(facts.getByText(name, { selector: 'dt' }).nextSibling).toHaveTextContent(value)
    expect(facts.queryByText('Branch')).toBeNull() // not a worktree
  })

  it('before its first init the session id reads "starting", and no account reads as the default account', async () => {
    install({ sessions: [appSession({ session_id: null, account: null })] })
    await open()
    expect(within(panel()).getByText('Session', { selector: 'dt' }).nextSibling).toHaveTextContent('starting')
    expect(within(panel()).getByText('Account', { selector: 'dt' }).nextSibling).toHaveTextContent('Default Claude Code account')
  })

  it('a worktree shows its branch and the changed file names; none yet says so', async () => {
    const files = ['src/a.rs', 'README.md']
    install({ sessions: [appSession({ worktree: true, branch: 'easy/20261117-1210-ab12', changed_files: files, folder: '/data/worktrees/x' })] })
    await open()
    expect(within(panel()).getByText('Branch', { selector: 'dt' }).nextSibling).toHaveTextContent('easy/20261117-1210-ab12')
    expect(within(panel()).getAllByRole('listitem').map((li) => li.textContent)).toEqual(expect.arrayContaining(files))
  })

  it('a worktree with no change shows "none yet"', async () => {
    install({ sessions: [appSession({ worktree: true, branch: 'easy/b', changed_files: [] })] })
    await open()
    expect(within(panel()).getByText('Changed files', { selector: 'dt' }).nextSibling).toHaveTextContent('none yet')
  })

  it('the state word is a polite status; an ended session gives its reason', async () => {
    install({ sessions: [appSession({ state: 'ended', reason: 'claude not found' })] })
    await open()
    expect(within(panel()).getByRole('status')).toHaveTextContent('Ended: claude not found')
  })
})

describe('the conversation (AC 2)', () => {
  it('shows the owner\'s messages, the assistant text and each tool call as its label with its outcome, all as text', async () => {
    const hostile = '<img src=x onerror="window.__pwned=1"> **not bold**'
    install({
      sessions: [appSession({
        state: 'working',
        items: [user('/help'), assistant(hostile), tool('Bash', 'ls', 'running'), tool('mcp__github__list', { server: 'github', tool: 'list' }, 'ok'), tool(null, null, 'error')],
      })],
    })
    await open()
    expect(within(log()).getByText('/help')).toBeInTheDocument()
    expect(within(log()).getByText(hostile)).toBeInTheDocument()
    expect(log().querySelector('img')).toBeNull()
    expect(window.__pwned).toBeUndefined()
    expect(within(log()).getByText('Bash · ls').nextSibling).toHaveTextContent('running')
    expect(within(log()).getByText('mcp__github__list · github · list').nextSibling).toHaveTextContent('ok')
    expect(within(log()).getByText('a tool').nextSibling).toHaveTextContent('error')
    expect(within(log()).getAllByText(/^(You|Claude)$/).map((e) => e.textContent)).toEqual(['You', 'Claude'])
  })

  it('an empty conversation says so', async () => {
    install({ sessions: [appSession()] })
    await open()
    expect(within(log()).getByText('The conversation shows when Claude starts the session.')).toBeInTheDocument()
  })

  it('streams: a new item shows up on the next read without a click', async () => {
    vi.useFakeTimers()
    const server = install({ sessions: [appSession({ state: 'working', items: [user('hi')] })] })
    await mountFake()
    await act(async () => {
      await vi.advanceTimersByTimeAsync(0)
    })
    expect(within(log()).queryByText('hello back')).toBeNull()
    server.state.sessions = [appSession({ state: 'idle', items: [user('hi'), assistant('hello back')] })]
    await act(async () => {
      await vi.advanceTimersByTimeAsync(APPS_FAST_MS)
    })
    expect(within(log()).getByText('hello back')).toBeInTheDocument()
    expect(within(panel()).getByRole('status')).toHaveTextContent('Waiting for your message')
  })

  it('a failed read keeps the last answer and says the app cannot be reached; the next good read clears it', async () => {
    vi.useFakeTimers()
    const server = install({ sessions: [appSession({ items: [assistant('still here')] })] })
    await mountFake()
    await act(async () => {
      await vi.advanceTimersByTimeAsync(0)
    })
    server.state.fail = true
    await act(async () => {
      await vi.advanceTimersByTimeAsync(APPS_FAST_MS)
    })
    expect(screen.getByText(/Cannot reach the app right now/)).toBeInTheDocument()
    expect(within(log()).getByText('still here')).toBeInTheDocument()
    server.state.fail = false
    await act(async () => {
      await vi.advanceTimersByTimeAsync(APPS_FAST_MS)
    })
    expect(screen.queryByText(/Cannot reach the app right now/)).toBeNull()
  })
})

describe('the message box (AC 2)', () => {
  const box = () => screen.getByLabelText('Message to Claude')

  it('Enter sends the text as typed, once, and clears the box; the answer updates the conversation', async () => {
    const server = install({
      sessions: [appSession()],
      routes: { 'POST /api/app-sessions/a1/message': (body) => appSession({ state: 'working', items: [user(body.text)] }) },
    })
    await open()
    fireEvent.change(box(), { target: { value: '/help' } })
    fireEvent.keyDown(box(), { key: 'Enter' })
    await waitFor(() => expect(box()).toHaveValue(''))
    const posts = server.posts('/message')
    expect(posts).toHaveLength(1)
    expect(posts[0].body).toEqual({ text: '/help', confirmed: true })
    expect(posts[0].url).toBe('/api/app-sessions/a1/message')
    expect(await within(log()).findByText('/help')).toBeInTheDocument()
  })

  it('Shift+Enter is a new line, not a send; an empty box sends nothing', async () => {
    const server = install({ sessions: [appSession()] })
    await open()
    fireEvent.change(box(), { target: { value: 'line one' } })
    fireEvent.keyDown(box(), { key: 'Enter', shiftKey: true })
    fireEvent.change(box(), { target: { value: '   ' } })
    fireEvent.keyDown(box(), { key: 'Enter' })
    expect(screen.getByRole('button', { name: 'Send' })).toBeDisabled()
    expect(server.posts('/message')).toHaveLength(0)
  })

  it('Enter while an input method is composing does not send', async () => {
    const server = install({ sessions: [appSession()] })
    await open()
    fireEvent.change(box(), { target: { value: 'ni' } })
    fireEvent.keyDown(box(), { key: 'Enter', isComposing: true })
    expect(server.posts('/message')).toHaveLength(0)
  })

  it('Safari ends a composition with keyCode 229 and isComposing false: that Enter does not send either', async () => {
    const server = install({ sessions: [appSession()] })
    await open()
    fireEvent.change(box(), { target: { value: 'ni' } })
    fireEvent.keyDown(box(), { key: 'Enter', keyCode: 229 })
    expect(server.posts('/message')).toHaveLength(0)
  })

  it('text typed while a message is on its way is kept; only the text that was sent is cleared', async () => {
    vi.useFakeTimers()
    const server = install({ sessions: [appSession()], routes: { 'POST /api/app-sessions/a1/message': () => appSession({ state: 'working' }) } })
    const held = hold(server, (c) => c.method === 'POST' && c.path.endsWith('/message'))
    await mountFake()
    await act(async () => {
      await vi.advanceTimersByTimeAsync(0)
    })
    held.on = true
    fireEvent.change(box(), { target: { value: 'first' } })
    fireEvent.keyDown(box(), { key: 'Enter' })
    fireEvent.change(box(), { target: { value: 'first and the next thought' } })
    await act(async () => {
      held.release()
      await vi.advanceTimersByTimeAsync(0)
    })
    expect(server.posts('/message')[0].body.text).toBe('first')
    expect(box()).toHaveValue(' and the next thought')
  })

  it('the size limit is in bytes: a message of 2-byte characters over 100,000 bytes is not sent, one at the limit is', async () => {
    const server = install({ sessions: [appSession()], routes: { 'POST /api/app-sessions/a1/message': () => appSession({ state: 'working' }) } })
    await open()
    fireEvent.change(box(), { target: { value: '\u00e9'.repeat(50_001) } }) // 50,001 characters, 100,002 bytes
    expect(screen.getByRole('alert')).toHaveTextContent('100,000 bytes')
    expect(screen.getByRole('button', { name: 'Send' })).toBeDisabled()
    fireEvent.keyDown(box(), { key: 'Enter' })
    expect(server.posts('/message')).toHaveLength(0)
    fireEvent.change(box(), { target: { value: '\u00e9'.repeat(50_000) } }) // exactly 100,000 bytes
    expect(screen.queryByRole('alert')).toBeNull()
    fireEvent.click(screen.getByRole('button', { name: 'Send' }))
    await waitFor(() => expect(server.posts('/message')).toHaveLength(1))
  })

  it('the Send button sends too', async () => {
    const server = install({ sessions: [appSession()], routes: { 'POST /api/app-sessions/a1/message': () => appSession({ state: 'working' }) } })
    await open()
    fireEvent.change(box(), { target: { value: 'go on' } })
    fireEvent.click(screen.getByRole('button', { name: 'Send' }))
    await waitFor(() => expect(server.posts('/message')).toHaveLength(1))
  })

  it('a refused message keeps the text and shows the server\'s words', async () => {
    install({ sessions: [appSession()], routes: { 'POST /api/app-sessions/a1/message': () => ({ status: 409, body: { error: 'the session has ended' } }) } })
    await open()
    fireEvent.change(box(), { target: { value: 'still typing' } })
    fireEvent.keyDown(box(), { key: 'Enter' })
    expect(await screen.findByRole('alert')).toHaveTextContent('the session has ended')
    expect(box()).toHaveValue('still typing')
  })

  it('an ended or stopping session takes no message', async () => {
    install({ sessions: [appSession({ state: 'ended', reason: 'stopped' })] })
    await open()
    expect(box()).toBeDisabled()
    expect(screen.getByRole('button', { name: 'Send' })).toBeDisabled()
  })
})

describe('Interrupt, Stop and Close (AC 2)', () => {
  it('Interrupt and Stop post confirmed: true and take the answer', async () => {
    const server = install({
      sessions: [appSession({ state: 'working' })],
      routes: {
        'POST /api/app-sessions/a1/interrupt': () => appSession({ state: 'idle' }),
        'POST /api/app-sessions/a1/stop': () => appSession({ state: 'ended', reason: 'stopped' }),
      },
    })
    await open()
    fireEvent.click(screen.getByRole('button', { name: 'Interrupt' }))
    await waitFor(() => expect(within(panel()).getByRole('status')).toHaveTextContent('Waiting for your message'))
    expect(server.posts('/interrupt')[0].body).toEqual({ confirmed: true })
    expect(screen.getByRole('button', { name: 'Interrupt' })).toBeDisabled() // nothing to interrupt when idle
    fireEvent.click(screen.getByRole('button', { name: 'Stop' }))
    await waitFor(() => expect(within(panel()).getByRole('status')).toHaveTextContent('Ended: stopped'))
    expect(server.posts('/stop')[0].body).toEqual({ confirmed: true })
  })

  it('both are off once the session has ended, and Close drops the panel', async () => {
    const server = install({ sessions: [appSession({ state: 'ended', reason: 'stopped', items: [assistant('secret text')] })], routes: { 'POST /api/app-sessions/a1/close': () => ({ closed: true }) } })
    await open()
    expect(screen.getByRole('button', { name: 'Interrupt' })).toBeDisabled()
    expect(screen.getByRole('button', { name: 'Stop' })).toBeDisabled()
    fireEvent.click(screen.getByRole('button', { name: 'Close' }))
    await waitFor(() => expect(screen.queryByRole('log')).toBeNull())
    expect(screen.queryByText('secret text')).toBeNull()
    expect(server.posts('/close')[0].body).toEqual({ confirmed: true })
  })

  it('a refused stop shows the server\'s words', async () => {
    install({ sessions: [appSession({ state: 'working' })], routes: { 'POST /api/app-sessions/a1/stop': () => ({ status: 404, body: { error: 'no such session' } }) } })
    await open()
    fireEvent.click(screen.getByRole('button', { name: 'Stop' }))
    expect(await screen.findByRole('alert')).toHaveTextContent('no such session')
  })

  it('a session id is a path piece: it is encoded', async () => {
    const server = install({ sessions: [appSession({ id: 'a/../1', state: 'working' })], routes: { 'POST /api/app-sessions/a%2F..%2F1/stop': () => appSession({ id: 'a/../1', state: 'ended' }) } })
    await open('a/../1')
    fireEvent.click(screen.getByRole('button', { name: 'Stop' }))
    await waitFor(() => expect(server.posts('/stop')).toHaveLength(1))
    expect(server.posts('/stop')[0].path).toBe('/api/app-sessions/a%2F..%2F1/stop')
  })
})

describe('prompt cards (AC 3)', () => {
  const answered = (server, routeId = 'a1') => server.calls.filter((c) => c.method === 'POST' && c.path === `/api/app-sessions/${routeId}/answer`)

  it('Allow answers the prompt by id with behavior allow; the card stays until the server says answered', async () => {
    let view = appSession({ state: 'waiting', items: [user('x'), toolPrompt()] })
    const server = install({ sessions: [view], routes: { 'POST /api/app-sessions/a1/answer': () => view } })
    await open()
    expect(within(panel()).getByText('ls -la')).toBeInTheDocument()
    expect(screen.getByRole('region', { name: 'Permission request from the session' })).toBeInTheDocument()
    view = appSession({ state: 'working', items: [user('x'), toolPrompt({ status: 'answered' })] })
    fireEvent.click(screen.getByRole('button', { name: 'Allow' }))
    await waitFor(() => expect(screen.queryByRole('region', { name: /Permission request/ })).toBeNull())
    expect(answered(server)[0].body).toEqual({ behavior: 'allow', prompt_id: 'p1', confirmed: true })
    expect(within(log()).getByText('Permission request · answered')).toBeInTheDocument()
  })

  it('the card stays when nothing answered it: a pending item is drawn on every read', async () => {
    install({ sessions: [appSession({ state: 'waiting', items: [toolPrompt()] })] })
    await open()
    expect(screen.getByRole('button', { name: 'Allow' })).toBeEnabled()
    expect(within(panel()).getByRole('status')).toHaveTextContent('Waiting for you')
  })

  it('Deny sends the note', async () => {
    const server = install({ sessions: [appSession({ state: 'waiting', items: [toolPrompt()] })], routes: { 'POST /api/app-sessions/a1/answer': () => appSession() } })
    await open()
    fireEvent.change(screen.getByLabelText(/Note to Claude/), { target: { value: 'use the tests instead' } })
    fireEvent.click(screen.getByRole('button', { name: 'Deny' }))
    await waitFor(() => expect(answered(server)).toHaveLength(1))
    expect(answered(server)[0].body).toEqual({ behavior: 'deny', message: 'use the tests instead', prompt_id: 'p1', confirmed: true })
  })

  it('a cut input (complete false) and a card of the earlier summary shape have no Allow', async () => {
    install({ sessions: [appSession({ state: 'waiting', items: [toolPrompt({ complete: false, input: 'x'.repeat(30) }), toolPrompt({ prompt_id: 'p7', input: undefined, complete: undefined, summary: 'ls' })] })] })
    await open()
    expect(screen.queryByRole('button', { name: 'Allow' })).toBeNull()
    expect(screen.getAllByRole('button', { name: 'Deny' })).toHaveLength(2)
  })

  it('a question: one choice per option plus Other; the answers go keyed by the question text', async () => {
    const server = install({ sessions: [appSession({ state: 'waiting', items: [questionPrompt()] })], routes: { 'POST /api/app-sessions/a1/answer': () => appSession() } })
    await open()
    fireEvent.click(screen.getByRole('radio', { name: /Blue/ }))
    fireEvent.click(screen.getByRole('button', { name: 'Send answer' }))
    await waitFor(() => expect(answered(server)).toHaveLength(1))
    expect(answered(server)[0].body).toEqual({ behavior: 'allow', updatedInput: { answers: { 'Which colour?': 'Blue' } }, prompt_id: 'p2', confirmed: true })
  })

  it('a question answered with Other sends the owner\'s own text', async () => {
    const server = install({ sessions: [appSession({ state: 'waiting', items: [questionPrompt()] })], routes: { 'POST /api/app-sessions/a1/answer': () => appSession() } })
    await open()
    fireEvent.click(screen.getByRole('radio', { name: /Other/ }))
    fireEvent.change(screen.getByLabelText('Your own answer to question 1'), { target: { value: 'Green' } })
    fireEvent.click(screen.getByRole('button', { name: 'Send answer' }))
    await waitFor(() => expect(answered(server)).toHaveLength(1))
    expect(answered(server)[0].body.updatedInput).toEqual({ answers: { 'Which colour?': 'Green' } })
  })

  it('answerBody joins a multi-select by ", " and puts Other last', () => {
    const prompt = readPrompt(questionPrompt({ questions: [{ question: 'Pick', header: '', multiSelect: true, options: [{ label: 'A', description: '' }, { label: 'B', description: '' }] }] }))
    expect(answerBody(prompt, { answers: [{ selected: [0, 1], other: 'C' }] })).toEqual({ behavior: 'allow', updatedInput: { answers: { Pick: 'A, B, C' } } })
  })

  it('a plan: Approve allows, Keep planning denies with the note', async () => {
    const server = install({ sessions: [appSession({ state: 'waiting', items: [planPrompt()] })], routes: { 'POST /api/app-sessions/a1/answer': () => appSession({ state: 'waiting', items: [planPrompt()] }) } })
    await open()
    expect(within(panel()).getByText('1. do it')).toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: 'Approve' }))
    await waitFor(() => expect(answered(server)).toHaveLength(1))
    expect(answered(server)[0].body).toEqual({ behavior: 'allow', prompt_id: 'p3', confirmed: true })
    fireEvent.change(screen.getByLabelText(/Note to Claude/), { target: { value: 'smaller steps' } })
    fireEvent.click(screen.getByRole('button', { name: 'Keep planning' }))
    await waitFor(() => expect(answered(server)).toHaveLength(2))
    expect(answered(server)[1].body).toEqual({ behavior: 'deny', message: 'smaller steps', prompt_id: 'p3', confirmed: true })
  })

  it('a prompt answered elsewhere (409): the card stays with the server\'s words and is not drawn twice', async () => {
    install({ sessions: [appSession({ state: 'waiting', items: [toolPrompt()] })], routes: { 'POST /api/app-sessions/a1/answer': () => ({ status: 409, body: { error: 'that prompt was answered or closed already' } }) } })
    await open()
    fireEvent.click(screen.getByRole('button', { name: 'Allow' }))
    expect(await screen.findByRole('alert')).toHaveTextContent('that prompt was answered or closed already')
    expect(screen.getAllByRole('region', { name: /Permission request/ })).toHaveLength(1)
    expect(screen.getByRole('button', { name: 'Allow' })).toBeEnabled()
  })

  it('a double click sends one answer', async () => {
    const server = install({ sessions: [appSession({ state: 'waiting', items: [toolPrompt()] })], routes: { 'POST /api/app-sessions/a1/answer': () => appSession() } })
    await open()
    const allow = screen.getByRole('button', { name: 'Allow' })
    fireEvent.click(allow)
    fireEvent.click(allow)
    await waitFor(() => expect(answered(server)).toHaveLength(1))
  })

  it('a settled card is one line: answered, or closed when the turn ended', async () => {
    install({ sessions: [appSession({ items: [toolPrompt({ status: 'cancelled' }), questionPrompt({ status: 'answered' }), planPrompt({ prompt_id: 'p4', status: 'answered' })] })] })
    await open()
    expect(within(log()).getByText('Permission request · closed (the turn or the session ended)')).toBeInTheDocument()
    expect(within(log()).getByText('Question · answered')).toBeInTheDocument()
    expect(within(log()).getByText('Plan · answered')).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Allow' })).toBeNull()
  })

  it('a pending prompt the page cannot read is said, not hidden', async () => {
    install({ sessions: [appSession({ state: 'waiting', items: [{ kind: 'prompt', status: 'pending', type: 'weird' }] })] })
    await open()
    expect(within(panel()).getByText(/prompt this page cannot show is waiting/)).toBeInTheDocument()
  })

  it('a new prompt starts with a clean note; the old one\'s note does not carry over (keyed by prompt id)', async () => {
    vi.useFakeTimers()
    const server = install({ sessions: [appSession({ state: 'waiting', items: [toolPrompt()] })] })
    await mountFake()
    await act(async () => {
      await vi.advanceTimersByTimeAsync(0)
    })
    fireEvent.change(screen.getByLabelText(/Note to Claude/), { target: { value: 'typed for p1' } })
    server.state.sessions = [appSession({ state: 'waiting', items: [toolPrompt({ status: 'answered' }), toolPrompt({ prompt_id: 'p5', input: 'pwd' })] })]
    await act(async () => {
      await vi.advanceTimersByTimeAsync(APPS_FAST_MS)
    })
    expect(within(panel()).getByText('pwd')).toBeInTheDocument()
    expect(screen.getByLabelText(/Note to Claude/)).toHaveValue('')
  })

  it('a pending card is in page flow below the conversation log, not inside the scroll box; the log keeps a one-line entry', async () => {
    install({ sessions: [appSession({ state: 'waiting', items: [user('x'), toolPrompt({ input: 'a long command '.repeat(40) })] })] })
    await open()
    const card = screen.getByRole('region', { name: 'Permission request from the session' })
    expect(panel().contains(card)).toBe(true)
    expect(log().contains(card)).toBe(false)
    expect(log().contains(screen.getByRole('button', { name: 'Allow' }))).toBe(false)
    expect(within(log()).getByText('Permission request \u00b7 waiting for you, below the conversation')).toBeInTheDocument()
  })

  it('an unreadable pending prompt is also said outside the log', async () => {
    install({ sessions: [appSession({ state: 'waiting', items: [{ kind: 'prompt', status: 'pending', type: 'weird' }] })] })
    await open()
    expect(log().contains(screen.getByText(/prompt this page cannot show is waiting/))).toBe(false)
  })

  it('a plan with no `complete` has no Approve; Keep planning is still offered', async () => {
    install({ sessions: [appSession({ state: 'waiting', items: [planPrompt({ complete: undefined })] })] })
    await open()
    expect(screen.queryByRole('button', { name: 'Approve' })).toBeNull()
    expect(screen.getByRole('button', { name: 'Keep planning' })).toBeEnabled()
  })

  it('a poll that was out when an answer came back does not bring the answered card back', async () => {
    vi.useFakeTimers()
    const pending = appSession({ state: 'waiting', items: [toolPrompt()] })
    const server = install({
      sessions: [pending],
      routes: { 'POST /api/app-sessions/a1/answer': () => appSession({ state: 'working', items: [toolPrompt({ status: 'answered' })] }) },
    })
    const held = hold(server, (c) => c.method === 'GET' && c.path === '/api/app-sessions')
    await mountFake()
    await act(async () => {
      await vi.advanceTimersByTimeAsync(0)
    })
    held.on = true // the next read is slow, and it saw the prompt still pending
    await act(async () => {
      await vi.advanceTimersByTimeAsync(APPS_FAST_MS)
    })
    fireEvent.click(screen.getByRole('button', { name: 'Allow' }))
    await act(async () => {
      await vi.advanceTimersByTimeAsync(0)
    })
    expect(screen.queryByRole('button', { name: 'Allow' })).toBeNull()
    await act(async () => {
      held.release()
      await vi.advanceTimersByTimeAsync(0)
    })
    expect(screen.queryByRole('button', { name: 'Allow' })).toBeNull()
    expect(within(log()).getByText('Permission request \u00b7 answered')).toBeInTheDocument()
  })
})

describe('the app sessions in the list (FR-128 AC 1)', () => {
  it('app sessions are rows of the list; picking one shows its panel, picking another switches', async () => {
    install({ sessions: [appSession({ id: 'a1', folder: '/work/one', items: [assistant('one text')] }), appSession({ id: 'a2', folder: '/work/two', state: 'working', items: [assistant('two text')] })] })
    mount()
    await waitFor(() => expect(rowOf('a2')).not.toBeNull())
    expect(screen.queryByRole('region', { name: /^Session in / })).toBeNull() // nothing is open until one is picked
    fireEvent.click(rowOf('a1'))
    expect(await within(log()).findByText('one text')).toBeInTheDocument()
    fireEvent.click(rowOf('a2'))
    expect(await within(log()).findByText('two text')).toBeInTheDocument()
    expect(within(log()).queryByText('one text')).toBeNull()
    expect(rowOf('a2')).toHaveAttribute('aria-current', 'true')
  })

  it('an app session the live tail lists is one row, not two: the live row opens the app session panel', async () => {
    install({ sessions: [appSession({ id: 'a1', session_id: 'abcdef1234567890', items: [] })] })
    render(<LiveView appSessions />)
    stream().emit('snapshot', snapshot([{ session_id: 'abcdef1234567890', account: 'Claude work', project: 'alpha', last_ts: new Date().toISOString(), tool_calls: 3, tokens_in: 10, tokens_out: 5, cost_usd: 0.5 }]))
    await waitFor(() => expect(rowOf('a1')).not.toBeNull())
    expect(document.querySelectorAll('.live-row')).toHaveLength(1)
    fireEvent.click(rowOf('a1'))
    expect(await screen.findByRole('button', { name: 'Interrupt' })).toBeInTheDocument()
    expect(screen.getByLabelText('Message to Claude')).toBeInTheDocument()
  })

  it('a session that was just started (the wish from the App sessions page) opens its panel once it is listed, and the wish is used once', async () => {
    install({ sessions: [appSession({ id: 'a1', folder: '/work/one' }), appSession({ id: 'a2', folder: '/work/two', items: [assistant('the second')] })] })
    const used = vi.fn()
    mount({ wantedApp: 'a2', onWantedAppUsed: used })
    expect(await within(await screen.findByRole('log', { name: 'Conversation' })).findByText('the second')).toBeInTheDocument()
    expect(used).toHaveBeenCalledTimes(1)
  })
})

describe('what the page never does (FR-115, FR-116)', () => {
  it('no URL it builds carries the token; the token is only in the header', async () => {
    const server = install({
      sessions: [appSession({ state: 'waiting', items: [toolPrompt()] })],
      routes: { 'POST /api/app-sessions/a1/answer': () => appSession(), 'POST /api/app-sessions/a1/message': () => appSession(), 'POST /api/app-sessions/a1/stop': () => appSession() },
    })
    await open()
    fireEvent.click(screen.getByRole('button', { name: 'Allow' }))
    await waitFor(() => expect(server.posts('/answer')).toHaveLength(1))
    fireEvent.change(screen.getByLabelText('Message to Claude'), { target: { value: 'hi' } })
    fireEvent.keyDown(screen.getByLabelText('Message to Claude'), { key: 'Enter' })
    fireEvent.click(screen.getByRole('button', { name: 'Stop' }))
    await waitFor(() => expect(server.posts('/stop')).toHaveLength(1))
    expect(server.calls.length).toBeGreaterThan(3)
    for (const call of server.calls) {
      expect(call.url).not.toMatch(/token|secret/i)
      if (call.path.startsWith('/api/app-sessions')) expect(call.url).not.toContain('?')
      expect(call.init.headers['X-Dashboard-Token']).toBe('secret-token')
    }
    expect(server.calls.filter((c) => c.method === 'POST').every((c) => c.body.confirmed === true)).toBe(true)
  })

  it('writes no text to localStorage', async () => {
    const set = vi.spyOn(Storage.prototype, 'setItem')
    install({ sessions: [appSession({ items: [assistant('kept in memory only')] })], routes: { 'POST /api/app-sessions/a1/message': () => appSession() } })
    await open()
    fireEvent.change(screen.getByLabelText('Message to Claude'), { target: { value: 'private words' } })
    fireEvent.keyDown(screen.getByLabelText('Message to Claude'), { key: 'Enter' })
    await waitFor(() => expect(screen.getByLabelText('Message to Claude')).toHaveValue(''))
    expect(set).not.toHaveBeenCalled()
    expect(window.localStorage.length).toBe(0)
    set.mockRestore()
  })

  it('the page names no program and no free argument: every write holds only the documented fields', async () => {
    const server = install({ sessions: [appSession({ state: 'working' })], routes: { 'POST /api/app-sessions/a1/interrupt': () => appSession() } })
    await open()
    fireEvent.click(screen.getByRole('button', { name: 'Interrupt' }))
    await waitFor(() => expect(server.posts('/interrupt')).toHaveLength(1))
    expect(Object.keys(server.posts('/interrupt')[0].body)).toEqual(['confirmed'])
  })
})
