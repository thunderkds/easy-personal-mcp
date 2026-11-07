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

/** "app/reports · 12 files · 3 easy-ui sessions · newest 2026-11-05": the relative path only; no easy-ui part for 0, no newest part for an empty folder. */
function folderText(f) {
  const parts = [`${f.repo}/${f.folder}`, f.files_capped ? `${f.files}+ files` : plural(f.files ?? 0, 'file')]
  if (f.easy_ui_sessions > 0) parts.push(plural(f.easy_ui_sessions, 'easy-ui session'))
  if (f.newest_utc) parts.push(`newest ${day(f.newest_utc)}`)
  return parts.join(' · ')
}

/** Report folders (FR-089) of one workspace, as plain reference text: nothing to click. Shows nothing while the read is pending. */
function ReportFolders({ name, reports }) {
  if (reports?.status === 'error') return <div className="ws-reports muted">report folders unavailable</div>
  if (reports?.status !== 'ready') return null
  const entry = (reports.data?.workspaces ?? []).find((w) => w.name === name)
  const folders = Array.isArray(entry?.folders) ? entry.folders : []
  if (folders.length === 0) return <div className="ws-reports muted">no report folders</div>
  return (
    <div className="ws-reports">
      <ul className="muted" aria-label={`Report folders of ${name}`}>
        {folders.map((f, i) => {
          const text = folderText(f)
          return (
            <li key={`${f.repo}/${f.folder}/${i}`} className="ws-report" title={text}>
              {text}
            </li>
          )
        })}
      </ul>
    </div>
  )
}

function Row({ workspace, selected, reports }) {
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
      <ReportFolders name={workspace.name} reports={reports} />
    </li>
  )
}

function Body({ state, selected, reports, onOpenWorkspaces }) {
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
          <Row key={w.name} workspace={w} selected={w.name === selected} reports={reports} />
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
 * `reports` is the read of GET /api/kit/reports (same shape; FR-089), also made by the Kit view in parallel: under each row it
 * adds the report folders as plain text. While it is pending, or missing, a row shows no report line and the block shows no pill.
 */
export default function WorkspaceKit({ state, selected = '', period = '7', reports = { status: 'loading' }, onOpenWorkspaces }) {
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
      <Body state={state} selected={selected} reports={reports} onOpenWorkspaces={onOpenWorkspaces} />
    </section>
  )
}
