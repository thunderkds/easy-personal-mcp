import { useCallback, useEffect, useState } from 'react'
import { getJson } from './api.js'
import AgentsView from './AgentsView.jsx'
import BoardView from './BoardView.jsx'
import FirstRun, { stepsLeft } from './FirstRun.jsx'
import JobsView from './JobsView.jsx'
import KitView from './KitView.jsx'
import LiveView from './LiveView.jsx'
import MemoryView from './MemoryView.jsx'
import Rail from './Rail.jsx'
import { NO_SCOPE, ScopeSelects, scopeNames } from './Scope.jsx'
import UsageView from './UsageView.jsx'
import WarningsPlanView from './WarningsPlanView.jsx'
import WorkspacesView from './WorkspacesView.jsx'

// `scoped` views filter by the global Workspace/Account scope; the others ignore it.
// `work` holds the Work view (HUN-162..164, Live HUN-178); a group with no views is hidden.
const GROUPS = [
  {
    id: 'work',
    label: '// work',
    tabs: [
      { id: 'live', label: 'Live', View: LiveView },
      { id: 'board', label: 'Board', View: BoardView, scoped: true },
      { id: 'scheduled', label: 'Scheduled', View: JobsView },
      { id: 'memory', label: 'Memory', View: MemoryView, scoped: true },
    ],
  },
  {
    id: 'usage',
    label: '// usage',
    tabs: [
      { id: 'usage', label: 'Usage', View: UsageView, scoped: true },
      { id: 'warnings', label: 'Warnings & plan', View: WarningsPlanView, scoped: true },
    ],
  },
  { id: 'kit', label: '// kit', tabs: [{ id: 'kit', label: 'Kit', View: KitView, scoped: true }] },
  {
    id: 'connect',
    label: '// connect',
    tabs: [
      { id: 'workspaces', label: 'Workspaces', View: WorkspacesView },
      { id: 'agents', label: 'Agents', View: AgentsView },
    ],
  },
  { id: 'system', label: '// system', tabs: [{ id: 'first-run', label: 'First run', View: FirstRun }] },
]
const TABS = GROUPS.flatMap((g) => g.tabs)

// The rest of the Easy family. Plain links: the page loads nothing from them.
const FAMILY = [
  { name: 'Easy Kit', href: 'https://personal-agentic-claude.vercel.app/' },
  { name: 'Easy Memory', href: 'https://easy-memory.vercel.app/' },
  { name: 'Easy Verifier', href: 'https://easy-verifier.vercel.app/' },
]

const STORAGE_KEY = 'easy-personal-scope'

/** The remembered scope, or none: storage may be missing, blocked or hold junk. */
function readScope() {
  try {
    const { workspace, account } = JSON.parse(window.localStorage.getItem(STORAGE_KEY))
    return { workspace: typeof workspace === 'string' ? workspace : '', account: typeof account === 'string' ? account : '' }
  } catch {
    return NO_SCOPE
  }
}

function writeScope(scope) {
  try {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(scope))
  } catch {
    // remembering is optional
  }
}

async function loadOptions() {
  const [ws, acc] = await Promise.all([getJson('/api/workspaces'), getJson('/api/accounts?by=account')])
  return {
    workspaces: ws.workspaces.filter((w) => !w.is_orphan),
    accounts: acc.accounts.filter((a) => a.provider === 'claude' && a.status === 'confirmed'),
    ignored: acc.accounts.filter((a) => a.provider === 'claude' && a.status === 'ignored').length,
  }
}

/** Keep only what still exists in `options`; a stale workspace or account becomes All. */
function validScope(scope, options) {
  return {
    workspace: options.workspaces.some((w) => w.name === scope.workspace) ? scope.workspace : '',
    account: options.accounts.some((a) => String(a.id) === scope.account) ? scope.account : '',
  }
}

/**
 * Sidebar shell. In-page views only: the URL fragment holds the dashboard token, so there is no routing.
 * App owns the Workspace/Account scope: it survives tab switches and is remembered in localStorage.
 * `options` is undefined while loading, null when the request failed (selects hidden, views unscoped).
 */
