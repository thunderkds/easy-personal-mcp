import { useContext, useEffect, useState } from 'react'
import { isIdle, LiveContext, statesOf } from './LiveView.jsx'
import { hasPct } from './Rail.jsx'
import { limitTone, until } from './format.js'

const TICK_MS = 15_000 // "active" and "4m ago" are worked out again this often (same as the Live view)
const MINUTE_MS = 60_000

const Warning = () => (
  <svg width="12" height="12" viewBox="0 0 16 16" aria-hidden="true" focusable="false">
    <path d="M8 2.5l6 11H2z" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinejoin="round" />
    <path d="M8 7v3M8 12v.1" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
  </svg>
)

/** "just now", "4m ago", "3h ago", "2d ago": the short form the top bar has room for. */
function shortAgo(utc, now) {
  const minutes = Math.max(0, Math.floor((now - Date.parse(utc)) / MINUTE_MS))
  if (minutes < 1) return 'just now'
  if (minutes < 60) return `${minutes}m ago`
  if (minutes < 1440) return `${Math.floor(minutes / 60)}h ago`
  return `${Math.floor(minutes / 1440)}d ago`
}

/** The newest `last_ingest_utc` over the sources of GET /api/store/health, or null when there is none. */
function newestIngest(health) {
  const times = Object.values(health?.sources ?? {})
    .map((s) => s?.last_ingest_utc)
    .filter((t) => typeof t === 'string' && !Number.isNaN(Date.parse(t)))
  return times.length === 0 ? null : times.reduce((a, b) => (Date.parse(b) > Date.parse(a) ? b : a))
}

/** The confirmed Claude account with the highest 5-hour percentage (a stale limits file counts as no data, as in the rail), or null. */
function highestFiveHour(accounts) {
  const found = (accounts ?? [])
    .filter((a) => a.provider === 'claude' && a.status === 'confirmed' && hasPct(a.status_line?.limits?.five_hour))
    .map((a) => ({ name: a.label || a.folder, window: a.status_line.limits.five_hour }))
  return found.length === 0 ? null : found.reduce((top, a) => (a.window.used_percentage > top.window.used_percentage ? a : top))
}

/**
 * The top bar's status chips (G1): live sessions, sessions waiting, last ingest, 5-hour plan limit.
 * Live counts come from the page's one stream (LiveContext); the rest from data App already loads.
 * `steps`, `health`, `accounts`: undefined while loading (chip left out), null when the request failed (shown as no data).
 */
export default function StatusChips({ steps, health, accounts, onOpenLive }) {
  const live = useContext(LiveContext)
  const [now, setNow] = useState(Date.now())
  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), TICK_MS)
    return () => clearInterval(timer)
  }, [])

  const sessions = live?.loaded ? Object.values(live.state.sessions) : null // before the first snapshot the counts are unknown, not 0
  const active = sessions?.filter((s) => !isIdle(s, now, live.state.windowS)).length
  const waiting = sessions?.filter((s) => statesOf(s).length > 0).length

  const ingest = newestIngest(health)
  const noTimer = Array.isArray(steps) && steps.some((s) => s.id === 'ingest_timer' && !s.done)
  const ingestText = `${ingest ? `ingest ${shortAgo(ingest, now)}` : 'ingest no data'}${noTimer ? ' · no timer' : ''}`
  const ingestTone = noTimer ? 'amber' : ingest ? null : 'faint'

  const five = highestFiveHour(accounts)
  const pct = five ? Math.round(five.window.used_percentage) : null
  const barPct = pct === null ? 0 : Math.min(100, Math.max(0, pct))
  const reset = five ? until(five.window.resets_at, now) : null

  return (
    <div className="status-chips" role="group" aria-label="Status">
      {sessions && (
        <button type="button" className="status-chip" data-on={active > 0} title="Live sessions" onClick={onOpenLive}>
          <span className="status-dot" aria-hidden="true" />
          {`${active} active`}
        </button>
      )}
      {waiting > 0 && (
        <button type="button" className="status-chip tone-amber" title="Sessions waiting for you" onClick={onOpenLive}>
          <Warning />
          {`${waiting} waiting`}
        </button>
      )}
      {health !== undefined && (
        <span
          className={`status-chip status-ingest${ingestTone ? ` tone-${ingestTone}` : ''}`}
          title={`Last ingest${ingest ? `: ${new Date(ingest).toLocaleString()}` : ': no time stored'}${noTimer ? '. The ingest timer is not installed' : ''}`}
        >
          {noTimer && <Warning />}
          {ingestText}
        </span>
      )}
      {accounts !== undefined &&
        (five ? (
          <span className={`status-chip status-limit tone-${limitTone(pct)}`} title={`5-hour plan limit: ${five.name}, resets ${reset === 'soon' ? 'soon' : `in ${reset}`}`}>
            {`5h ${pct}%`}
            <span className="status-bar" aria-hidden="true">
              <span className="status-bar-fill" style={{ width: `${barPct}%` }} />
            </span>
          </span>
        ) : (
          <span className="status-chip tone-faint" title="5-hour plan limit: no status-line data">
            5h no data
          </span>
        ))}
    </div>
  )
}
