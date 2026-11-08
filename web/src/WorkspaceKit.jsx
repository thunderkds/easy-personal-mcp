import { useState } from 'react'
import { getBytes, getJson } from './api.js'
import { fmt } from './format.js'
import Loading from './Loading.jsx'
import { PERIODS, sinceOf } from './UsageView.jsx'
import { useLoad } from './useLoad.js'
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

const SHOWN_LIMIT = 200
/** '12 KB': whole units, 1024 steps. */
function sizeText(n) {
  if (typeof n !== 'number' || n < 0) return '–'
  if (n < 1024) return `${n} B`
  if (n < 1024 * 1024) return `${Math.round(n / 1024)} KB`
  return `${Math.round(n / (1024 * 1024))} MB`
}
/** 'YYYY-MM-DD', or a dash when the server could not give a time (mtime_utc null) or it does not parse. */
const dateText = (utc) => (utc && !Number.isNaN(Date.parse(utc)) ? day(utc) : '–')

const query = (params) => new URLSearchParams(params).toString()

/**
 * Open one report file in a new tab (FR-090). The blank tab opens synchronously in the click (so a pop-up blocker allows it),
 * the bytes come with the token header, and the tab goes to a blob: URL (no token in a URL). Returns 'blocked' | 'pending'.
 * `onFail` runs when the fetch fails.
 */
function openReportFile(base, path, onFail) {
  const win = window.open('', '_blank')
  if (!win) return 'blocked'
  // Do not drop this as redundant. The blob tab is same-origin with the dashboard, and a repo-authored
  // report is only stopped from running script by the CSP the blob document inherits from this page.
  // Chromium and Gecko inherit it; WebKit has shipped bugs where it does not, and a blob URL pasted into
  // a fresh tab gets no inherited policy at all. Disowning the opener is the layer that still holds in
  // those cases: without it, script in the tab could read the per-launch token from opener.location.hash.
  win.opener = null
  getBytes(`/api/kit/report-file?${query({ ...base, path })}`).then(
    ({ bytes, type }) => {
      const url = URL.createObjectURL(new Blob([bytes], { type }))
      win.location = url
      setTimeout(() => URL.revokeObjectURL(url), 60000)
    },
    () => {
      win.close()
      onFail()
    },
  )
  return 'pending'
}

function FileRow({ file, base }) {
  const [notice, setNotice] = useState('')
  const open = () => {
    setNotice('')
    if (openReportFile(base, file.path, () => setNotice('cannot open this file')) === 'blocked') setNotice('allow pop-ups to open the file')
  }
  return (
    <li className="ws-file">
      {file.openable ? (
        <button type="button" className="ws-file-name" title={file.path} onClick={open}>
          {file.path}
        </button>
      ) : (
        <span className="ws-file-name muted" title="this file type is not opened here">
          {file.path}
        </span>
      )}
      <span className="ws-file-meta muted">{`${sizeText(file.size)} · ${dateText(file.mtime_utc)}`}</span>
      {notice && (
        <span role="status" className="ws-file-notice">
          {notice}
        </span>
      )}
    </li>
  )
}

/** The files of one report folder: fetched once, when the folder is first opened (the parent keeps this mounted while closed). */
function ReportFiles({ name, folder, open }) {
  const base = { workspace: name, repo_no: String(folder.repo_no ?? 0), folder: folder.folder }
  const state = useLoad(() => getJson(`/api/kit/report-files?${query(base)}`), [])
  let body
  if (state.status === 'loading') body = <span className="muted">loading files</span>
  else if (state.status === 'error') body = <span className="muted">files unavailable</span>
  else {
    const files = Array.isArray(state.data?.files) ? state.data.files : []
    body =
      files.length === 0 ? (
        <span className="muted">no files</span>
      ) : (
        <>
          <ul aria-label={`Files in ${folder.repo}/${folder.folder}`}>
            {files.map((f, i) => (
              <FileRow key={`${f.path}/${i}`} file={f} base={base} />
            ))}
          </ul>
          {state.data.truncated && <span className="muted">{`first ${SHOWN_LIMIT} files shown`}</span>}
        </>
      )
  }
  return (
    <div className="ws-files" hidden={!open}>
      {body}
    </div>
  )
}

/** One report folder line (FR-090): a disclosure button; the file list is requested on first open and kept after. */
function ReportFolder({ name, folder }) {
  const [open, setOpen] = useState(false)
  const [seen, setSeen] = useState(false)
  const text = folderText(folder)
  return (
    <li className="ws-report">
      <button
        type="button"
        className="ws-report-toggle"
        aria-expanded={open}
        title={text}
        onClick={() => {
          setOpen(!open)
          setSeen(true)
        }}
      >
        <span className="ws-chevron" aria-hidden="true" />
        <span className="ws-report-text">{text}</span>
      </button>
      {seen && <ReportFiles name={name} folder={folder} open={open} />}
    </li>
  )
}

/** Report folders (FR-089, FR-090) of one workspace: each line opens to its files. Shows nothing while the read is pending. */
function ReportFolders({ name, reports }) {
  if (reports?.status === 'error') return <div className="ws-reports muted">report folders unavailable</div>
  if (reports?.status !== 'ready') return null
  const entry = (reports.data?.workspaces ?? []).find((w) => w.name === name)
  const folders = Array.isArray(entry?.folders) ? entry.folders : []
  if (folders.length === 0) return <div className="ws-reports muted">no report folders</div>
  return (
    <div className="ws-reports">
      <ul aria-label={`Report folders of ${name}`}>
        {folders.map((f, i) => (
          <ReportFolder key={`${f.repo}/${f.folder}/${i}`} name={name} folder={f} />
        ))}
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
 * adds the report folders, each a button that lists its files (FR-090). While it is pending, or missing, a row shows no report line and the block shows no pill.
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
