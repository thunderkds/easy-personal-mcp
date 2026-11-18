import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import App from './App.jsx'
import LiveView from './LiveView.jsx'
import { ToastProvider } from './Toast.jsx'
import WorkspacesView from './WorkspacesView.jsx'
import { agentRunOn, fakeFetch, workspaceOverrides } from './test/fixtures.js'
import { appServer, appSession } from './test/appSessions.js'

// The two entry points of the app session page (T104): a workspace card on Workspaces, and an app session's row on Live.

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
    act(() => (this.listeners[type] ?? []).forEach((fn) => fn({ type, data: data === undefined ? undefined : JSON.stringify(data), lastEventId: '' })))
  }
}
const row = (id, over = {}) => ({ session_id: id, account: 'Claude work', project: 'alpha', last_ts: new Date().toISOString(), tool_calls: 0, tokens_in: 0, tokens_out: 0, cost_usd: 0, ...over })

/** `fakeFetch` for everything, the app session stub for its own routes. */
function both(server, overrides = {}) {
  const base = fakeFetch({ agentRun: agentRunOn(), ...overrides })
  const fn = (url, init) => (new URL(String(url), 'http://127.0.0.1').pathname.startsWith('/api/app-sessions') ? server(url, init) : base(url, init))
  fn.calls = base.calls
  return fn
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

describe('Workspaces: New session', () => {
  it('a workspace with repos has New session; one without has none; no handler, no button', async () => {
    vi.stubGlobal('fetch', fakeFetch(workspaceOverrides()))
    const onNewSession = vi.fn()
    const { unmount } = render(<ToastProvider><WorkspacesView onNewSession={onNewSession} /></ToastProvider>)
    const company = await screen.findByRole('region', { name: 'company' })
    expect(within(screen.getByRole('region', { name: 'side' })).queryByRole('button', { name: 'New session' })).toBeNull()
    fireEvent.click(within(company).getByRole('button', { name: 'New session' }))
    expect(onNewSession).toHaveBeenCalledWith('company')
    unmount()
    render(<ToastProvider><WorkspacesView /></ToastProvider>)
    await screen.findByRole('region', { name: 'company' })
    expect(screen.queryByRole('button', { name: 'New session' })).toBeNull()
  })

  it('in the app: the button opens the App sessions page with the New session form on that workspace', async () => {
    vi.stubGlobal('fetch', both(appServer(), workspaceOverrides({ workspaces: { workspaces: [{ id: 2, name: 'company', is_orphan: false, repos: ['/work/company', '/work/company-web'] }, { id: 3, name: 'home', is_orphan: false, repos: ['/home/o/notes'] }] } })))
    render(<App />)
    const nav = screen.getByRole('navigation', { name: 'Views' })
    fireEvent.click(within(nav).getByRole('button', { name: 'Workspaces' }))
    const home = await screen.findByRole('region', { name: 'home' })
    fireEvent.click(within(home).getByRole('button', { name: 'New session' }))
    expect(await screen.findByRole('heading', { name: 'App sessions' })).toBeInTheDocument()
    expect(within(nav).getByRole('button', { name: 'App sessions' })).toHaveAttribute('aria-current', 'page')
    expect(await screen.findByLabelText('Repo')).toHaveDisplayValue('/home/o/notes')
  })

  it('the sidebar has App sessions under // work, and opening it by hand shows the page', async () => {
    vi.stubGlobal('fetch', both(appServer({ sessions: [appSession({ items: [] })] })))
    render(<App />)
    const nav = screen.getByRole('navigation', { name: 'Views' })
    fireEvent.click(within(nav).getByRole('button', { name: 'App sessions' }))
    expect(await screen.findByRole('region', { name: /^Session in / })).toBeInTheDocument()
    expect(screen.queryByLabelText('Repo')).toBeNull() // no form unless asked for
  })
})

describe('Live: Open in app', () => {
  const stream = () => FakeEventSource.instances.at(-1)
  const snapshot = (sessions) => ({ active_window_s: 300, last_id: 0, sessions })

  it('an app session\'s row has Open in app; another row has none', async () => {
    vi.stubGlobal('fetch', appServer({ sessions: [appSession({ id: 'a1', session_id: 'abcdef1234567890' })] }))
    const onOpen = vi.fn()
    render(<LiveView onOpenAppSession={onOpen} />)
    stream().emit('snapshot', snapshot([row('abcdef1234567890'), row('99999999-terminal')]))
    const mine = screen.getByText('abcdef12', { selector: '.live-id' }).closest('li')
    const other = screen.getByText('99999999', { selector: '.live-id' }).closest('li')
    const button = await within(mine).findByRole('button', { name: 'Open in app' })
    expect(within(other).queryByRole('button', { name: 'Open in app' })).toBeNull()
    fireEvent.click(button)
    expect(onOpen).toHaveBeenCalledWith('a1')
  })

  it('a row whose app session got its Claude session id later gets Open in app on a later read (at most one read per 5 s)', async () => {
    vi.useFakeTimers()
    const server = appServer({ sessions: [appSession({ id: 'a1', session_id: null })] })
    vi.stubGlobal('fetch', server)
    render(<LiveView onOpenAppSession={() => {}} />)
    stream().emit('snapshot', snapshot([row('abcdef1234567890')]))
    await act(async () => {
      await vi.advanceTimersByTimeAsync(0)
    })
    const reads = () => server.calls.filter((c) => c.path === '/api/app-sessions').length
    const first = reads() // the empty list, then the snapshot's set of rows: one read each
    expect(screen.queryByRole('button', { name: 'Open in app' })).toBeNull()
    server.state.sessions = [appSession({ id: 'a1', session_id: 'abcdef1234567890' })]
    await act(async () => {
      await vi.advanceTimersByTimeAsync(4_900)
    })
    expect(reads()).toBe(first) // not before 5 s
    await act(async () => {
      await vi.advanceTimersByTimeAsync(200)
    })
    expect(reads()).toBe(first + 1)
    expect(screen.getByRole('button', { name: 'Open in app' })).toBeInTheDocument()
    await act(async () => {
      await vi.advanceTimersByTimeAsync(30_000)
    })
    expect(reads()).toBe(first + 1) // every live row is known: no more reads
  })

  it('a terminal row that is no app session costs at most one read per 5 s', async () => {
    vi.useFakeTimers()
    const server = appServer({ sessions: [] })
    vi.stubGlobal('fetch', server)
    render(<LiveView onOpenAppSession={() => {}} />)
    stream().emit('snapshot', snapshot([row('99999999-terminal')]))
    await act(async () => {
      await vi.advanceTimersByTimeAsync(20_000)
    })
    expect(server.calls.filter((c) => c.path === '/api/app-sessions').length).toBeLessThanOrEqual(2 + 4) // the two reads of the row set changing, then one per 5 s
  })

  it('a click on Open in app does not open the row (it is above the row\'s click area)', async () => {
    vi.stubGlobal('fetch', appServer({ sessions: [appSession({ id: 'a1', session_id: 'abcdef1234567890' })] }))
    render(<LiveView onOpenAppSession={() => {}} />)
    stream().emit('snapshot', snapshot([row('abcdef1234567890')]))
    const mine = screen.getByText('abcdef12', { selector: '.live-id' }).closest('li')
    fireEvent.click(await within(mine).findByRole('button', { name: 'Open in app' }))
    expect(mine.querySelector('button.live-row')).toHaveAttribute('aria-expanded', 'false')
  })

  it('with no way to open one, Live asks the server for no app session', async () => {
    const server = appServer({ sessions: [appSession({ session_id: 'abcdef1234567890' })] })
    vi.stubGlobal('fetch', server)
    render(<LiveView />)
    stream().emit('snapshot', snapshot([row('abcdef1234567890')]))
    await screen.findByText('abcdef12', { selector: '.live-id' })
    expect(server.calls).toHaveLength(0)
  })

  it('a server with no app session route leaves Live as it was', async () => {
    vi.stubGlobal('fetch', appServer({ list: null }))
    render(<LiveView onOpenAppSession={() => {}} />)
    stream().emit('snapshot', snapshot([row('abcdef1234567890')]))
    await screen.findByText('abcdef12', { selector: '.live-id' })
    await waitFor(() => expect(fetch.calls.length).toBeGreaterThan(0))
    expect(screen.queryByRole('button', { name: 'Open in app' })).toBeNull()
  })

  it('in the app: Open in app goes to the App sessions page with that session shown', async () => {
    const server = appServer({ sessions: [appSession({ id: 'a1', session_id: 'abcdef1234567890', folder: '/work/one' }), appSession({ id: 'a2', session_id: 'zzzz', folder: '/work/two', items: [{ kind: 'assistant', text: 'second session text' }] })] })
    vi.stubGlobal('fetch', both(server))
    render(<App />)
    const nav = screen.getByRole('navigation', { name: 'Views' })
    fireEvent.click(within(nav).getByRole('button', { name: 'Live' }))
    await waitFor(() => expect(stream()).toBeDefined())
    stream().emit('snapshot', snapshot([row('zzzz')]))
    fireEvent.click(await screen.findByRole('button', { name: 'Open in app' }))
    expect(await screen.findByText('second session text')).toBeInTheDocument()
    expect(within(nav).getByRole('button', { name: 'App sessions' })).toHaveAttribute('aria-current', 'page')
  })
})

// T105 (FR-112): Resume in app / Fork in app on a Live row, Continue in app on a Workspaces repo.
describe('Live: Resume in app and Fork in app', () => {
  const stream = () => FakeEventSource.instances.at(-1)
  const snapshot = (sessions) => ({ active_window_s: 600, last_id: 0, sessions })
  const ago = (seconds) => new Date(Date.now() - seconds * 1000).toISOString()
  const FORK = 'The open window does not see a fork: it becomes a new session.'
  const li = (id) => screen.getByText(id.slice(0, 8), { selector: '.live-id' }).closest('li')

  it('a quiet session (no record for 2 minutes) has Resume in app and no note', async () => {
    vi.stubGlobal('fetch', appServer())
    const onResume = vi.fn()
    render(<LiveView onResumeInApp={onResume} />)
    stream().emit('snapshot', snapshot([row('99999999-terminal', { last_ts: ago(121), account: 'Claude work', project: 'alpha' })]))
    const button = await within(li('99999999-terminal')).findByRole('button', { name: 'Resume in app' })
    expect(within(li('99999999-terminal')).queryByRole('button', { name: 'Fork in app' })).toBeNull()
    expect(screen.queryByText(FORK)).toBeNull()
    fireEvent.click(button)
    expect(onResume).toHaveBeenCalledWith({ id: '99999999-terminal', project: 'alpha', account: 'Claude work', fork: false })
  })

  it('a session with a record in the last 2 minutes has Fork in app and the fixed note', async () => {
    vi.stubGlobal('fetch', appServer())
    const onResume = vi.fn()
    render(<LiveView onResumeInApp={onResume} />)
    stream().emit('snapshot', snapshot([row('88888888-terminal', { last_ts: ago(119) })]))
    const mine = li('88888888-terminal')
    const button = await within(mine).findByRole('button', { name: 'Fork in app' })
    expect(within(mine).queryByRole('button', { name: 'Resume in app' })).toBeNull()
    expect(within(mine).getByText(FORK)).toBeInTheDocument()
    fireEvent.click(button)
    expect(onResume).toHaveBeenCalledWith({ id: '88888888-terminal', project: 'alpha', account: 'Claude work', fork: true })
  })

  it('the button follows the clock: a row that was active is a Resume once 2 minutes have passed', async () => {
    vi.useFakeTimers()
    vi.stubGlobal('fetch', appServer())
    render(<LiveView onResumeInApp={() => {}} />)
    stream().emit('snapshot', snapshot([row('77777777-terminal', { last_ts: ago(100) })]))
    await act(async () => {
      await vi.advanceTimersByTimeAsync(0)
    })
    expect(screen.getByRole('button', { name: 'Fork in app' })).toBeInTheDocument()
    await act(async () => {
      await vi.advanceTimersByTimeAsync(30_000) // the page works out "now" again every 15 s
    })
    expect(screen.getByRole('button', { name: 'Resume in app' })).toBeInTheDocument()
    expect(screen.queryByText(FORK)).toBeNull()
  })

  it('an app session\'s row has Open in app and neither of these; no handler or no app session route: no button', async () => {
    vi.stubGlobal('fetch', appServer({ sessions: [appSession({ id: 'a1', session_id: 'abcdef1234567890' })] }))
    const { unmount } = render(<LiveView onResumeInApp={() => {}} onOpenAppSession={() => {}} />)
    stream().emit('snapshot', snapshot([row('abcdef1234567890', { last_ts: ago(300) })]))
    await within(li('abcdef1234567890')).findByRole('button', { name: 'Open in app' })
    expect(screen.queryByRole('button', { name: /^(Resume|Fork) in app$/ })).toBeNull()
    unmount()
    vi.stubGlobal('fetch', appServer({ list: null }))
    render(<LiveView onResumeInApp={() => {}} />)
    stream().emit('snapshot', snapshot([row('66666666-terminal', { last_ts: ago(300) })]))
    await screen.findByText('66666666', { selector: '.live-id' })
    await waitFor(() => expect(fetch.calls.length).toBeGreaterThan(0))
    expect(screen.queryByRole('button', { name: /in app$/ })).toBeNull()
  })

  it('before the app sessions are known a row has no button, so an app session never flashes Resume', async () => {
    let release
    const held = new Promise((resolve) => {
      release = resolve
    })
    const server = appServer({ sessions: [appSession({ id: 'a1', session_id: 'abcdef1234567890' })] })
    vi.stubGlobal('fetch', async (url, init) => {
      await held
      return server(url, init)
    })
    render(<LiveView onResumeInApp={() => {}} onOpenAppSession={() => {}} />)
    stream().emit('snapshot', snapshot([row('abcdef1234567890', { last_ts: ago(300) })]))
    await screen.findByText('abcdef12', { selector: '.live-id' })
    expect(screen.queryByRole('button', { name: /in app$/ })).toBeNull()
    release()
    expect(await screen.findByRole('button', { name: 'Open in app' })).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: /^(Resume|Fork) in app$/ })).toBeNull()
  })

  it('a click on the button does not open the row', async () => {
    vi.stubGlobal('fetch', appServer())
    render(<LiveView onResumeInApp={() => {}} />)
    stream().emit('snapshot', snapshot([row('99999999-terminal', { last_ts: ago(300) })]))
    fireEvent.click(await within(li('99999999-terminal')).findByRole('button', { name: 'Resume in app' }))
    expect(li('99999999-terminal').querySelector('button.live-row')).toHaveAttribute('aria-expanded', 'false')
  })

  it('in the app: Resume in app opens the App sessions page with the Resume form for that session, named by short id and project', async () => {
    const server = appServer()
    vi.stubGlobal('fetch', both(server))
    render(<App />)
    const nav = screen.getByRole('navigation', { name: 'Views' })
    fireEvent.click(within(nav).getByRole('button', { name: 'Live' }))
    await waitFor(() => expect(stream()).toBeDefined())
    stream().emit('snapshot', snapshot([row('99999999-terminal', { last_ts: ago(300), project: 'alpha' })]))
    fireEvent.click(await screen.findByRole('button', { name: 'Resume in app' }))
    expect(await screen.findByRole('heading', { name: 'Resume session' })).toBeInTheDocument()
    expect(within(nav).getByRole('button', { name: 'App sessions' })).toHaveAttribute('aria-current', 'page')
    expect(screen.getByText('99999999')).toBeInTheDocument()
    expect(screen.queryByLabelText('Repo')).toBeNull()
    fireEvent.change(screen.getByLabelText('First prompt'), { target: { value: 'carry on' } })
    fireEvent.click(screen.getByRole('button', { name: 'Review' }))
    fireEvent.click(screen.getByRole('button', { name: 'Start session' }))
    await waitFor(() => expect(server.posts('/api/app-sessions')).toHaveLength(1))
    expect(server.posts('/api/app-sessions')[0].body).toMatchObject({ resume: '99999999-terminal', confirmed: true })
    expect(server.posts('/api/app-sessions')[0].body).not.toHaveProperty('repo')
  })

  it('in the app: Fork in app opens the Fork form with the note', async () => {
    vi.stubGlobal('fetch', both(appServer()))
    render(<App />)
    fireEvent.click(within(screen.getByRole('navigation', { name: 'Views' })).getByRole('button', { name: 'Live' }))
    await waitFor(() => expect(stream()).toBeDefined())
    stream().emit('snapshot', snapshot([row('88888888-terminal', { last_ts: ago(10) })]))
    fireEvent.click(await screen.findByRole('button', { name: 'Fork in app' }))
    expect(await screen.findByRole('heading', { name: 'Fork session' })).toBeInTheDocument()
    expect(screen.getByText(FORK, { selector: '.prompt-cut' })).toBeInTheDocument()
  })
})

