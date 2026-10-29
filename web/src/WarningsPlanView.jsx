import { getJson } from './api.js'
import { NO_SCOPE, ScopeLine, scopedPath } from './Scope.jsx'
import { useLoad } from './useLoad.js'

const load = (scope) => getJson(scopedPath('/api/warnings', scope))

// Every rule looks back 7 days; the all-clear line needs a window when no hit names one.
const WINDOW_DAYS = 7
const CHIPS = { warning: 'Fix now', improve: 'Improve', aware: 'Watch' }
const DETAIL_TITLES = { projects: 'Projects', project: 'Project', version: 'Claude Code version' }

function headline(hits) {
  const fix = hits.filter((h) => h.label !== 'aware').length
  const watch = hits.length - fix
  if (hits.length === 0) return `All clear — nothing to fix in the last ${WINDOW_DAYS} days`
  const parts = []
  if (fix) parts.push(`${fix} ${fix === 1 ? 'thing' : 'things'} to fix`)
  if (watch) parts.push(`${watch} to watch`)
  return parts.join(', ')
}

const steps = (hit) => hit.steps ?? (hit.fix ? [hit.fix] : [])
const capital = (text) => text.charAt(0).toUpperCase() + text.slice(1)
// "no stored history", "no sessions in the window" ... : the rule found nothing to read.
const isNoData = (reason) => /^no\b/i.test(reason)

function Card({ hit }) {
  const list = steps(hit)
  return (
    <li>
      <article className="action-card" data-rule={hit.rule_id}>
        <header>
          <span className={`chip chip-${hit.label}`}>{CHIPS[hit.label] ?? hit.label}</span>
          <h3>{hit.title ?? hit.rule_id}</h3>
        </header>
        {hit.summary && <p className="card-summary">{hit.summary}</p>}
        {hit.why && (
          <p className="card-why">
            <strong>Why it matters</strong> {hit.why}
          </p>
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
      </article>
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

export default function WarningsPlanView({ scope = NO_SCOPE, options = null, onOpenAgents }) {
  const state = useLoad(() => load(scope), [scope.workspace, scope.account])
  return (
    <main>
      <h1>Warnings and plan</h1>
      <ScopeLine scope={scope} options={options} />
      {state.status === 'loading' && <p role="status">Loading warnings</p>}
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
              {state.data.hits.map((h) => (
                <Card key={h.rule_id} hit={h} />
              ))}
            </ol>
          )}
          <NotChecked abstained={state.data.abstained} onOpenAgents={onOpenAgents} />
        </>
      )}
    </main>
  )
}
