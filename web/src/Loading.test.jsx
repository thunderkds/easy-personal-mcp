import { render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import AgentsView from './AgentsView.jsx'
import BoardView from './BoardView.jsx'
import FirstRun from './FirstRun.jsx'
import JobsView from './JobsView.jsx'
import KitView from './KitView.jsx'
import Loading from './Loading.jsx'
import MemoryView from './MemoryView.jsx'
import UsageView from './UsageView.jsx'
import WarningsPlanView from './WarningsPlanView.jsx'
import WorkspacesView from './WorkspacesView.jsx'
import App from './App.jsx'
import { ToastProvider } from './Toast.jsx'

afterEach(() => vi.unstubAllGlobals())

// Never answers: every view stays in its loading state.
const hang = () => vi.stubGlobal('fetch', vi.fn(() => new Promise(() => {})))
// The workspace list answers, the account list never does (Agents loads them one after the other).
const hangAccounts = () =>
  vi.stubGlobal(
    'fetch',
    vi.fn((url) =>
      String(url).startsWith('/api/workspaces') ? Promise.resolve({ ok: true, status: 200, json: async () => ({ workspaces: [] }) }) : new Promise(() => {}),
    ),
  )

describe('Loading pill (AC7)', () => {
  it('is a role=status pill with a spinner and its label', () => {
    const { container } = render(<Loading label="Loading usage" />)
    const pill = screen.getByRole('status')
    expect(pill).toHaveTextContent('Loading usage')
    expect(pill).toHaveClass('loading-pill')
    expect(pill.tagName).not.toBe('P')
    expect(container.querySelector('.spinner')).toHaveAttribute('aria-hidden', 'true')
  })

  const views = [
    ['Loading usage', () => <UsageView />],
    ['Loading board', () => <BoardView />],
    ['Loading scheduled jobs', () => <JobsView />],
    ['Loading kit', () => <KitView />],
    ['Loading warnings', () => <WarningsPlanView />],
    ['Loading workspaces', () => <WorkspacesView />],
    ['Loading', () => <MemoryView />],
    ['Loading', () => <FirstRun steps={undefined} />],
  ]
  it.each(views)('%s: shows the floating pill and no inline loading paragraph', (label, view) => {
    hang()
    const { container } = render(<ToastProvider>{view()}</ToastProvider>)
    const status = screen.getByRole('status')
    expect(status).toHaveClass('loading-pill')
    expect(status).toHaveTextContent(label)
    expect(container.querySelector('p[role="status"]')).toBeNull()
  })

  it('Agents: shows the floating pill for the account list and no inline loading paragraph', async () => {
    hangAccounts()
    const { container } = render(
      <ToastProvider>
        <AgentsView />
      </ToastProvider>,
    )
    const status = await screen.findByRole('status')
    expect(status).toHaveClass('loading-pill')
    expect(status).toHaveTextContent('Loading agent accounts')
    expect(container.querySelector('p[role="status"]')).toBeNull()
  })

  it('shows the pill while App waits for the scope options (scoped view)', () => {
    hang()
    render(<App />)
    const pills = document.querySelectorAll('.loading-pill')
    expect(pills.length).toBeGreaterThan(0)
    expect(document.querySelector('p[role="status"]:not(.muted)')).toBeNull()
  })
})
