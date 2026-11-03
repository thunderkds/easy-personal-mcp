import Loading from './Loading.jsx'

// What each API step id is called here, and how a not-done step is finished:
// `command` = run the hint in a terminal; `agents` / `workspaces` = a button to that view.
const STEPS = {
  first_ingest: { title: 'First ingest', how: 'command', why: 'Copies the usage metadata of your Claude Code sessions into the store.' },
  ingest_timer: { title: 'Ingest timer installed', how: 'command', why: 'Keeps the store current, because Claude Code deletes old transcripts.' },
  accounts_confirmed: { title: 'Agent accounts confirmed', how: 'agents', why: 'Only confirmed accounts are read.' },
  first_workspace: { title: 'First workspace', how: 'workspaces', why: 'Groups your repos and connectors.' },
}

/** Number of steps still open; `null` while unknown. */
export function stepsLeft(steps) {
  return Array.isArray(steps) ? steps.filter((s) => !s.done).length : null
}

function Action({ step, how, onOpenAgents, onOpenWorkspaces }) {
  if (how === 'command') return <code className="first-run-command">{step.hint}</code>
  const [label, open] = how === 'agents' ? ['Open Agents', onOpenAgents] : ['Open Workspaces', onOpenWorkspaces]
  return (
    <button type="button" onClick={open}>
      {label}
    </button>
  )
}

/** The `// system` checklist. `steps` comes from App: undefined while loading, null when the request failed. */
export default function FirstRun({ steps, onOpenAgents, onOpenWorkspaces }) {
  const known = Array.isArray(steps) ? steps.filter((s) => STEPS[s.id]) : steps
  return (
    <main className="first-run">
      <h1>First run</h1>
      {known === undefined && <Loading label="Loading" />}
      {known === null && (
        <p role="alert" className="error">
          Could not load the checklist.
        </p>
      )}
      {Array.isArray(known) && (
        <>
          <ul className="first-run-steps">
            {known.map((s) => {
              const { title, how, why } = STEPS[s.id]
              return (
                <li key={s.id} aria-label={title} className="card" data-done={s.done}>
                  <h2>{title}</h2>
                  <p className="first-run-state">{s.done ? 'Done' : 'Not done'}</p>
                  <p className="muted">{why}</p>
                  {!s.done && <Action step={s} how={how} onOpenAgents={onOpenAgents} onOpenWorkspaces={onOpenWorkspaces} />}
                </li>
              )
            })}
          </ul>
          {stepsLeft(known) === 0 && <p className="muted">Setup is complete.</p>}
        </>
      )}
    </main>
  )
}
