import { useState } from 'react'
import AgentsView from './AgentsView.jsx'
import KitView from './KitView.jsx'
import UsageView from './UsageView.jsx'
import WarningsPlanView from './WarningsPlanView.jsx'
import WorkspacesView from './WorkspacesView.jsx'

const TABS = [
  { id: 'usage', label: 'Usage', View: UsageView },
  { id: 'warnings', label: 'Warnings & plan', View: WarningsPlanView },
  { id: 'kit', label: 'Kit', View: KitView },
  { id: 'workspaces', label: 'Workspaces', View: WorkspacesView },
  { id: 'agents', label: 'Agents', View: AgentsView },
]

/**
 * Sidebar shell. In-page views only: the URL fragment holds the dashboard token, so there is no routing.
 * `slot` is the top bar's filter area: a view may portal its filters into it.
 */
export default function App() {
  const [active, setActive] = useState('usage')
  const [menuOpen, setMenuOpen] = useState(false)
  const [slot, setSlot] = useState(null)
  const { label, View } = TABS.find((t) => t.id === active)
  return (
    <div className="shell">
      <aside id="sidebar" className="sidebar" data-open={menuOpen}>
        <p className="brand">Easy Personal</p>
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
          <div className="topbar-slot" ref={setSlot} />
        </header>
        <div id="view">
          <View slot={slot} onOpenAgents={() => setActive('agents')} />
        </div>
      </div>
    </div>
  )
}
