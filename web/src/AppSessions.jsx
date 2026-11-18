import { useEffect, useState } from 'react'
import { getAgentRun, getAppSessions, getJson } from './api.js'
import Loading from './Loading.jsx'
import NewSession from './NewSession.jsx'
import { useLoad } from './useLoad.js'

// The App sessions page (T104, FR-109, T118 FR-128): it only starts a session. The session itself, its conversation, cards and controls live in
// the one Live panel; `onOpenAppSession(id)` takes the new session there.
export const OFF = 'App sessions are off. Choose an agent run account in Settings.'
export const NO_CLAUDE = 'Claude Code was not found on this machine, so no session can start.'

function Loaded({ list, agentRun, workspaces, accounts, target, onTargetUsed, onOpenSettings, onOpenAppSession }) {
  const [form, setForm] = useState(() => (target?.newFor !== undefined ? { workspace: target.newFor } : target?.resume ? { resume: target.resume } : target?.continueIn ? { continueIn: target.continueIn } : null))
  useEffect(() => {
    if (target) onTargetUsed?.()
  }, [])
  const going = list.sessions.filter((s) => s.state !== 'ended').length
  const full = going >= list.max
  const off = !agentRun.enabled
  const noClaude = agentRun.claude_found === false
  const defaultAccount = agentRun.account ? String(agentRun.account.id) : ''
  return (
    <>
      {off && (
        <p className="muted">
          {OFF}{' '}
          {onOpenSettings && (
            <button type="button" className="link-button" onClick={onOpenSettings}>
              Open Settings
            </button>
          )}
        </p>
      )}
      {!off && noClaude && <p role="alert" className="error">{NO_CLAUDE}</p>}
      <div className="actions">
        <button type="button" className="primary" disabled={off || noClaude || full || form !== null} onClick={() => setForm({ workspace: undefined })}>
          New session
        </button>
        {full && <span className="muted">{`${list.max} sessions are going; stop one first.`}</span>}
      </div>
      {form && !off && !noClaude && (
        <NewSession
          workspaces={workspaces}
          accounts={accounts}
          defaultAccount={defaultAccount}
          initialWorkspace={form.workspace}
          resume={form.resume}
          continueIn={form.continueIn}
          onStarted={(id) => {
            setForm(null)
            onOpenAppSession?.(id)
          }}
          onCancel={() => setForm(null)}
        />
      )}
      {!form && <p className="muted">Start a session with New session. It opens in Live, where every session is.</p>}
    </>
  )
}

const load = () => Promise.all([getAppSessions(), getAgentRun(), getJson('/api/workspaces'), getJson('/api/accounts?by=account')])

/**
 * The page. `target`: `{newFor: <workspace name>}` opens the form for that workspace (Workspaces page), `{resume: {id, project, account, fork}}`
 * opens it for a stored Claude session (Live view), `{continueIn: {workspace, repo}}` opens it to continue a repo (Workspaces page);
 * `onTargetUsed` is called once it was read. `onOpenAppSession(id)`: a session was started, show it in Live.
 */
export default function AppSessions({ target = null, onTargetUsed, onOpenSettings, onOpenAppSession }) {
  const state = useLoad(load, [])
  const [list, agentRun, workspaces, accounts] = state.status === 'ready' ? state.data : []
  return (
    <main className="app-sessions">
      <h1>App sessions</h1>
      <p className="muted">
        A session here is Claude Code on this machine, started from this page and then run from Live like any other session. What you type and what Claude answers is shown there and kept in memory only: it is dropped when you close an ended session and when the app exits.
      </p>
      {state.status === 'loading' && <Loading label="Loading app sessions" />}
      {state.status === 'error' && (
        <p role="alert" className="error">
          Could not load app sessions: {state.message}
        </p>
      )}
      {state.status === 'ready' && (list === null || agentRun === null) && <p className="muted">App sessions are not available in this version of the app.</p>}
      {state.status === 'ready' && list && agentRun && (
        <Loaded
          list={list}
          agentRun={agentRun}
          workspaces={workspaces.workspaces}
          accounts={accounts.accounts.filter((a) => a.provider === 'claude' && a.status === 'confirmed')}
          target={target}
          onTargetUsed={onTargetUsed}
          onOpenSettings={onOpenSettings}
          onOpenAppSession={onOpenAppSession}
        />
      )}
    </main>
  )
}
