import { getJson } from './api.js'
import FeedCard from './FeedCard.jsx'
import { NO_SCOPE, scopedPath } from './Scope.jsx'
import { useLoad } from './useLoad.js'
import Loading from './Loading.jsx'
import { useNotify } from './Toast.jsx'
import SavedReports from './SavedReports.jsx'

const load = (scope) => getJson(scopedPath('/api/warnings', scope))

const INVESTIGATE = '/mcp__easy-personal__investigate' // the MCP prompt of FR-075, run in the owner's Claude Code
const CHIPS = { warning: 'Fix now', improve: 'Improve', aware: 'Watch' }
const DETAIL_TITLES = { projects: 'Projects', project: 'Project', version: 'Claude Code version' }

function headline(hits) {
  const fix = hits.filter((h) => h.label !== 'aware').length
  const watch = hits.length - fix
  if (hits.length === 0) return 'Nothing to fix this week.'
  const parts = []
  if (fix) parts.push(`${fix} ${fix === 1 ? 'thing' : 'things'} to fix`)
  if (watch) parts.push(`${watch} to watch`)
  return parts.join(', ')
}

// Plan priority, 1 first; the sort is stable, so equal priorities keep the server's order.
export const byPriority = (hits) => [...hits].sort((a, b) => (a.priority ?? 99) - (b.priority ?? 99))
const steps = (hit) => hit.steps ?? (hit.fix ? [hit.fix] : [])
const capital = (text) => text.charAt(0).toUpperCase() + text.slice(1)
// "no stored history", "no sessions in the window" ... : the rule found nothing to read.
const isNoData = (reason) => /^no\b/i.test(reason)

function Causes({ causes, id }) {
  if (!causes?.length) return null
  return (
    <div className="card-causes" role="group" aria-labelledby={id}>
      <strong id={id}>Why this happened</strong>
      <ul>
        {causes.map((c) => (
          <li key={`${c.kind}-${c.name}`}>
            {c.text ?? c.name}
            {c.new && <span className="chip chip-new">new</span>}
          </li>
        ))}
      </ul>
    </div>
  )
}

// `idPrefix` keeps element ids unique when a second list of cards is on the page (an open saved report, HUN-224);
// `children` is extra body content under the steps (the report's "Now:" line).
export function Card({ hit, rank, idPrefix = '', children }) {
  const list = steps(hit)
  const title = hit.title ?? hit.rule_id
  return (
    <li>
      <FeedCard
        id={`${idPrefix}action-${rank}`}
        className="action-card"
        data-rule={hit.rule_id}
        icon={String(rank)}
        tone={hit.label === 'warning' ? 'amber' : undefined}
        title={title}
        summary={<span className={`chip chip-${hit.label}`}>{CHIPS[hit.label] ?? hit.label}</span>}
        time={hit.window_days ? `last ${hit.window_days} days` : undefined}
        footer={list.length > 0 && <span>{`${list.length} ${list.length === 1 ? 'step' : 'steps'}`}</span>}
      >
        {hit.summary && <p className="card-summary">{hit.summary}</p>}
        {hit.why && (
          <p className="card-why">
            <strong>Why it matters</strong> {hit.why}
          </p>
        )}
        <Causes causes={hit.causes} id={`${idPrefix}why-${rank}`} />
        {typeof hit.cache_read_share === 'number' && (
          <p className="card-cache">{Math.round(hit.cache_read_share * 100)}% of these tokens are cache reads</p>
        )}
        {list.length > 0 && (
          <ol className="steps">
            {list.map((s) => (
              <li key={s}>{s}</li>
            ))}
          </ol>
        )}
        {hit.detail &&
          Object.entries(hit.detail).map(([key, value]) => (
            <div key={key} className="card-detail">
              <span className="muted">{DETAIL_TITLES[key] ?? key}</span>
              <ul>
                {[].concat(value).map((v) => (
                  <li key={v}>{v}</li>
                ))}
              </ul>
            </div>
          ))}
        {children}
      </FeedCard>
    </li>
  )
}

function NotChecked({ abstained, onOpenAgents }) {
  if (abstained.length === 0) return null
  return (
    <section aria-labelledby="h-not-checked">
      <details className="not-checked">
        <summary id="h-not-checked">Not checked ({abstained.length})</summary>
        {onOpenAgents && abstained.some((a) => isNoData(a.reason)) && (
          <p>
            Some checks found no data to read.{' '}
            <button type="button" onClick={onOpenAgents}>
              Confirm the accounts you use
            </button>
          </p>
        )}
        <ul className="entries">
          {abstained.map((a) => (
            <li key={a.rule_id}>
              <strong>{a.title ?? a.rule_id}</strong>
              <span className="muted"> {capital(a.reason)}</span>
            </li>
          ))}
        </ul>
      </details>
    </section>
  )
}

function InvestigateHint() {
  const notify = useNotify()
  const copy = async () => {
    try {
      await navigator.clipboard.writeText(INVESTIGATE)
      notify('success', 'Copied. Paste it into Claude Code.')
    } catch {
      notify('error', `Could not copy: copy it by hand from the page (${INVESTIGATE}).`)
    }
  }
  return (
    <p className="investigate-hint" data-testid="investigate-hint">
      Ask Claude to investigate: run <code>{INVESTIGATE}</code> in Claude Code.{' '}
      <button type="button" onClick={copy} aria-label="Copy the investigate command">
        Copy
      </button>
    </p>
  )
}

export default function WarningsPlanView({ scope = NO_SCOPE, onOpenAgents }) {
  const state = useLoad(() => load(scope), [scope.workspace, scope.account])
  return (
    <main className="warnings">
      <h1>Warnings and plan</h1>
      {state.status === 'loading' && <Loading label="Loading warnings" />}
      {state.status === 'error' && (
        <p role="alert" className="error">
          Could not load warnings: {state.message}
        </p>
      )}
      {state.status === 'ready' && (
        <>
          <p className="headline" data-testid="headline">
            {headline(state.data.hits)}
          </p>
          {state.data.hits.length > 0 && (
            <ol className="actions" aria-label="Things to do, first one first">
              {byPriority(state.data.hits).map((h, i) => (
                <Card key={h.rule_id} hit={h} rank={i + 1} />
              ))}
            </ol>
          )}
          <NotChecked abstained={state.data.abstained} onOpenAgents={onOpenAgents} />
          <InvestigateHint />
        </>
      )}
      <SavedReports command={INVESTIGATE} waiting={state.status === 'loading'} />
    </main>
  )
}
