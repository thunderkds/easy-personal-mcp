import { useState } from 'react'
import KitView from './KitView.jsx'
import UsageView from './UsageView.jsx'
import WarningsPlanView from './WarningsPlanView.jsx'

const TABS = [
  { id: 'usage', label: 'Usage', View: UsageView },
  { id: 'warnings', label: 'Warnings & plan', View: WarningsPlanView },
  { id: 'kit', label: 'Kit', View: KitView },
]

/** In-page tabs only: the URL fragment holds the dashboard token, so there is no routing. */
export default function App() {
  const [active, setActive] = useState('usage')
  const { View } = TABS.find((t) => t.id === active)
  return (
    <>
      <div className="tabs" role="tablist" aria-label="Views">
        {TABS.map((t) => (
          <button
            key={t.id}
            type="button"
            role="tab"
            id={`tab-${t.id}`}
            aria-selected={t.id === active}
            aria-controls="view"
            onClick={() => setActive(t.id)}
          >
            {t.label}
          </button>
        ))}
      </div>
      <div id="view" role="tabpanel" aria-labelledby={`tab-${active}`}>
        <View />
      </div>
    </>
  )
}
