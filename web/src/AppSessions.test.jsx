import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import AppSessions, { NO_CLAUDE, OFF } from './AppSessions.jsx'
import { agentRunOff, agentRunOn } from './test/fixtures.js'
import { appServer, appSession, user } from './test/appSessions.js'

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
  render(<AppSessions {...props} />)
  await screen.findByRole('heading', { name: 'App sessions' })
  await waitFor(() => expect(screen.queryByText('Loading app sessions')).toBeNull())
}
describe('availability', () => {
  it('says so, and offers nothing, when the server has no app session route (the Python server)', async () => {
    install({ list: null })
    await open()
    expect(screen.getByText('App sessions are not available in this version of the app.')).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'New session' })).toBeNull()
  })

  it('off until an agent run account is chosen: the reason, a link to Settings, and New session is off', async () => {
    install({ agentRun: agentRunOff })
    const onOpenSettings = vi.fn()
    await open({ onOpenSettings })
    expect(screen.getByText(OFF, { exact: false })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'New session' })).toBeDisabled()
    fireEvent.click(screen.getByRole('button', { name: 'Open Settings' }))
    expect(onOpenSettings).toHaveBeenCalled()
  })

  it('says when Claude Code is not found', async () => {
    install({ agentRun: agentRunOn({ claude_found: false }) })
    await open()
    expect(screen.getByRole('alert')).toHaveTextContent(NO_CLAUDE)
    expect(screen.getByRole('button', { name: 'New session' })).toBeDisabled()
  })

  it('with no session it says how to start one', async () => {
    install()
    await open()
    expect(screen.getByText(/Start a session with New session/)).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'New session' })).toBeEnabled()
  })

  it('a load failure is an alert', async () => {
    vi.stubGlobal('fetch', async () => ({ ok: false, status: 500, json: async () => ({}) }))
    render(<AppSessions />)
    expect(await screen.findByRole('alert')).toHaveTextContent('Could not load app sessions')
  })

  it('at the limit New session is off and says why', async () => {
    install({ max: 2, sessions: [appSession({ id: 'a1' }), appSession({ id: 'a2' }), appSession({ id: 'a3', state: 'ended', reason: 'stopped' })] })
    await open()
    expect(screen.getByRole('button', { name: 'New session' })).toBeDisabled()
    expect(screen.getByText('2 sessions are going; stop one first.')).toBeInTheDocument()
  })
})

describe('starting a session (FR-128 AC 1)', () => {
  it('the page lists no session and shows no panel: sessions are in Live', async () => {
    install({ sessions: [appSession({ id: 'a1', items: [user('secret')] })] })
    await open()
    expect(screen.queryByRole('region', { name: /^Session in / })).toBeNull()
    expect(screen.queryByRole('navigation', { name: 'App sessions' })).toBeNull()
    expect(screen.queryByText('secret')).toBeNull()
  })

  it('New session -> form -> Review -> Start posts the form and takes the new session to Live', async () => {
    const fresh = appSession({ id: 'a7', folder: '/work/company', state: 'working', items: [user('fix it')] })
    const onOpenAppSession = vi.fn()
    const server = install({
      routes: {
        'POST /api/app-sessions': (_body, state) => {
          state.sessions = [fresh]
          return { status: 202, body: { id: 'a7' } }
        },
      },
    })
    await open({ onOpenAppSession })
    fireEvent.click(screen.getByRole('button', { name: 'New session' }))
    expect(screen.getByRole('heading', { name: 'New session' })).toBeInTheDocument()
    fireEvent.change(screen.getByLabelText('First prompt'), { target: { value: 'fix it' } })
    fireEvent.click(screen.getByRole('button', { name: 'Review' }))
    fireEvent.click(screen.getByRole('button', { name: 'Start session' }))
    await waitFor(() => expect(onOpenAppSession).toHaveBeenCalledWith('a7'))
    expect(screen.queryByRole('heading', { name: 'New session' })).toBeNull()
    expect(server.posts('/api/app-sessions')[0].body).toMatchObject({ repo: '/work/company', workspace: 'company', account: '2', confirmed: true })
  })

  it('opened for a workspace (the Workspaces page) the form starts there, and the wish is used once', async () => {
    install()
    const used = vi.fn()
    await open({ target: { newFor: 'home' }, onTargetUsed: used })
    expect(screen.getByLabelText('Repo')).toHaveDisplayValue('/home/o/notes')
    expect(used).toHaveBeenCalledTimes(1)
  })
})
