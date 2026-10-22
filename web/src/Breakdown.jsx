import Cost from './Cost.jsx'
import { fmt, totalTokens } from './format.js'

const LABELS = { day: 'Day', model: 'Model', project: 'Project', tool: 'Tool' }
const TITLES = { day: 'By day', model: 'By model', project: 'By project', tool: 'By tool' }

const W = 600
const H = 160
const PAD = 24

function DayChart({ groups }) {
  const max = Math.max(...groups.map(totalTokens), 1)
  const slot = (W - PAD) / groups.length
  const barW = Math.max(slot * 0.7, 1)
  return (
    <svg className="chart" viewBox={`0 0 ${W} ${H}`} role="img" aria-label={`Tokens per day, ${groups.length} days, peak ${fmt(max)} tokens`}>
      {groups.map((g, i) => {
        const h = (totalTokens(g) / max) * (H - PAD - 8)
        const x = PAD + i * slot + (slot - barW) / 2
        return (
          <g key={i}>
            <rect className="bar" x={x} y={H - PAD - h} width={barW} height={h}>
              <title>{`${g.key}: ${fmt(totalTokens(g))} tokens`}</title>
            </rect>
            {groups.length <= 14 && (
              <text className="axis" x={x + barW / 2} y={H - 8} textAnchor="middle">{g.key.slice(5)}</text>
            )}
          </g>
        )
      })}
    </svg>
  )
}

export default function Breakdown({ by, data }) {
  const title = TITLES[by]
  return (
    <section aria-labelledby={`h-${by}`}>
      <h2 id={`h-${by}`}>{title}</h2>
      {data.groups.length === 0 ? (
        <p className="muted">No usage recorded in this period.</p>
      ) : (
        <>
          {by === 'day' && <DayChart groups={data.groups} />}
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
                {data.groups.map((g, i) => (
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
        </>
      )}
      {by === 'tool' && <p className="muted">A message that used several tools appears in each tool it used, so rows overlap.</p>}
    </section>
  )
}
