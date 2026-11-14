import { getJson } from './api.js'
import { limitTone, percent, until } from './format.js'
import { scopeQuery } from './Scope.jsx'
import { PERIODS, sinceOf } from './UsageView.jsx'
import { useLoad } from './useLoad.js'
import { Progress, Ring } from './Widgets.jsx'

const TOP = 3
export const hasPct = (w) => Boolean(w) && !w.stale && typeof w.used_percentage === 'number'

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

const MIN_MESSAGES = 20 // a model needs this many messages in the period to count for "Lowest"
const MIN_MODELS = 2
const promptOf = (row) => row.input_tokens + row.cache_creation_input_tokens + row.cache_read_input_tokens

/** The model with the lowest cache hit ratio among those with enough messages, or null with fewer than two such models. */
function lowestModel(groups) {
  const rated = groups
    .filter((g) => g.key !== '(unknown)' && g.messages >= MIN_MESSAGES && promptOf(g) > 0)
    .map((g) => ({ name: g.key, ratio: g.cache_read_input_tokens / promptOf(g) }))
  return rated.length < MIN_MODELS ? null : rated.reduce((low, m) => (m.ratio < low.ratio ? m : low))
}

function CacheReuse({ scope, period }) {
  const chosen = PERIODS.find((p) => p.id === period) ?? PERIODS[0]
  const state = useLoad(async () => {
    const since = chosen.days === null ? '' : `since=${encodeURIComponent(sinceOf(chosen.days))}`
    const url = (path, extra = '') => {
      const query = [extra, since, scopeQuery(scope)].filter(Boolean).join('&')
      return query ? `${path}?${query}` : path
    }
    // The breakdown only adds the "Lowest" line: when it fails, the ring and the written line stay.
    const [summary, models] = await Promise.all([getJson(url('/api/usage/summary')), getJson(url('/api/usage/breakdown', 'by=model')).catch(() => null)])
    return { summary, models }
  }, [period, scope.workspace, scope.account])
  const summary = state.status === 'ready' ? state.data.summary : null
  const ratio = summary ? summary.cache_hit_ratio : null
  const prompt = summary ? promptOf(summary) : 0
  const lowest = summary && prompt > 0 && state.data.models ? lowestModel(state.data.models.groups) : null
  const days = chosen.label.toLowerCase()
  return (
    <Card title="Cache reuse" state={state}>
      <Ring ratio={ratio} label={ratio === null ? 'Cache reuse: no data' : `Cache reuse ${percent(ratio)}, ${days}`} />
      <p className="muted rail-note">{`Share of prompt tokens read from cache, ${days}.`}</p>
      {prompt > 0 && <p className="muted rail-note">{`Written to cache ${percent(summary.cache_creation_input_tokens / prompt)}`}</p>}
      {lowest && <p className="muted rail-note">{`Lowest: ${lowest.name} ${percent(lowest.ratio)}`}</p>}
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
export default function Rail({ scope, refresh, onOpenPlan, period = '7' }) {
  return (
    <div className="rail">
      <CacheReuse scope={scope} period={period} />
      <NextActions scope={scope} onOpenPlan={onOpenPlan} />
      <PlanLimits refresh={refresh} />
    </div>
  )
}
