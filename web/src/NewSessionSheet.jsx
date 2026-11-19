import { useEffect, useRef } from 'react'
import { getAgentRun, getAppSessions, getJson } from './api.js'
import Loading from './Loading.jsx'
import NewSession from './NewSession.jsx'
import { useLoad } from './useLoad.js'

// The New session sheet of the Live view (T104, FR-109; T128, FR-133): the FR-109 form, drawn in place in Live (never a dialog: FR-128 says no dialog
// covers a pending prompt card), with the reasons it cannot start. It lists no session; Live does, and `onStarted(id)` takes the new one there.
export const OFF = 'App sessions are off. Choose an agent run account in Settings.'
export const NO_CLAUDE = 'Claude Code was not found on this machine, so no session can start.'

const load = () => Promise.all([getAppSessions(), getAgentRun(), getJson('/api/workspaces'), getJson('/api/accounts?by=account')])

/**
 * `form`: `{workspace}` opens the form on that workspace's first repo (or the first repo), `{resume: {id, project, account, fork}}` opens it for a stored
 * Claude session, `{continueIn: {workspace, repo}}` opens it to continue a repo. `onStarted(id)`, `onCancel()`, `onOpenSettings()`.
 * Give it a `key` that changes for each opening: the form's fields are read once.
 */
export default function NewSessionSheet({ form, onStarted, onCancel, onOpenSettings }) {
  const state = useLoad(load, [])
  const box = useRef(null)
  useEffect(() => {
    box.current?.scrollIntoView?.({ block: 'nearest' }) // opened from a button further down the page: bring it into view
    box.current?.focus({ preventScroll: true }) // and move the keyboard in
  }, [])
  const close = (
    <div className="actions">
      <button type="button" onClick={onCancel}>
        Close
      </button>
    </div>
  )
  let body
  if (state.status === 'loading') body = <Loading label="Loading the form" />
  else if (state.status === 'error')
    body = (
      <>
        <p role="alert" className="error">
          Could not load the form: {state.message}
        </p>
        {close}
      </>
    )
  else {
    const [list, agentRun, workspaces, accounts] = state.data
    const going = list ? list.sessions.filter((s) => s.state !== 'ended').length : 0
    if (list === null || agentRun === null)
      body = (
        <>
          <p className="muted">App sessions are not available in this version of the app.</p>
          {close}
        </>
      )
    else if (!agentRun.enabled)
      body = (
        <>
          <p className="muted">
            {OFF}{' '}
            {onOpenSettings && (
              <button type="button" className="link-button" onClick={onOpenSettings}>
                Open Settings
              </button>
            )}
          </p>
          {close}
        </>
      )
    else if (agentRun.claude_found === false)
      body = (
        <>
          <p role="alert" className="error">
            {NO_CLAUDE}
          </p>
          {close}
        </>
      )
    else if (going >= list.max)
      body = (
        <>
          <p className="muted">{`${list.max} sessions are going; stop one first.`}</p>
          {close}
        </>
      )
    else
      body = (
        <NewSession
          workspaces={workspaces.workspaces}
          accounts={accounts.accounts.filter((a) => a.provider === 'claude' && a.status === 'confirmed')}
          defaultAccount={agentRun.account ? String(agentRun.account.id) : ''}
          initialWorkspace={form.workspace}
          resume={form.resume}
          continueIn={form.continueIn}
          onStarted={onStarted}
          onCancel={onCancel}
        />
      )
  }
  return (
    <div ref={box} tabIndex={-1} className="new-session-sheet" role="group" aria-label="Session form">
      {body}
    </div>
  )
}
