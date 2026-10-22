import { getJson } from './api.js'
import { fmt } from './format.js'
import { useLoad } from './useLoad.js'

const load = () => Promise.all([getJson('/api/warnings'), getJson('/api/plan')])

function Detail({ detail }) {
  if (!detail) return null
  const text = Object.entries(detail)
    .map(([k, v]) => `${k}: ${Array.isArray(v) ? v.join(', ') : v}`)
    .join('; ')
  return <span className="muted">{text}</span>
}

function Entry({ item, window_days }) {
  return (
    <li>
      <div className="entry-head">
        <span className={`label label-${item.label}`}>{item.label}</span>
        <strong>{item.rule_id}</strong>
        <span>
          Measured {fmt(item.measured)} (threshold {fmt(item.threshold)}
          {window_days ? `, last ${window_days} days` : ''})
        </span>
      </div>
      <p className="fix">{item.fix}</p>
      <Detail detail={item.detail} />
    </li>
  )
}

function Warnings({ data }) {
  return (
    <section aria-labelledby="h-warnings">
      <h2 id="h-warnings">Warnings</h2>
      {data.hits.length === 0 ? (
        <p className="muted">No warnings fired.</p>
      ) : (
        <ul className="entries">
          {data.hits.map((h) => (
            <Entry key={h.rule_id} item={h} window_days={h.window_days} />
          ))}
        </ul>
      )}
      {data.abstained.length > 0 && (
        <>
          <h3>Could not be measured</h3>
          <ul className="entries">
            {data.abstained.map((a) => (
              <li key={a.rule_id}>
                <strong>{a.rule_id}</strong>: {a.reason}
              </li>
            ))}
          </ul>
        </>
      )}
    </section>
  )
}

function Plan({ data }) {
  return (
    <section aria-labelledby="h-plan">
      <h2 id="h-plan">Improvement plan</h2>
      {data.groups.length === 0 && <p className="muted">No actions in the plan. No warning fired.</p>}
      {data.groups.map((g) => (
        <section key={g.priority} aria-labelledby={`h-prio-${g.priority}`} className="group">
          <h3 id={`h-prio-${g.priority}`}>{`Priority ${g.priority}`}</h3>
          <ul className="entries">
            {g.items.map((item) => (
              <Entry key={item.rule_id} item={item} />
            ))}
          </ul>
        </section>
      ))}
    </section>
  )
}

export default function WarningsPlanView() {
  const state = useLoad(load)
  return (
    <main>
      <h1>Warnings and plan</h1>
      {state.status === 'loading' && <p role="status">Loading warnings</p>}
      {state.status === 'error' && (
        <p role="alert" className="error">
          Could not load warnings: {state.message}
        </p>
      )}
      {state.status === 'ready' && (
        <>
          <Warnings data={state.data[0]} />
          <Plan data={state.data[1]} />
        </>
      )}
    </main>
  )
}
