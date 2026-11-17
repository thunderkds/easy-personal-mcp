import { act, fireEvent, render, screen, within } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import AgentRun from './AgentRun.jsx'
import { NO_SCOPE } from './Scope.jsx'
import { agentLast, agentRunError, agentRunOff, agentRunOn, fakeFetch } from './test/fixtures.js'

// The Run block of Warnings & plan (T092, FR-104 part 3). Every state comes from a fixture: no `claude` is ever started.
const RUN = /^Run investigate$/
const STOP = /^Stop$/
const writes = (f) => f.calls.filter((c) => c.init.method).map((c) => ({ method: c.init.method, url: c.url, body: JSON.parse(c.init.body) }))
const gets = (f) => f.calls.filter((c) => c.url === '/api/agent-run' && !c.init.method)

/** `answers.get` is the current GET answer (a test changes it); a write answers `answers.post` / `answers.stop` (default: the GET answer). */
function stub(get, answers = {}) {
  const state = { get, ...answers }
  const f = fakeFetch({
    agentRun: (method, path) => {
      if (method === 'GET') return state.get
      const answer = path.endsWith('/stop') ? state.stop : state.post
      return typeof answer === 'function' ? answer() : (answer ?? state.get)
    },
  })
  vi.stubGlobal('fetch', f)
  return { f, state }
}

function open(props = {}) {
  const view = render(<AgentRun scope={NO_SCOPE} onOpenSettings={() => {}} onReportSaved={() => {}} {...props} />)
  return view
}
const ready = () => screen.findByRole('heading', { name: 'Agent run' })

beforeEach(() => {
  window.location.hash = '#secret-token'
  delete window.__pwned
})
afterEach(() => {
  vi.useRealTimers()
  vi.unstubAllGlobals()
})

