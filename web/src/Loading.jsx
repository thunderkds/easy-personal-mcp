import { useState } from 'react'
import { pickLoadingLine } from './loadingLines.js'

/** A small floating pill (fixed, so the page does not move): spinner, the label of what is loading, and one funny line (picked once, kept while the pill shows). */
export default function Loading({ label }) {
  const [line] = useState(() => pickLoadingLine())
  return (
    <div role="status" className="loading-pill">
      <span className="spinner" aria-hidden="true" />
      <span className="loading-text">
        {label}
        <span className="loading-line" aria-hidden="true">{line}</span>
      </span>
    </div>
  )
}
