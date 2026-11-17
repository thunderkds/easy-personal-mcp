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
