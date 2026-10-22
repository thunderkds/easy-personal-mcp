import { costText } from './format.js'

/** One cost value. The "API-equivalent cost" label sits in the column header or next to it. */
export default function Cost({ cost }) {
  const abstained = cost.usd === null || cost.abstained_models.length > 0
  return <span className={abstained ? 'muted' : undefined}>{costText(cost)}</span>
}