describe('Workspaces: Continue in app', () => {
  it('each repo of a workspace has Continue in app; no handler, no button; the click names the workspace and the repo', async () => {
    vi.stubGlobal('fetch', fakeFetch(workspaceOverrides()))
    const onContinue = vi.fn()
    const { unmount } = render(<ToastProvider><WorkspacesView onContinueInApp={onContinue} /></ToastProvider>)
    const company = await screen.findByRole('region', { name: 'company' })
    const buttons = within(company).getAllByRole('button', { name: /^Continue in app/ })
    expect(buttons.map((b) => b.textContent)).toEqual(['Continue in app', 'Continue in app'])
    fireEvent.click(within(company).getByRole('button', { name: 'Continue in app: /work/company-web' }))
    expect(onContinue).toHaveBeenCalledWith('company', '/work/company-web')
    expect(within(screen.getByRole('region', { name: 'side' })).queryByRole('button', { name: /Continue in app/ })).toBeNull()
    unmount()
    render(<ToastProvider><WorkspacesView /></ToastProvider>)
    await screen.findByRole('region', { name: 'company' })
    expect(screen.queryByRole('button', { name: /Continue in app/ })).toBeNull()
  })

  it('in the app: Continue in app opens the Continue form for that repo and starts with continue: true', async () => {
    const server = appServer()
    vi.stubGlobal('fetch', both(server, workspaceOverrides({ workspaces: { workspaces: [{ id: 2, name: 'company', is_orphan: false, repos: ['/work/company', '/work/company-web'] }] } })))
    render(<App />)
    const nav = screen.getByRole('navigation', { name: 'Views' })
    fireEvent.click(within(nav).getByRole('button', { name: 'Workspaces' }))
    const company = await screen.findByRole('region', { name: 'company' })
    fireEvent.click(within(company).getByRole('button', { name: 'Continue in app: /work/company-web' }))
    expect(await screen.findByRole('heading', { name: 'Continue session' })).toBeInTheDocument()
    expect(screen.queryByLabelText('Repo')).toBeNull()
    fireEvent.change(screen.getByLabelText('First prompt'), { target: { value: 'go on' } })
    fireEvent.click(screen.getByRole('button', { name: 'Review' }))
    fireEvent.click(screen.getByRole('button', { name: 'Start session' }))
    await waitFor(() => expect(server.posts('/api/app-sessions')).toHaveLength(1))
    expect(server.posts('/api/app-sessions')[0].body).toMatchObject({ continue: true, repo: '/work/company-web', workspace: 'company', confirmed: true })
    expect(server.posts('/api/app-sessions')[0].body).not.toHaveProperty('worktree')
  })
})
