import { useEffect, useState } from 'react'
import { getJson } from './api.js'
import AgentsView from './AgentsView.jsx'
import KitView from './KitView.jsx'
import { NO_SCOPE, ScopeSelects, scopeNames } from './Scope.jsx'
import UsageView from './UsageView.jsx'
import WarningsPlanView from './WarningsPlanView.jsx'
import WorkspacesView from './WorkspacesView.jsx'

// `scoped` views filter by the global Workspace/Account scope; the others ignore it.
const TABS = [
  { id: 'usage', label: 'Usage', View: UsageView, scoped: true },
  { id: 'warnings', label: 'Warnings & plan', View: WarningsPlanView, scoped: true },
  { id: 'kit', label: 'Kit', View: KitView, scoped: true },
  { id: 'workspaces', label: 'Workspaces', View: WorkspacesView },
  { id: 'agents', label: 'Agents', View: AgentsView },
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
  const [options, setOptions] = useState(undefined)

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

  const scope = options ? validScope(chosen, options) : NO_SCOPE
  const choose = (next) => {
    setChosen(next)
    writeScope(next)
  }
  const { label, View, scoped } = TABS.find((t) => t.id === active)
  return (
    <div className="shell">
      <aside id="sidebar" className="sidebar" data-open={menuOpen}>
        <p className="brand">Easy Personal</p>
        {options && <ScopeSelects options={options} scope={scope} onChange={choose} />}
        <nav aria-label="Views">
          {TABS.map((t) => (
            <button
              key={t.id}
              type="button"
              aria-current={t.id === active ? 'page' : undefined}
              onClick={() => {
                setActive(t.id)
                setMenuOpen(false)
              }}
            >
              {t.label}
            </button>
          ))}
        </nav>
      </aside>
      <div className="content">
        <header className="topbar">
          <button type="button" className="menu-button" aria-label="Menu" aria-controls="sidebar" aria-expanded={menuOpen} onClick={() => setMenuOpen((o) => !o)}>
            <svg viewBox="0 0 24 24" width="24" height="24" aria-hidden="true" focusable="false">
              <path d="M4 6h16M4 12h16M4 18h16" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
            </svg>
          </button>
          <p className="topbar-title">{label}</p>
          {scoped && options && <p className="topbar-scope">Scope: {scopeNames(scope, options).join(' · ')}</p>}
        </header>
        <div id="view">
          {scoped && options === undefined ? (
            <main>
              <p role="status">Loading</p>
            </main>
          ) : (
            <View scope={scope} options={options ?? null} onOpenAgents={() => setActive('agents')} />
          )}
        </div>
      </div>
    </div>
  )
}