export default function App() {
  const [active, setActive] = useState('usage')
  const [menuOpen, setMenuOpen] = useState(false)
  const [chosen, setChosen] = useState(readScope)
  const [period, setPeriod] = useState('7') // Usage period id, app-wide (FR-064); not remembered across reloads
  const [options, setOptions] = useState(undefined)
  const [closed, setClosed] = useState({}) // group id -> collapsed
  const [firstRun, setFirstRun] = useState(undefined) // checklist steps; null when the request failed

  useEffect(() => {
    let current = true
    loadOptions().then(
      (data) => current && setOptions(data),
      () => current && setOptions(null),
    )
    return () => {
      current = false
    }
  }, [])

  // The checklist is read on start and again on every view change, so the badge follows what you just did.
  useEffect(() => {
    let current = true
    getJson('/api/first-run').then(
      (data) => current && setFirstRun(data.steps),
      () => current && setFirstRun(null),
    )
    return () => {
      current = false
    }
  }, [active])

  // After Agents changes an account: read the options again. A failed reload keeps the old ones.
  const reloadOptions = useCallback(() => {
    loadOptions().then(setOptions, () => {})
  }, [])

  const scope = options ? validScope(chosen, options) : NO_SCOPE
  const choose = (next) => {
    setChosen(next)
    writeScope(next)
  }
  const { label, View, scoped } = TABS.find((t) => t.id === active)
  const left = stepsLeft(firstRun)
  return (
    <div className="shell">
      <aside id="sidebar" className="sidebar" data-open={menuOpen}>
        <div className="brand">
          <svg width="24" height="24" viewBox="0 0 26 26" fill="none" aria-hidden="true" focusable="false">
            <rect x="1.5" y="1.5" width="23" height="23" rx="5" stroke="currentColor" strokeWidth="1.5" />
            <circle cx="13" cy="10" r="3.2" stroke="currentColor" strokeWidth="1.4" />
            <path d="M7 19.5c1.2-3 3.4-4.4 6-4.4s4.8 1.4 6 4.4" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" />
          </svg>
          <span>Easy Personal</span>
        </div>
        {options && <ScopeSelects options={options} scope={scope} onChange={choose} />}
        <nav aria-label="Views">
          {GROUPS.map((g) => (
            <div key={g.id} className="nav-group" data-group={g.id} hidden={g.tabs.length === 0}>
              <button
                type="button"
                className="group-toggle"
                aria-expanded={!closed[g.id]}
                aria-controls={`group-${g.id}`}
                onClick={() => setClosed((c) => ({ ...c, [g.id]: !c[g.id] }))}
              >
                <svg className="chevron" width="10" height="10" viewBox="0 0 10 10" aria-hidden="true" focusable="false">
                  <path d="M2 3.5l3 3 3-3" fill="none" stroke="currentColor" strokeWidth="1.4" />
                </svg>
                {g.label}
              </button>
              <div id={`group-${g.id}`} className="group-items">
                {!closed[g.id] &&
                  g.tabs.map((t) => (
                    <button
                      key={t.id}
                      type="button"
                      aria-current={t.id === active ? 'page' : undefined}
                      onClick={() => {
                        setActive(t.id)
                        setMenuOpen(false)
                      }}
                    >
                      <span className="dot" aria-hidden="true" />
                      {t.label}
                      {t.id === 'first-run' && left > 0 && <span className="badge">{left} left</span>}
                    </button>
                  ))}
              </div>
            </div>
          ))}
        </nav>
        <div className="family">
          <span className="family-label">// family</span>
          {FAMILY.map((f) => (
            <a key={f.name} href={f.href} target="_blank" rel="noopener noreferrer">
              {f.name}
            </a>
          ))}
        </div>
      </aside>
      <div className="content">
        <header className="topbar">
          <button type="button" className="menu-button" aria-label="Menu" aria-controls="sidebar" aria-expanded={menuOpen} onClick={() => setMenuOpen((o) => !o)}>
            <svg viewBox="0 0 24 24" width="18" height="18" aria-hidden="true" focusable="false">
              <path d="M4 6h16M4 12h16M4 18h16" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" />
            </svg>
          </button>
          <p className="topbar-title">{label}</p>
          {scoped && options && <p className="topbar-scope">scope: {scopeNames(scope, options).join(' · ')}</p>}
        </header>
        <div className="body">
          <div id="view">
            {scoped && options === undefined ? (
              <main>
                <p role="status">Loading</p>
              </main>
            ) : (
              <View
                scope={scope}
                options={options ?? null}
                steps={firstRun}
                onOpenAgents={() => setActive('agents')}
                onOpenWorkspaces={() => setActive('workspaces')}
                onAccountsChanged={reloadOptions}
                period={period}
                onPeriodChange={setPeriod}
              />
            )}
          </div>
          {options !== undefined && <Rail scope={scope} period={period} refresh={options} onOpenPlan={() => setActive('warnings')} />}
        </div>
      </div>
    </div>
  )
}
