const number = new Intl.NumberFormat('en-US')

export const fmt = (n) => number.format(n)

const UNITS = [
  [1e9, 'B'],
  [1e6, 'M'],
  [1e3, 'K'],
]
/** Short form for a big figure: exact below 10,000, else 3 significant digits and a unit ("3.11 B", "21.3 K", "412 M"). */
export function compact(n) {
  if (Math.abs(n) < 10000) return fmt(n)
  const [size, unit] = UNITS.find(([s]) => Math.abs(n) >= s)
  const value = n / size
  // 999.6 K rounds to 1000: move up one unit instead of printing "1000 K".
  if (Number(value.toPrecision(3)) >= 1000 && size < 1e9) return compact(Math.sign(n) * size * 1000)
  return `${Number(value.toPrecision(3))} ${unit}`
}

export const totalTokens = (row) =>
  row.input_tokens + row.output_tokens + row.cache_creation_input_tokens + row.cache_read_input_tokens

export const percent = (ratio) => (ratio === null || ratio === undefined ? 'no data' : `${(ratio * 100).toFixed(1)}%`)

export function duration(startUtc, endUtc) {
  const minutes = Math.round((new Date(endUtc) - new Date(startUtc)) / 60000)
  return `${Math.floor(minutes / 60)}h ${minutes % 60}m`.replace(/^0h /, '')
}

export const localTime = (utc) => new Date(utc).toLocaleString()

/** Cost cell text: abstained models are named, never shown as $0. */
export function costText(cost) {
  const missing = cost.abstained_models.length ? `abstained: ${cost.abstained_models.join(', ')}` : ''
  if (cost.usd === null) return missing || 'abstained'
  const usd = `$${cost.usd.toFixed(2)}`
  return missing ? `${usd} + ${missing}` : usd
}

/** The dollar figure alone, or "no total" when there is none; abstained models are listed by `abstainedText`. */
export const costFigure = (cost) => (cost.usd === null ? 'no total' : `$${cost.usd.toFixed(2)}`)

/** "abstained: a, b" for the muted line under a cost figure; '' when no model abstained. */
export const abstainedText = (cost) => (cost.abstained_models.length ? `abstained: ${cost.abstained_models.join(', ')}` : '')

/** "just now", "5 min ago", "2 h ago", "3 d ago"; "never" when there is no time. `now` is injectable for tests. */
export function ago(utc, now = Date.now()) {
  if (!utc) return 'never'
  const minutes = Math.max(0, Math.floor((now - new Date(utc)) / 60000))
  if (minutes < 1) return 'just now'
  if (minutes < 60) return `${minutes} min ago`
  if (minutes < 1440) return `${Math.floor(minutes / 60)} h ago`
  return `${Math.floor(minutes / 1440)} d ago`
}

/** Time until a reset: "45m", "1h 20m", "3d 4h"; "soon" when the time has passed or is unknown. */
export function until(utc, now = Date.now()) {
  const minutes = utc ? Math.floor((new Date(utc) - now) / 60000) : 0
  if (minutes < 1) return 'soon'
  if (minutes < 60) return `${minutes}m`
  if (minutes < 1440) return `${Math.floor(minutes / 60)}h ${minutes % 60}m`
  return `${Math.floor(minutes / 1440)}d ${Math.floor((minutes % 1440) / 60)}h`
}

/** Same thresholds as the status line: green below 70 %, amber 70-89 %, red from 90 %. */
export const limitTone = (pct) => (pct >= 90 ? 'red' : pct >= 70 ? 'amber' : 'green')
