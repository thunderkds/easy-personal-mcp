import { useEffect, useRef, useState } from 'react'
import { getAgentRun, startAgentRun, stopAgentRun } from './api.js'

// The Run block of Warnings & plan (FR-104 part 3). The server holds the truth; this block asks `GET /api/agent-run`
// and words what it answers. It draws no text of a server answer: every line below is fixed, and an error is a line chosen by its status.
const POLL_MS = 5000
const NOT_FOUND = 'Claude Code was not found on this machine.'
const RUNNING = 'Claude is investigating…'
const ENDED = new Map([
  ['failed', 'The run failed. Check that this account is logged in to Claude Code and its plan limit is not reached.'],
  ['timed out', 'The run was stopped after 10 minutes.'],
  ['stopped', 'Stopped.'],
  ['claude not found', NOT_FOUND],
])
const START_ERRORS = new Map([
  [403, 'Agent runs are off. Choose an account in Settings.'],
  [409, 'A run is already going.'],
  [400, 'The run could not start: the workspace is not known.'],
])

/** The outcome of the last run in words; '' for no run yet or a word this page does not know. */
function outcomeLine(last) {
  if (!last) return ''
  if (last.outcome === 'ok') return last.report_id != null ? 'Done. The report is first in Saved reports.' : 'Done, but no report was saved.'
  return ENDED.get(last.outcome) ?? ''
}

const clock = (ms) => {
  const seconds = Math.max(0, Math.floor(ms / 1000))
  return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, '0')}`
}

function SettingsLink({ onOpenSettings }) {
  return (
    <button type="button" className="link-button" onClick={onOpenSettings}>
      Settings
    </button>
  )
}

/**
 * Run investigate / Stop, above Saved reports. Shows nothing while loading and when the server has no such route.
 * `scope` is the page's scope (the workspace is the run's scope); `onReportSaved` is called once when a run this block saw going ends ok with a report.
 */
export default function AgentRun({ scope, onOpenSettings, onReportSaved }) {
  const [state, setState] = useState({ status: 'loading' }) // loading | none (404) | error | ready (+ data)
  const [notice, setNotice] = useState('') // a fixed line for the last click that failed
  const [busy, setBusy] = useState(false)
  const [now, setNow] = useState(() => Date.now())
  const click = useRef(false) // a click is being answered: a second one is dropped
  const epoch = useRef(0) // the newest request; an older answer arriving late is dropped
  const latest = useRef(null)
  const since = useRef(null) // when this page began to see the run: {at, exact}; exact when this page started it
  const alive = useRef(true)
  const saved = useRef(onReportSaved)
  saved.current = onReportSaved

  function apply(next) {
    const before = latest.current
    latest.current = next
    if (next.running) since.current ??= { at: Date.now(), exact: false }
    else since.current = null
    if (before?.running !== next.running) setNotice('')
    if (before?.running && !next.running && next.last?.outcome === 'ok' && next.last.report_id != null) saved.current()
    setState({ status: 'ready', data: next })
  }

  /** One read of the state; `fresh` is false for the first read, which may find there is no route. */
  function read() {
    const mine = ++epoch.current
    return getAgentRun().then(
      (data) => {
        if (!alive.current || mine !== epoch.current) return
        if (data === null) setState({ status: 'none' })
        else apply(data)
      },
      () => {
        if (alive.current && mine === epoch.current && !latest.current) setState({ status: 'error' })
      },
    )
  }

  useEffect(() => {
    alive.current = true
    read()
    return () => {
      alive.current = false
    }
  }, [])

  const running = state.status === 'ready' && state.data.running
  // Ask again every 5 s, and tick the clock every second, only while a run is going.
  useEffect(() => {
    if (!running) return undefined
    setNow(Date.now())
    const poll = setInterval(read, POLL_MS)
    const tick = setInterval(() => setNow(Date.now()), 1000)
    return () => {
      clearInterval(poll)
      clearInterval(tick)
    }
  }, [running])

  /** A click on Run or Stop: one request at a time; a failure becomes the fixed line of its status. */
  async function act(request, failed, exact) {
    if (click.current) return
    click.current = true
    setBusy(true)
    setNotice('')
    const mine = ++epoch.current
    try {
      if (exact) since.current = { at: Date.now(), exact: true }
      const next = await request()
      if (alive.current && mine === epoch.current) apply(next)
    } catch (error) {
      if (!alive.current) return
      if (exact) since.current = null
      setNotice(failed.get(error.status) ?? failed.get('other'))
      if (error.status === 403 || error.status === 409) read() // the setting or the run changed under this page
    } finally {
      click.current = false
      if (alive.current) setBusy(false)
    }
  }
  const run = () => act(() => startAgentRun(scope.workspace), new Map([...START_ERRORS, ['other', 'Could not start the run. Try again.']]), true)
  const stop = () =>
    act(stopAgentRun, new Map([[409, 'This run was started outside the app. Stop it there.'], ['other', 'Could not stop the run. Try again.']]), false)

  if (state.status === 'loading' || state.status === 'none') return null
  const { data } = state
  const account = data?.account
  const label = account ? account.label : 'the default Claude account'
  let body
  if (state.status === 'error') {
    body = <p className="muted">Could not read the agent run state.</p>
  } else if (!data.enabled) {
    body = (
      <p>
        Agent runs are off. Choose an account in <SettingsLink onOpenSettings={onOpenSettings} />.
      </p>
    )
  } else if (!data.claude_found) {
    body = <p>{NOT_FOUND}</p>
  } else if (account?.status === 'missing') {
    body = (
      <p>
        The agent run account&apos;s folder is gone. Choose another in <SettingsLink onOpenSettings={onOpenSettings} />.
      </p>
    )
  } else {
    const elapsed = since.current ? clock(now - since.current.at) : '0:00'
    body = (
      <>
        <p>{`Runs with ${label}. It counts against that account's plan limits.`}</p>
        <p className="muted">{`Scope: ${scope.workspace || 'all workspaces'}`}</p>
        <p role="status" aria-live="polite" className="agent-run-status">
          {notice || (data.running ? RUNNING : outcomeLine(data.last))}
        </p>
        {data.running && <p className="muted">{`Running for ${since.current?.exact ? '' : 'at least '}${elapsed}`}</p>}
        <div className="actions">
          <button type="button" className="primary" disabled={data.running || busy} onClick={run}>
            Run investigate
          </button>
          {data.running && (
            <button type="button" disabled={busy} onClick={stop}>
              Stop
            </button>
          )}
        </div>
      </>
    )
  }
  return (
    <section aria-labelledby="h-agent-run" className="agent-run card">
      <h2 id="h-agent-run">Agent run</h2>
      {body}
    </section>
  )
}
