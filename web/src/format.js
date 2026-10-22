const number = new Intl.NumberFormat('en-US')

export const fmt = (n) => number.format(n)

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
