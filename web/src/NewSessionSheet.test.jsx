import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import NewSessionSheet, { NO_CLAUDE, OFF } from './NewSessionSheet.jsx'
import { agentRunOff, agentRunOn } from './test/fixtures.js'
import { appServer, appSession, termSession } from './test/appSessions.js'

// The New session sheet of the Live view (T128, FR-133): the FR-109 form with the reasons it cannot start. It lists no session; Live does.
const install = (opts) => {
  const server = appServer(opts)
  vi.stubGlobal('fetch', server)
  return server
}
beforeEach(() => {
  window.location.hash = '#secret-token'
  window.localStorage.clear()
})
afterEach(() => {
  vi.useRealTimers()
  vi.unstubAllGlobals()
})
const open = async (props = {}) => {
  render(<NewSessionSheet form={{}} onStarted={() => {}} onCancel={() => {}} {...props} />)
  await waitFor(() => expect(screen.queryByText('Loading the form')).toBeNull())
}

describe('availability', () => {
  it('says so, and offers no form, when the server has no app session route (the Python server)', async () => {
    install({ list: null })
    await open()
    expect(screen.getByText('App sessions are not available in this version of the app.')).toBeInTheDocument()
    expect(screen.queryByLabelText('First prompt')).toBeNull()
  })

  it('off until an agent run account is chosen: the reason and a link to Settings, no form', async () => {
    install({ agentRun: agentRunOff })
    const onOpenSettings = vi.fn()
    await open({ onOpenSettings })
    expect(screen.getByText(OFF, { exact: false })).toBeInTheDocument()
    expect(screen.queryByLabelText('First prompt')).toBeNull()
    fireEvent.click(screen.getByRole('button', { name: 'Open Settings' }))
    expect(onOpenSettings).toHaveBeenCalled()
  })

  it('says when Claude Code is not found, and offers no form', async () => {
    install({ agentRun: agentRunOn({ claude_found: false }) })
    await open()
    expect(screen.getByRole('alert')).toHaveTextContent(NO_CLAUDE)
    expect(screen.queryByLabelText('First prompt')).toBeNull()
  })

  it('a load failure is an alert and the sheet can still be closed', async () => {
    vi.stubGlobal('fetch', async () => ({ ok: false, status: 500, json: async () => ({}) }))
    const onCancel = vi.fn()
    await open({ onCancel })
    expect(screen.getByRole('alert')).toHaveTextContent('Could not load the form')
    fireEvent.click(screen.getByRole('button', { name: 'Close' }))
    expect(onCancel).toHaveBeenCalled()
  })

  it('at the limit it says why and offers no form', async () => {
    install({ max: 2, sessions: [appSession({ id: 'a1' }), appSession({ id: 'a2' }), appSession({ id: 'a3', state: 'ended', reason: 'stopped' })] })
    await open()
    expect(screen.getByText('2 sessions are going; stop one first.')).toBeInTheDocument()
    expect(screen.queryByLabelText('First prompt')).toBeNull()
  })
})

describe('the form (FR-109)', () => {
  it('shows the New session form with the Cancel button closing the sheet', async () => {
    install()
    const onCancel = vi.fn()
    await open({ onCancel })
    expect(screen.getByRole('heading', { name: 'New session' })).toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: 'Cancel' }))
    expect(onCancel).toHaveBeenCalled()
  })

  it('Review -> Start posts the form and hands the new id to Live', async () => {
    const onStarted = vi.fn()
    const server = install({
      routes: {
        'POST /api/app-sessions': (_body, state) => {
          state.sessions = [appSession({ id: 'a7', state: 'working' })]
          return { status: 202, body: { id: 'a7' } }
        },
      },
    })
    await open({ onStarted })
    fireEvent.change(screen.getByLabelText('First prompt'), { target: { value: 'fix it' } })
    fireEvent.click(screen.getByRole('button', { name: 'Review' }))
    fireEvent.click(screen.getByRole('button', { name: 'Start session' }))
    await waitFor(() => expect(onStarted).toHaveBeenCalledWith('a7'))
    expect(server.posts('/api/app-sessions')[0].body).toMatchObject({ repo: '/work/company', workspace: 'company', account: '2', confirmed: true })
  })

  it('opened for a workspace the form starts on that workspace\'s repo', async () => {
    install()
    await open({ form: { workspace: 'home' } })
    expect(screen.getByLabelText('Repo')).toHaveDisplayValue('/home/o/notes')
  })

  it('opened to resume a stored session it shows the Resume form with no repo', async () => {
    install()
    await open({ form: { resume: { id: '99999999-terminal', project: 'alpha', account: 'Claude personal', fork: false } } })
    expect(screen.getByRole('heading', { name: 'Resume session' })).toBeInTheDocument()
    expect(screen.queryByLabelText('Repo')).toBeNull()
  })

  it('opened to continue a repo it shows the Continue form', async () => {
    install()
    await open({ form: { continueIn: { workspace: 'company', repo: '/work/company-web' } } })
    expect(screen.getByRole('heading', { name: 'Continue session' })).toBeInTheDocument()
  })
})

// T134 (FR-136): the sheet offers Terminal only when the server has the terminal route, and one cap covers both kinds.
describe('the Terminal choice (T134)', () => {
  it('is offered when the server lists terminal sessions', async () => {
    install({ terms: [] })
    await open()
    expect(screen.getByRole('group', { name: 'Session type' })).toBeInTheDocument()
  })

  it('is not offered by a server with no terminal route (404): the form is Chat only', async () => {
    install()
    await open()
    expect(screen.queryByRole('group', { name: 'Session type' })).toBeNull()
    expect(screen.getByLabelText('First prompt')).toBeInTheDocument()
  })

  it('a failing terminal list (500) leaves the Chat form working, with no Terminal choice', async () => {
    install({ routes: { 'GET /api/term-sessions': () => ({ status: 500, body: {} }) } })
    await open()
    expect(screen.queryByRole('group', { name: 'Session type' })).toBeNull()
    expect(screen.getByLabelText('First prompt')).toBeInTheDocument()
  })

  it('starts a Terminal session and hands its id to Live as a terminal', async () => {
    const server = install({ terms: [], routes: { 'POST /api/term-sessions': () => ({ status: 201, body: { id: 't7' } }) } })
    const onStarted = vi.fn()
    await open({ onStarted })
    fireEvent.click(screen.getByRole('button', { name: 'Terminal' }))
    fireEvent.click(screen.getByRole('button', { name: 'Review' }))
    fireEvent.click(screen.getByRole('button', { name: 'Start session' }))
    await waitFor(() => expect(onStarted).toHaveBeenCalledWith('t7', 'terminal'))
    expect(server.posts('/api/term-sessions')[0].body).toMatchObject({ confirmed: true, repo: '/work/company' })
  })

  it('the one cap counts the running terminal sessions with the app sessions (max 2: one of each is the limit)', async () => {
    install({ max: 2, sessions: [appSession({ id: 'a1' })], terms: [termSession({ id: 't1' }), termSession({ id: 't2', state: 'exited', code: 0 })] })
    await open()
    expect(screen.getByText('2 sessions are going; stop one first.')).toBeInTheDocument()
    expect(screen.queryByLabelText('First prompt')).toBeNull()
  })
})
