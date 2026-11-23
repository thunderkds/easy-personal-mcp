import { lazy, Suspense } from 'react'
import { createRoot } from 'react-dom/client'
import App from './App.jsx'
import './app.css'
import './watch.css'
import './appsession.css'

// `?term=<id>`: a terminal popped out of Live into its own window (T135). It loads only the terminal, not the dashboard.
const TermWindow = lazy(() => import('./TermWindow.jsx'))
const termId = new URLSearchParams(window.location.search).get('term')

createRoot(document.getElementById('root')).render(
  termId ? (
    <Suspense fallback={null}>
      <TermWindow id={termId} />
    </Suspense>
  ) : (
    <App />
  ),
)