describe('states from fixtures (AC3)', () => {
  it('off: says so, links to Settings and has no button to run', async () => {
    stub(agentRunOff)
    const onOpenSettings = vi.fn()
    open({ onOpenSettings })
    await ready()
    expect(screen.getByText(/^Agent runs are off\. Choose an account in/)).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: RUN })).toBeNull()
    fireEvent.click(screen.getByRole('button', { name: 'Settings' }))
    expect(onOpenSettings).toHaveBeenCalledTimes(1)
  })

  it('off is a choice, not a problem: no alert and no error colour class', async () => {
    stub(agentRunOff)
    const { container } = open()
    await ready()
    expect(screen.queryByRole('alert')).toBeNull()
    expect(container.querySelector('.error')).toBeNull()
  })

  it('claude not found: the line, and no button', async () => {
    stub(agentRunOn({ claude_found: false }))
    open()
    await ready()
    expect(screen.getByText('Claude Code was not found on this machine.')).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: RUN })).toBeNull()
  })

  it('off wins over claude not found (the setting is the first thing to fix)', async () => {
    stub({ ...agentRunOff, claude_found: false })
    open()
    await ready()
    expect(screen.getByText(/^Agent runs are off\./)).toBeInTheDocument()
    expect(screen.queryByText('Claude Code was not found on this machine.')).toBeNull()
  })

  it('an account whose folder is gone: a line that sends to Settings, no button', async () => {
    stub(agentRunOn({ account: { id: '2', label: 'Claude personal', status: 'missing' } }))
    open()
    await ready()
    expect(screen.getByText(/^The agent run account's folder is gone\./)).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: RUN })).toBeNull()
  })

  it('on and idle: the button, the account that pays next to it, the scope', async () => {
    stub(agentRunOn())
    open({ scope: { workspace: 'Acme Co', account: '' } })
    await ready()
    const button = screen.getByRole('button', { name: RUN })
    expect(button).toBeEnabled()
    expect(screen.getByText("Runs with Claude personal. It counts against that account's plan limits.")).toBeInTheDocument()
    expect(screen.getByText('Scope: Acme Co')).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: STOP })).toBeNull()
  })

  it('on with the default account (a flag from the terminal): the same block with the default account named', async () => {
    stub(agentRunOn({ account: null }))
    open()
    await ready()
    expect(screen.getByText("Runs with the default Claude account. It counts against that account's plan limits.")).toBeInTheDocument()
    expect(screen.getByText('Scope: all workspaces')).toBeInTheDocument()
  })

  it('running: the status in one polite live region, the elapsed time, Stop, and Run off', async () => {
    stub(agentRunOn({ running: true }))
    open()
    await ready()
    const status = await screen.findByText('Claude is investigating…')
    expect(status).toHaveAttribute('aria-live', 'polite')
    expect(status).toHaveAttribute('role', 'status')
    expect(screen.getByText(/^Running for at least \d+:\d\d$/)).toBeInTheDocument()
    expect(screen.getByRole('button', { name: RUN })).toBeDisabled()
    expect(screen.getByRole('button', { name: STOP })).toBeEnabled()
    // the ticking time is outside the live region, so it is not read out every second
    expect(status.textContent).not.toMatch(/\d:\d\d/)
    expect(screen.getAllByRole('status')).toHaveLength(1)
  })

  it.each([
    ['ok with a report', agentLast('ok', 7), 'Done. The report is first in Saved reports.'],
    ['ok without a report', agentLast('ok', null), 'Done, but no report was saved.'],
    ['failed', agentLast('failed'), 'The run failed. Check that this account is logged in to Claude Code and its plan limit is not reached.'],
    ['timed out', agentLast('timed out'), 'The run was stopped after 10 minutes.'],
    ['stopped', agentLast('stopped'), 'Stopped.'],
    ['claude not found', agentLast('claude not found'), 'Claude Code was not found on this machine.'],
  ])('ended %s: its exact line, and the button is back', async (_, last, line) => {
    stub(agentRunOn({ last }))
    open()
    await ready()
    expect(await screen.findByText(line)).toBeInTheDocument()
    expect(screen.getByRole('button', { name: RUN })).toBeEnabled()
  })

  it('an outcome word the page does not know draws no line (no guess)', async () => {
    stub(agentRunOn({ last: agentLast('something new') }))
    open()
    await ready()
    expect(screen.getByRole('status')).toHaveTextContent('')
  })

  it('shows nothing when the route is not there (the Python server answers 404)', async () => {
    const { f } = stub(undefined)
    const { container } = open()
    await vi.waitFor(() => expect(gets(f)).toHaveLength(1))
    await act(async () => {})
    expect(container).toBeEmptyDOMElement()
  })

  it('a failed GET: one quiet line, no button, and no second request', async () => {
    const f = fakeFetch({ agentRun: () => agentRunError(500) })
    vi.stubGlobal('fetch', f)
    open()
    expect(await screen.findByText('Could not read the agent run state.')).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: RUN })).toBeNull()
    expect(screen.queryByRole('alert')).toBeNull()
    await act(async () => {
      await new Promise((r) => setTimeout(r, 50))
    })
    expect(gets(f)).toHaveLength(1)
    expect(document.body.textContent).not.toContain('server text')
  })
})

