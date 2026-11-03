/** A small floating pill (fixed, so the page does not move): spinner plus the label of what is loading. */
export default function Loading({ label }) {
  return (
    <div role="status" className="loading-pill">
      <span className="spinner" aria-hidden="true" />
      {label}
    </div>
  )
}
