import { getJson } from './api.js'
import { limitTone, percent, until } from './format.js'
import { scopeQuery } from './Scope.jsx'
import { useLoad } from './useLoad.js'
import { Progress, Ring } from './Widgets.jsx'

const TOP = 3
const DAY_MS = 86400000
const hasPct = (w) => Boolean(w) && !w.stale && typeof w.used_percentage === 'number'

/** The right rail: three cards from data the views already load (summary, warnings, accounts). */
function Card({ title, state, children }) {
  const id = `rail-${title.toLowerCase().replace(/\W+/g, '-')}`
  const failed = state.status === 'error'
  return (
    <section className="rail-card" aria-labelledby={id}>
      <h2 id={id}>{title}</h2>
      {state.status === 'loading' && <p role="status" className="muted">Loading</p>}
      {failed && <p className="muted">no data</p>}
      {state.status === 'ready' && children}
    </section>
  )
}

function CacheReuse({ scope }) {
  const state = useLoad(() => {
    const since = `since=${encodeURIComponent(new Date(Date.now() - 7 * DAY_MS).toISOString())}`
    return getJson(`/api/usage/summary?${[since, scopeQuery(scope)].filter(Boolean).join('&')}`)
  }, [scope.workspace, scope.account])
  const ratio = state.status === 'ready' ? state.data.cache_hit_ratio : null
  return (
    <Card title="Cache reuse" state={state}>
      <Ring ratio={ratio} label={ratio === null ? 'Cache reuse: no data' : `Cache reuse ${percent(ratio)}, last 7 days`} />
      <p className="muted rail-note">Share of prompt tokens read from cache, last 7 days.</p>
    </Card>
  )
}

function NextActions({ scope, onOpenPlan }) {
  const state = useLoad(() => getJson(`/api/warnings${scopeQuery(scope) ? `?${scopeQuery(scope)}` : ''}`), [scope.workspace, scope.account])
  const hits = state.status === 'ready' ? state.data.hits : []
  return (
    <Card title={state.status === 'ready' ? `Next actions (${hits.length})` : 'Next actions'} state={state}>
      {hits.length === 0 ? (
        <p className="muted">Nothing to fix</p>
      ) : (
        <ol className="rail-list">
          {hits.slice(0, TOP).map((h) => (
            <li key={h.rule_id}>{h.title ?? h.rule_id}</li>
          ))}
        </ol>
      )}
      {onOpenPlan && (
        <button type="button" className="link-button" onClick={onOpenPlan}>
          Open the plan →
        </button>
      )}
    </Card>
  )
}

function Window({ name, label, window }) {
  if (!hasPct(window)) return null
  const pct = Math.round(window.used_percentage)
  const reset = until(window.resets_at)
  return (
    <div className={`limit limit-${limitTone(pct)}`}>
      <p>
        <span>{`${name} · ${pct}%`}</span>
        <span className="rail-reset">{`resets ${reset === 'soon' ? 'soon' : `in ${reset}`}`}</span>
      </p>
      <Progress ratio={window.used_percentage / 100} label={`${label} ${name} limit ${pct}%`} />
    </div>
  )
}

function PlanLimits({ refresh }) {
  const state = useLoad(() => getJson('/api/accounts?by=account'), [refresh])
  const accounts =
    state.status === 'ready'
      ? state.data.accounts.filter((a) => a.provider === 'claude' && a.status === 'confirmed' && a.status_line?.limits && [a.status_line.limits.five_hour, a.status_line.limits.seven_day].some(hasPct))
      : []
  return (
    <Card title="Plan limits" state={state}>
      {accounts.length === 0 ? (
        <p className="muted">no data</p>
      ) : (
        <ul className="rail-list">
          {accounts.map((a) => {
            const name = a.label || a.folder
            return (
              <li key={a.id}>
                <span className="rail-name" title={name}>
                  {name}
                </span>
                <Window name="5h" label={name} window={a.status_line.limits.five_hour} />
                <Window name="7d" label={name} window={a.status_line.limits.seven_day} />
              </li>
            )
          })}
        </ul>
      )}
      <p className="muted">Bars turn amber at 70%.</p>
    </Card>
  )
}

/** `refresh` changes when the accounts changed (App reloads its scope options), so the limits are read again. */
export default function Rail({ scope, refresh, onOpenPlan }) {
  return (
    <div className="rail">
      <CacheReuse scope={scope} />
      <NextActions scope={scope} onOpenPlan={onOpenPlan} />
      <PlanLimits refresh={refresh} />
    </div>
  )
}