describe('Run and Stop (AC4)', () => {
  it('a click on Run sends one POST: the job, the page workspace, no account key', async () => {
    const { f } = stub(agentRunOn(), { post: agentRunOn({ running: true }) })
    open({ scope: { workspace: 'Acme Co', account: '2' } })
    await ready()
    fireEvent.click(await screen.findByRole('button', { name: RUN }))
    expect(await screen.findByText('Claude is investigating…')).toBeInTheDocument()
    expect(writes(f)).toEqual([{ method: 'POST', url: '/api/agent-run', body: { job: 'investigate', workspace: 'Acme Co' } }])
    expect(Object.keys(writes(f)[0].body)).not.toContain('account')
    // the exact elapsed time of a run this page started counts from the click
    expect(screen.getByText(/^Running for \d+:\d\d$/)).toBeInTheDocument()
  })

  it('with no workspace in the scope the body is the job alone', async () => {
    const { f } = stub(agentRunOn(), { post: agentRunOn({ running: true }) })
    open()
    fireEvent.click(await screen.findByRole('button', { name: RUN }))
    await screen.findByText('Claude is investigating…')
    expect(writes(f)[0].body).toEqual({ job: 'investigate' })
  })

  it('a double click sends one request', async () => {
    let release
    const { f } = stub(agentRunOn(), { post: () => new Promise((r) => (release = r)) })
    open()
    const button = await screen.findByRole('button', { name: RUN })
    fireEvent.click(button)
    fireEvent.click(button)
    expect(writes(f)).toHaveLength(1)
    release(agentRunOn({ running: true }))
    await screen.findByText('Claude is investigating…')
    expect(writes(f)).toHaveLength(1)
  })

  it('Stop sends one POST to the stop route and shows Stopped.', async () => {
    const { f } = stub(agentRunOn({ running: true }), { stop: agentRunOn({ running: false, last: agentLast('stopped') }) })
    open()
    fireEvent.click(await screen.findByRole('button', { name: STOP }))
    expect(await screen.findByText('Stopped.')).toBeInTheDocument()
    expect(writes(f)).toEqual([{ method: 'POST', url: '/api/agent-run/stop', body: {} }])
    expect(screen.getByRole('button', { name: RUN })).toBeEnabled()
    expect(screen.queryByRole('button', { name: STOP })).toBeNull()
  })

  it.each([
    [403, 'Agent runs are off. Choose an account in Settings.'],
    [409, 'A run is already going.'],
    [400, 'The run could not start: the workspace is not known.'],
    [500, 'Could not start the run. Try again.'],
  ])('a %i answer shows its fixed line, never the body text, never markup', async (status, line) => {
    const { f } = stub(agentRunOn(), { post: agentRunError(status) })
    const { container } = open({ scope: { workspace: 'Acme Co', account: '' } })
    fireEvent.click(await screen.findByRole('button', { name: RUN }))
    expect(await screen.findByText(line)).toBeInTheDocument()
    expect(within(screen.getByRole('status')).getByText(line)).toBeInTheDocument()
    expect(document.body.textContent).not.toContain('server text')
    expect(document.body.innerHTML).not.toContain('<script')
    expect(container.querySelector('script')).toBeNull()
    expect(window.__pwned).toBeUndefined()
    expect(writes(f)).toHaveLength(1)
  })

  it('a Stop that fails shows a fixed line and keeps the Stop button', async () => {
    stub(agentRunOn({ running: true }), { stop: agentRunError(500) })
    open()
    fireEvent.click(await screen.findByRole('button', { name: STOP }))
    expect(await screen.findByText('Could not stop the run. Try again.')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: STOP })).toBeEnabled()
  })

  it('a Stop that answers 409 says the run belongs to another process, with a fixed line and no server text', async () => {
    stub(agentRunOn({ running: true }), { stop: agentRunError(409) })
    open()
    fireEvent.click(await screen.findByRole('button', { name: STOP }))
    expect(await screen.findByText('This run was started outside the app. Stop it there.')).toBeInTheDocument()
    expect(screen.queryByText('Could not stop the run. Try again.')).toBeNull()
  })

  it('markup in the account label and the scope is drawn as text', async () => {
    stub(agentRunOn({ account: { id: '2', label: '<img src=x onerror="window.__pwned=1">', status: 'confirmed' } }))
    const { container } = open({ scope: { workspace: '<b>w</b>', account: '' } })
    await ready()
    expect(container.querySelector('img, b')).toBeNull()
    expect(screen.getByText(/Runs with <img src=x onerror="window.__pwned=1">\./)).toBeInTheDocument()
  })
})

