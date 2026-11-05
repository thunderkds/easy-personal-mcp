import { useState } from 'react'
import { getReport, getReports } from './api.js'
import { fmt, localTime } from './format.js'
import { useLoad } from './useLoad.js'

// Saved reports (FR-078). The server measures every number; the page only words it. A rule's measured value is a
// ratio shown as a percent, except these two.
const VALUE_KIND = { session_huge_no_compact: 'tokens', context_index_stale: 'count' }
const ratio = new Intl.NumberFormat('en-US', { maximumFractionDigits: 1 })

function valueText(ruleId, value) {
  const kind = VALUE_KIND[ruleId]
  if (kind === 'tokens') return `${fmt(Math.round(value))} tokens`
  if (kind === 'count') return `${fmt(value)} stale`
  return `${ratio.format(value * 100)}%`
}

function Side({ ruleId, side }) {
  if (side.state === 'fired' || side.state === 'clear') {
    return (
      <>
        <span className="ba-num">{valueText(ruleId, side.measured)}</span>{' '}
        <span className={`chip ${side.state === 'fired' ? 'chip-warning' : 'chip-ok'}`}>{side.state}</span>
      </>
    )
  }
  return <span className="ba-none">{side.state === 'abstained' ? `abstained: ${side.reason}` : side.reason}</span>
}

const cite = (fix) =>
  [fix.tool, fix.number]
    .filter((v) => v != null && v !== '')
    .map((v) => (typeof v === 'number' ? fmt(v) : v))
    .join(': ')

function ReportOpen({ id }) {
  const state = useLoad(() => getReport(id), [id])
  if (state.status === 'loading') return <p className="muted report-wait">Loading report</p>
  if (state.status === 'error') {
    return <p className="error">{/status 404$/.test(state.message) ? 'Report not found' : `Could not load the report: ${state.message}`}</p>
  }
  const report = state.data
  return (
    <div className="report-open" id={`report-${id}`}>
      <h3>Summary</h3>
      <p className="report-summary">{report.summary}</p>
      {report.fixes.length > 0 && (
        <>
          <h3>Fixes</h3>
          <ol className="report-fixes">
            {report.fixes.map((fix, i) => (
              <li key={i}>
                {fix.text}
                {cite(fix) && <span className="muted"> ({cite(fix)})</span>}
              </li>
            ))}
          </ol>
        </>
      )}
      <h3>
        Before / now <span className="muted">(now = last 7 days)</span>
      </h3>
      <ul className="ba-list" aria-label="Before and now, one line per rule">
        {report.rules.map((rule) => (
          <li key={rule.rule_id} className="ba-rule">
            <span className="ba-title">{rule.title}</span>
            <span className="ba-values">
              <span className="ba-side">
                <span className="sr-only">then </span>
                <Side ruleId={rule.rule_id} side={rule.before} />
              </span>{' '}
              <span className="ba-arrow" aria-hidden="true">
                →
              </span>{' '}
              <span className="ba-side">
                <span className="sr-only">now </span>
                <Side ruleId={rule.rule_id} side={rule.now} />
              </span>
            </span>
          </li>
        ))}
      </ul>
    </div>
  )
}

const scopeText = (r) => [r.workspace, r.account && `account ${r.account}`].filter(Boolean).join(' / ') || 'all workspaces'

/** The Saved reports section of Warnings & plan; `command` is the investigate prompt the empty text names. */
export default function SavedReports({ command }) {
  const state = useLoad(getReports, [])
  const [open, setOpen] = useState(null) // the id of the one open report
  return (
    <section aria-labelledby="h-saved-reports" className="saved-reports">
      <h2 id="h-saved-reports">Saved reports</h2>
      {state.status === 'loading' && <p className="muted">Loading saved reports</p>}
      {state.status === 'error' && <p className="error">Could not load saved reports: {state.message}</p>}
      {state.status === 'ready' && state.data.reports.length === 0 && (
        <p className="muted">
          No saved reports yet. Run <code>{command}</code> in Claude Code.
        </p>
      )}
      {state.status === 'ready' && state.data.reports.length > 0 && (
        <ul className="report-list">
          {state.data.reports.map((r) => {
            const isOpen = open === r.id
            return (
              <li key={r.id}>
                <button
                  type="button"
                  className="report-row"
                  aria-expanded={isOpen}
                  aria-controls={isOpen ? `report-${r.id}` : undefined}
                  onClick={() => setOpen(isOpen ? null : r.id)}
                >
                  <span className="report-meta">
                    {localTime(r.saved_utc)} · {scopeText(r)} · {r.fired_count} {r.fired_count === 1 ? 'rule' : 'rules'} fired
                  </span>
                  <span className="report-line">{r.summary_first_line}</span>
                  <span className="report-caret" aria-hidden="true">
                    {isOpen ? '▾' : '▸'}
                  </span>
                </button>
                {isOpen && <ReportOpen id={r.id} />}
              </li>
            )
          })}
        </ul>
      )}
    </section>
  )
}
