import Cost from './Cost.jsx'
import { Compact, useTopN } from './Widgets.jsx'
import { abstainedText, costFigure, fmt, totalTokens } from './format.js'

const LABELS = { day: 'Day', model: 'Model', project: 'Project', tool: 'Tool' }
const TITLES = { day: 'By day', model: 'By model', project: 'By project', tool: 'By tool' }
const DENSE_DAYS = 14

/** One bar per day, tallest = 100 %; the latest day is the full-color bar. Labels only while they fit. */
function DayChart({ groups }) {
  const max = Math.max(...groups.map(totalTokens), 1)
  const dense = groups.length > DENSE_DAYS
  return (
    <div className={`day-chart${dense ? ' dense' : ''}`} role="img" aria-label={`Tokens per day, ${groups.length} days, peak ${fmt(max)} tokens`}>
      <div className="day-bars">
        {groups.map((g, i) => (
          <div
            key={i}
            className={`bar${i === groups.length - 1 ? ' bar-latest' : ''}`}
            style={{ height: `${Math.max((totalTokens(g) / max) * 100, 1)}%` }}
            title={`${g.key}: ${fmt(totalTokens(g))} tokens`}
          />
        ))}
      </div>
      {!dense && (
        <div className="day-labels">
          {groups.map((g, i) => (
            <span key={i}>{g.key.slice(5)}</span>
          ))}
        </div>
      )}
    </div>
  )
}

/** One row per project: name, tokens and a bar of its share of the period. Two projects may share a name. */
function ProjectBars({ groups }) {
  const total = groups.reduce((sum, g) => sum + totalTokens(g), 0)
  const { shown, toggle } = useTopN(groups)
  return (
    <>
      <ul className="bar-rows">
        {shown.map((g, i) => (
          <li key={i} className={i === 0 ? 'bar-row bar-row-top' : 'bar-row'}>
            <div className="bar-row-head">
              <span className="mono-ellipsis" title={g.key}>{g.key}</span>
              <span className="bar-row-value">{fmt(totalTokens(g))}</span>
            </div>
            <div className="bar-track" aria-hidden="true">
              <div className="bar-fill" style={{ width: `${total > 0 ? (totalTokens(g) / total) * 100 : 0}%` }} />
            </div>
          </li>
        ))}
      </ul>
      {toggle}
    </>
  )
}

/** The compact table of the artboard: model, total tokens, output, cost. Short headers; aria-label keeps the full names. */
function ModelTable({ groups }) {
  const { shown, toggle } = useTopN(groups)
  return (
    <>
      <div className="table-wrap">
        <table className="model-table">
          <thead>
            <tr>
              <th scope="col">Model</th>
              <th scope="col" aria-label="Total tokens" title="Total tokens">Total</th>
              <th scope="col" aria-label="Output tokens" title="Output tokens">Output</th>
              <th scope="col" aria-label="API-equivalent cost" title="API-equivalent cost">Cost</th>
            </tr>
          </thead>
          <tbody>
            {shown.map((g, i) => (
              <tr key={i}>
                <th scope="row" title={g.key}>{g.key}</th>
                <td><Compact n={totalTokens(g)} /></td>
                <td><Compact n={g.output_tokens} /></td>
                <td className="cost-cell">
                  <span className={g.api_equivalent_cost.usd === null ? 'muted' : undefined}>{costFigure(g.api_equivalent_cost)}</span>
                  {abstainedText(g.api_equivalent_cost) && <span className="muted cost-note">{abstainedText(g.api_equivalent_cost)}</span>}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      {toggle}
    </>
  )
}

/** Every counter of the dimension, tokens first and cost last. The day table is a timeline: never cut. */
function FullTable({ by, groups }) {
  const { shown, toggle } = useTopN(groups, by === 'day' ? Infinity : undefined)
  return (
    <>
      <div className="table-wrap">
        <table>
          <thead>
            <tr>
              <th scope="col">{LABELS[by]}</th>
              <th scope="col">Total tokens</th>
              <th scope="col">Input</th>
              <th scope="col">Output</th>
              <th scope="col">Cache write</th>
              <th scope="col">Cache read</th>
              <th scope="col">Messages</th>
              <th scope="col">API-equivalent cost</th>
            </tr>
          </thead>
          <tbody>
            {shown.map((g, i) => (
              <tr key={i}>
                <th scope="row">{g.key}</th>
                <td>{fmt(totalTokens(g))}</td>
                <td>{fmt(g.input_tokens)}</td>
                <td>{fmt(g.output_tokens)}</td>
                <td>{fmt(g.cache_creation_input_tokens)}</td>
                <td>{fmt(g.cache_read_input_tokens)}</td>
                <td>{fmt(g.messages)}</td>
                <td><Cost cost={g.api_equivalent_cost} /></td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      {toggle}
    </>
  )
}

export default function Breakdown({ by, data }) {
  const empty = data.groups.length === 0
  return (
    <section className="card" aria-labelledby={`h-${by}`}>
      <h2 id={`h-${by}`}>{TITLES[by]}</h2>
      {empty && <p className="muted">No usage recorded in this period.</p>}
      {!empty && by === 'day' && <DayChart groups={data.groups} />}
      {!empty && by === 'project' && <ProjectBars groups={data.groups} />}
      {!empty && by === 'model' && <ModelTable groups={data.groups} />}
      {!empty && by === 'tool' && <FullTable by={by} groups={data.groups} />}
      {!empty && (by === 'day' || by === 'project') && (
        <details className="full-table">
          <summary>Show table</summary>
          <FullTable by={by} groups={data.groups} />
        </details>
      )}
      {by === 'tool' && <p className="muted">A message that used several tools appears in each tool it used, so rows overlap.</p>}
    </section>
  )
}