describe('polling and the Saved reports reload (AC5)', () => {
  beforeEach(() => vi.useFakeTimers())
  const settle = () => act(async () => void (await vi.advanceTimersByTimeAsync(0)))
  const tick = (ms) => act(async () => void (await vi.advanceTimersByTimeAsync(ms)))

  it('asks again every 5 s only while a run is going, and stops after it ends', async () => {
    const { f, state } = stub(agentRunOn({ running: true }))
    open()
    await settle()
    expect(gets(f)).toHaveLength(1)
    await tick(4999)
    expect(gets(f)).toHaveLength(1)
    await tick(1)
    expect(gets(f)).toHaveLength(2)
    await tick(5000)
    expect(gets(f)).toHaveLength(3)
    state.get = agentRunOn({ last: agentLast('failed') })
    await tick(5000)
    expect(gets(f)).toHaveLength(4)
    expect(screen.getByText(/^The run failed\./)).toBeInTheDocument()
    await tick(30000)
    expect(gets(f)).toHaveLength(4)
  })

  it('does not poll at all when no run is going', async () => {
    const { f } = stub(agentRunOn())
    open()
    await settle()
    await tick(60000)
    expect(gets(f)).toHaveLength(1)
  })

  it('stops asking when the block goes away', async () => {
    const { f } = stub(agentRunOn({ running: true }))
    const { unmount } = open()
    await settle()
    await tick(5000)
    expect(gets(f)).toHaveLength(2)
    unmount()
    await tick(30000)
    expect(gets(f)).toHaveLength(2)
  })

  it('a run that ends ok with a report asks for the Saved reports list once', async () => {
    const { state } = stub(agentRunOn({ running: true }))
    const onReportSaved = vi.fn()
    open({ onReportSaved })
    await settle()
    await tick(5000)
    expect(onReportSaved).not.toHaveBeenCalled()
    state.get = agentRunOn({ last: agentLast('ok', 7) })
    await tick(5000)
    expect(screen.getByText('Done. The report is first in Saved reports.')).toBeInTheDocument()
    expect(onReportSaved).toHaveBeenCalledTimes(1)
    await tick(30000)
    expect(onReportSaved).toHaveBeenCalledTimes(1)
  })

  it('an end without a report, a failure, and a block that never saw the run going ask for no reload', async () => {
    const { state } = stub(agentRunOn({ running: true }))
    const onReportSaved = vi.fn()
    open({ onReportSaved })
    await settle()
    state.get = agentRunOn({ last: agentLast('ok', null) })
    await tick(5000)
    expect(screen.getByText('Done, but no report was saved.')).toBeInTheDocument()
    expect(onReportSaved).not.toHaveBeenCalled()
  })

  it('a page that opens after an ok run does not reload the list (the list loads itself)', async () => {
    stub(agentRunOn({ last: agentLast('ok', 7) }))
    const onReportSaved = vi.fn()
    open({ onReportSaved })
    await settle()
    expect(onReportSaved).not.toHaveBeenCalled()
  })

  it('the elapsed time ticks every second from the click of this page', async () => {
    vi.setSystemTime(new Date('2026-11-17T03:00:00Z'))
    const { state } = stub(agentRunOn(), { post: agentRunOn({ running: true }) })
    open()
    await settle()
    fireEvent.click(screen.getByRole('button', { name: RUN }))
    await settle()
    expect(screen.getByText('Running for 0:00')).toBeInTheDocument()
    state.get = agentRunOn({ running: true })
    await tick(65000)
    expect(screen.getByText('Running for 1:05')).toBeInTheDocument()
  })

  it('a poll that fails keeps the last answer and tries again at the next tick', async () => {
    const { f, state } = stub(agentRunOn({ running: true }))
    open()
    await settle()
    state.get = agentRunError(500)
    await tick(5000)
    expect(screen.getByText('Claude is investigating…')).toBeInTheDocument()
    state.get = agentRunOn({ last: agentLast('timed out') })
    await tick(5000)
    expect(gets(f)).toHaveLength(3)
    expect(screen.getByText('The run was stopped after 10 minutes.')).toBeInTheDocument()
  })
})
