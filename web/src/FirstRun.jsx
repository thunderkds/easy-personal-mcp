import { useEffect, useRef, useState } from 'react'
import Loading from './Loading.jsx'

// What each API step id is called here, and how a not-done step is finished:
// `command` = run the hint in a terminal; `agents` / `workspaces` = a button to that view.
// An id not listed here still gets a card (its id as the title) and no action.
const STEPS = {
  first_ingest: { title: 'First ingest', how: 'command', why: 'Copies the usage metadata of your Claude Code sessions into the store.' },
  ingest_timer: { title: 'Ingest timer installed', how: 'command', why: 'Keeps the store current, because Claude Code deletes old transcripts.' },
  accounts_confirmed: { title: 'Agent accounts confirmed', how: 'agents', why: 'Only confirmed accounts are read.' },
  first_workspace: { title: 'First workspace', how: 'workspaces', why: 'Groups your repos and connectors.' },
}

const COPIED_MS = 2000
const STATE_TEXT = { done: 'done', next: 'next step', later: 'not done' }

/** Number of steps still open; `null` while unknown. */
export function stepsLeft(steps) {
  return Array.isArray(steps) ? steps.filter((s) => !s.done).length : null
}

const describe = (step) => {
  const known = STEPS[step.id]
  if (known) return known
  const words = String(step.id).replace(/_/g, ' ')
  return { title: words.charAt(0).toUpperCase() + words.slice(1), how: 'none', why: '' }
}

function Tick() {
  return (
    <svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true" focusable="false">
      <path d="M2.5 6.2l2.3 2.3 4.7-4.9" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  )
}

/**
 * The `// system` checklist. `steps` comes from App: undefined while loading, null when the request failed.
 * `onCheckAgain` re-reads `/api/first-run` in App and rejects when that fails; the page then keeps the old steps.
 */
export default function FirstRun({ steps, onOpenAgents, onOpenWorkspaces, onOpenUsage, onCheckAgain }) {
  const [copiedId, setCopiedId] = useState(null)
  const [notice, setNotice] = useState(null) // { id, text }: one short error line under one card
  const [checking, setChecking] = useState(false)
  const timer = useRef(null)
  const commands = useRef({})

  useEffect(() => () => clearTimeout(timer.current), [])

  // Select the command text, so a failed Copy can be finished with the keyboard.
  const select = (id) => {
    try {
      const node = commands.current[id]
      const selection = window.getSelection()
      if (!node || !selection) return
      const range = document.createRange()
      range.selectNodeContents(node)
      selection.removeAllRanges()
      selection.addRange(range)
    } catch {
      // Selecting is a courtesy only.
    }
  }

  const copy = async (step) => {
    try {
      await navigator.clipboard.writeText(step.hint)
      clearTimeout(timer.current) // after the await: two fast presses leave one timer
      setNotice(null)
      setCopiedId(step.id)
      timer.current = setTimeout(() => setCopiedId(null), COPIED_MS)
    } catch {
      clearTimeout(timer.current)
      setCopiedId(null)
      setNotice({ id: step.id, text: 'Copy failed — select and copy' })
      select(step.id)
    }
  }

  const checkAgain = async (step) => {
    setChecking(true)
    setNotice(null)
    try {
      await onCheckAgain?.()
    } catch {
      setNotice({ id: step.id, text: 'Could not check again. The steps shown are the last ones read.' })
    } finally {
      setChecking(false)
    }
  }

  const total = Array.isArray(steps) ? steps.length : 0
  const left = stepsLeft(steps)
  const doneCount = total - (left ?? 0)
  const next = Array.isArray(steps) ? steps.findIndex((s) => !s.done) : -1
  return (
    <main className="first-run">
      <div className="first-run-head">
        <h1>First run</h1>
        {Array.isArray(steps) && (
          <p className="first-run-intro">
            {total === 4 ? 'Four' : total} steps to fill the dashboard with your own sessions. Everything stays on this machine.
          </p>
        )}
      </div>
      {steps === undefined && <Loading label="Loading" />}
      {steps === null && (
        <p role="alert" className="error">
          Could not load the checklist.
        </p>
      )}
      {Array.isArray(steps) && (
        <>
          <div className="first-run-progress">
            <div className="first-run-count">
              <span>
                <span className="first-run-done">{doneCount}</span> of {total} steps done
              </span>
              <span className="first-run-left">{left === 0 ? 'all done' : `${left} left`}</span>
            </div>
            <div className="first-run-bar" role="progressbar" aria-label="Setup progress" aria-valuenow={doneCount} aria-valuemin={0} aria-valuemax={total}>
              <div style={{ width: `${total ? (doneCount / total) * 100 : 0}%` }} />
            </div>
          </div>
          {left === 0 && (
            <div className="first-run-complete">
              <p>Setup is complete. Your usage appears after the next ingest.</p>
              <button type="button" className="solid tall" onClick={onOpenUsage}>
                Go to Usage
              </button>
            </div>
          )}
          <ol className="first-run-steps">
            {steps.map((s, i) => {
              const { title, how, why } = describe(s)
              const state = s.done ? 'done' : i === next ? 'next' : 'later'
              const solid = state === 'next' ? 'solid' : ''
              return (
                <li key={s.id} aria-label={title} data-state={state}>
                  <span className="first-run-no">{String(i + 1).padStart(2, '0')}</span>
                  <div className="first-run-body">
                    <div className="first-run-title">
                      <h2>{title}</h2>
                      <span className="first-run-state">
                        {state === 'done' && <Tick />}
                        {STATE_TEXT[state]}
                      </span>
                    </div>
                    {why && <p className="first-run-why">{why}</p>}
                    {!s.done && how === 'command' && (
                      <>
                        <div className="first-run-command">
                          <code>
                            <span className="first-run-prompt">$ </span>
                            <span ref={(node) => (commands.current[s.id] = node)}>{s.hint}</span>
                          </code>
                          <button type="button" className={solid} onClick={() => copy(s)}>
                            {copiedId === s.id ? 'Copied' : 'Copy'}
                          </button>
                        </div>
                        <div className="first-run-recheck">
                          <span>Ran it in your terminal?</span>
                          <button type="button" disabled={checking} onClick={() => checkAgain(s)}>
                            Check again
                          </button>
                        </div>
                      </>
                    )}
                    {!s.done && how === 'agents' && (
                      <div>
                        <button type="button" className={`tall ${solid}`} onClick={onOpenAgents}>
                          Open Agents
                        </button>
                      </div>
                    )}
                    {!s.done && how === 'workspaces' && (
                      <div>
                        <button type="button" className={`tall ${solid}`} onClick={onOpenWorkspaces}>
                          Open Workspaces
                        </button>
                      </div>
                    )}
                    {notice?.id === s.id && (
                      <p role="alert" className="error first-run-notice">
                        {notice.text}
                      </p>
                    )}
                  </div>
                </li>
              )
            })}
          </ol>
        </>
      )}
    </main>
  )
}
