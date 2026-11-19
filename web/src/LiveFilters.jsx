// The filters above the Live list (T119, FR-128; T122): state (all / needs me / working / idle) and kind (terminal / app / headless) as button groups, project as a select, each with the number
// of sessions it would show. The choice is kept in `localStorage` (the three values only, never a session's text) and read back guarded: a missing,
// blocked or odd value gives no filter. The page works out each session's state, project and kind; this file only counts and draws.
export const FILTERS_KEY = 'easy-personal-live-filters'
export const STATES = [
  ['all', 'All'],
  ['needs', 'Needs me'],
  ['working', 'Working'],
  ['idle', 'Idle'],
]
export const KINDS = ['terminal', 'app', 'headless']
export const NO_FILTERS = { state: 'all', project: '', kind: '' }
export const PROJECT_MAX = 255 // the server's limit for a project filter (FR-126)

/** A project filter is one visible line. */
export const isProjectFilter = (v) => typeof v === 'string' && v.length <= PROJECT_MAX && ![...v].some((c) => c.charCodeAt(0) < 32 || c.charCodeAt(0) === 127)

/** The saved filters; anything missing, blocked or odd is no filter. */
export function loadFilters() {
  try {
    const raw = JSON.parse(window.localStorage.getItem(FILTERS_KEY))
    if (!raw || typeof raw !== 'object') return NO_FILTERS
    return {
      state: STATES.some(([v]) => v === raw.state) ? raw.state : 'all',
      project: isProjectFilter(raw.project) ? raw.project : '',
      kind: KINDS.includes(raw.kind) ? raw.kind : '',
    }
  } catch {
    return NO_FILTERS
  }
}

/** Keep the filters (the three values only); a blocked store is ignored, the page still works. */
export function saveFilters({ state, project, kind }) {
  try {
    window.localStorage.setItem(FILTERS_KEY, JSON.stringify({ state, project, kind }))
  } catch {
    /* no storage: the filters last until the page is closed */
  }
}

export const isFiltered = (f) => f.state !== 'all' || f.project !== '' || f.kind !== ''

/** Does a session's `{state, project, kind}` pass the filters? `skip` leaves one filter out: the counts beside a filter ignore that filter itself. */
export const passes = (facts, f, skip = null) => (skip === 'state' || f.state === 'all' || facts.state === f.state) && (skip === 'project' || f.project === '' || facts.project === f.project) && (skip === 'kind' || f.kind === '' || facts.kind === f.kind)

/**
 * `facts`: one `{state, project, kind}` per listed session. `filters`: the current choice. `onChange(patch)`: a change of one filter.
 * State and Kind are button groups like the period buttons of the other pages: the pressed one is the choice. Clear filters is always there, disabled while nothing is filtered,
 * so the bar never changes shape when a filter is set.
 */
export default function FilterBar({ facts, filters, onChange }) {
  const count = (skip, test) => facts.filter((x) => passes(x, filters, skip) && test(x)).length
  const projects = new Map()
  for (const x of facts) if (x.project !== '' && passes(x, filters, 'project')) projects.set(x.project, (projects.get(x.project) ?? 0) + 1)
  if (filters.project !== '' && !projects.has(filters.project)) projects.set(filters.project, 0) // a saved project with no session now stays choosable, and clearable
  return (
    <div className="live-filters live-filters-bar" role="group" aria-label="Filter sessions">
      <div className="periods" role="group" aria-label="State">
        {STATES.map(([value, label]) => (
          <button key={value} type="button" aria-pressed={filters.state === value} onClick={() => onChange({ state: value })}>
            {`${label} (${count('state', (x) => value === 'all' || x.state === value)})`}
          </button>
        ))}
      </div>
      <div className="periods" role="group" aria-label="Kind">
        {[['', 'All'], ...KINDS.map((k) => [k, `${k[0].toUpperCase()}${k.slice(1)}`])].map(([value, label]) => (
          <button key={value} type="button" aria-pressed={filters.kind === value} onClick={() => onChange({ kind: value })}>
            {`${label} (${count('kind', (x) => value === '' || x.kind === value)})`}
          </button>
        ))}
      </div>
      <label className="filter-project">
        <span>Project</span>
        <select value={filters.project} onChange={(e) => onChange({ project: e.target.value })}>
          <option value="">All projects</option>
          {[...projects.keys()].sort().map((p) => (
            <option key={p} value={p}>{`${p} (${projects.get(p)})`}</option>
          ))}
        </select>
      </label>
      <button type="button" className="filter-clear" disabled={!isFiltered(filters)} onClick={() => onChange({ ...NO_FILTERS })}>
        Clear filters
      </button>
    </div>
  )
}

/** A session's kind: an app session, a headless run (an `sdk…` entrypoint), else a terminal session. */
export const kindOf = (s, app) => (app ? 'app' : typeof s.entrypoint === 'string' && s.entrypoint.startsWith('sdk') ? 'headless' : 'terminal')
