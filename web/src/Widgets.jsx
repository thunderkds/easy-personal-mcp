import { percent } from './format.js'

const W = 200
const H = 40
const PAD = 4

/** % change, as plain text. "no previous data" when there is nothing to compare against. */
export function Trend({ current, previous }) {
  if (current === null || current === undefined || !previous) {
    return <span className="trend trend-none">no previous data</span>
  }
  const change = ((current - previous) / previous) * 100
  const up = change >= 0
  return (
    <span className={`trend ${up ? 'trend-up' : 'trend-down'}`}>
      {`${up ? '↑' : '↓'} ${Math.abs(change).toFixed(1)}%`}
    </span>
  )
}

/** Line of one value per day, in the accent color. */
export function Sparkline({ values, label }) {
  if (values.length === 0) return null
  const max = Math.max(...values, 0)
  const x = (i) => (i / (values.length - 1)) * W
  const y = (v) => (max === 0 ? H - PAD : H - PAD - (v / max) * (H - 2 * PAD))
  // One value: draw a flat line across the whole width.
  const points = values.length === 1 ? [[0, y(values[0])], [W, y(values[0])]] : values.map((v, i) => [x(i), y(v)])
  const line = `M${points.map(([px, py]) => `${px} ${py}`).join(' L')}`
  return (
    <svg className="spark" viewBox={`0 0 ${W} ${H}`} preserveAspectRatio="none" role="img" aria-label={label}>
      <path className="spark-line" d={line} fill="none" vectorEffect="non-scaling-stroke" />
    </svg>
  )
}

const RADIUS = 42
const CIRCUMFERENCE = 2 * Math.PI * RADIUS

/** Ring gauge of a 0..1 ratio. `null` is "no data", never 0%. */
export function Ring({ ratio, label }) {
  const known = ratio !== null && ratio !== undefined
  return (
    <svg className="ring" viewBox="0 0 100 100" role="img" aria-label={label}>
      <circle className="ring-track" cx="50" cy="50" r={RADIUS} fill="none" />
      {known && (
        <circle
          className="ring-value"
          cx="50"
          cy="50"
          r={RADIUS}
          fill="none"
          strokeLinecap="round"
          strokeDasharray={`${Math.min(Math.max(ratio, 0), 1) * CIRCUMFERENCE} ${CIRCUMFERENCE}`}
          transform="rotate(-90 50 50)"
        />
      )}
      <text className="ring-text" x="50" y="50" textAnchor="middle" dominantBaseline="central">
        {known ? percent(ratio) : 'no data'}
      </text>
    </svg>
  )
}

/** Horizontal bar of a 0..1 ratio. `null` shows "no data" and no bar. */
export function Progress({ ratio, label }) {
  if (ratio === null || ratio === undefined) return <p className="muted">no data</p>
  const pct = Math.min(Math.max(ratio, 0), 1) * 100
  return (
    <div className="progress" role="progressbar" aria-label={label} aria-valuemin={0} aria-valuemax={100} aria-valuenow={Number(pct.toFixed(2))}>
      <div className="progress-fill" style={{ width: `${pct}%` }} />
    </div>
  )
}
