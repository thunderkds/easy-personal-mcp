import { fmt } from './format.js'
import Loading from './Loading.jsx'
import { PERIODS, sinceOf } from './UsageView.jsx'
import { Progress } from './Widgets.jsx'

const HEX_COLOR = /^#[0-9a-fA-F]{6}$/
const plural = (n, word) => `${fmt(n)} ${word}${n === 1 ? '' : 's'}`
const pad = (n) => String(n).padStart(2, '0')
/** 'YYYY-MM-DD' of a UTC time in local time. */
const day = (utc) => {
  const d = new Date(utc)
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`
}
/** A score or mean as a number: whole when it is whole, else one decimal. */
const score = (n) => String(Number(n.toFixed(1)))

/** The request for the block: the page's period (as `since`) and account. The workspace filter is not sent: every workspace is listed. */
export function workspaceKitPath(period, account, now = Date.now()) {
  const chosen = PERIODS.find((p) => p.id === period) ?? PERIODS[0]
  const parts = []
  if (chosen.days !== null) parts.push(`since=${encodeURIComponent(sinceOf(chosen.days, now))}`)
  if (account) parts.push(`account=${encodeURIComponent(account)}`)
  return ['/api/kit/workspaces', parts.join('&')].filter(Boolean).join('?')
}

function Verifier({ part }) {
  const hasScore = part?.status === 'ok' && typeof part.mean === 'number'
  if (!hasScore) return <span>no data</span>
  const { repos_scored: scored, repos_total: total, lowest, newest_utc: newest } = part
  return (
    <>
      {typeof scored === 'number' && typeof total === 'number' && <span>{`${fmt(scored)} of ${plural(total, 'repo')} scored`}</span>}
      <div className="ws-mean">
        mean {score(part.mean)}
        <Progress ratio={part.mean / 100} label={`Verifier mean ${score(part.mean)} of 100`} />
      </div>
      {lowest && typeof lowest.value === 'number' && <span>{`lowest ${lowest.project} ${score(lowest.value)}`}</span>}
      {newest && <span className="muted">{`newest report ${day(newest)}`}</span>}
    </>
  )
}

function Ui({ part }) {
  if (part?.status !== 'ok') return <span>not used</span>
  const last = part.last_utc ? ` · last ${day(part.last_utc)}` : ''
  return <span>{`${plural(part.sessions ?? 0, 'session')} · ${plural(part.calls ?? 0, 'call')}${last}`}</span>
}

function Row({ workspace, selected }) {
  const dot = HEX_COLOR.test(workspace.color ?? '') ? { background: workspace.color } : undefined
  return (
    <li className="ws-row" aria-label={workspace.name} aria-current={selected ? 'true' : undefined}>
      <div className="ws-head">
        <span className="ws-dot" style={dot} aria-hidden="true" />
        <span className="ws-name" title={workspace.name}>
          {workspace.name}
        </span>
        {selected && <span className="chip">selected</span>}
      </div>
      <div className="ws-cell">
        <span className="ws-label">Verifier</span>
        <Verifier part={workspace.verifier} />
      </div>
      <div className="ws-cell">
        <span className="ws-label">easy-ui</span>
        <Ui part={workspace.ui} />
      </div>
    </li>
  )
}

function Body({ state, selected, onOpenWorkspaces }) {
  if (state.status === 'loading') return <Loading label="Loading kit by workspace" />
  if (state.status === 'error') {
    return (
      <p role="alert" className="error">
        Could not load kit by workspace: {state.message}
      </p>
    )
  }
  const { workspaces = [], ui_checks: checks } = state.data
  if (workspaces.length === 0) {
    return (
      <p className="muted">
        No workspaces yet.{' '}
        {onOpenWorkspaces && (
          <button type="button" className="link-button" onClick={onOpenWorkspaces}>
            Open Workspaces
          </button>
        )}
      </p>
    )
  }
  return (
    <>
      <ul className="ws-rows" aria-label="Kit by workspace">
        {workspaces.map((w) => (
          <Row key={w.name} workspace={w} selected={w.name === selected} />
        ))}
      </ul>
      {checks?.status === 'ok' && (
        <p className="muted">
          {`easy-ui checks (all workspaces): ${plural(checks.sessions, 'session')}, ${fmt(checks.passed)} passed, ${fmt(checks.failed)} failed — not split by workspace`}
        </p>
      )}
    </>
  )
}

/**
 * The Kit view's "by workspace" block (FR-087, FR-088). `state` is the read of GET /api/kit/workspaces
 * ({status: 'loading' | 'ready' | 'error', data, message}); the Kit view makes it, in parallel with its own read.
 * Every workspace is listed; `selected` (the page's workspace filter) only marks its row.
 */
export default function WorkspaceKit({ state, selected = '', period = '7', onOpenWorkspaces }) {
  const count = state.status === 'ready' ? (state.data.workspaces ?? []).length : null
  const days = (PERIODS.find((p) => p.id === period) ?? PERIODS[0]).days
  return (
    <section aria-labelledby="h-kit-ws">
      <h2 id="h-kit-ws">
        Kit by workspace {count !== null && <span className="count">{`(${count})`}</span>}
      </h2>
      <p className="muted">
        {`Newest verifier score per repo, and easy-ui use ${days === null ? 'over all time' : `in the last ${days} days`} (the period is set on Usage).`}
      </p>
      <Body state={state} selected={selected} onOpenWorkspaces={onOpenWorkspaces} />
    </section>
  )
}
