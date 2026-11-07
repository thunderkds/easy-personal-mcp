import { fireEvent, render, screen, within } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import WorkspaceKit, { workspaceKitPath } from './WorkspaceKit.jsx'
import { workspaceKit } from './test/fixtures.js'

const ready = (data = workspaceKit) => ({ status: 'ready', data })
const view = (state, props = {}) => render(<WorkspaceKit state={state} {...props} />)
const row = (name) => screen.getByRole('listitem', { name })
const withWorkspaces = (...workspaces) => ({ ...workspaceKit, workspaces })
const one = (overrides) => ({ ...structuredClone(workspaceKit.workspaces[0]), ...overrides })

describe('rows (AC1)', () => {
  it('renders one row per workspace with the verifier and easy-ui text', () => {
    view(ready())
    expect(screen.getAllByRole('listitem')).toHaveLength(2)
    const acme = row('acme')
    expect(acme).toHaveTextContent('2 of 3 repos scored')
    expect(acme).toHaveTextContent('mean 70')
    expect(acme).toHaveTextContent('lowest api 60')
    expect(acme).toHaveTextContent('newest report 2026-11-04')
    expect(acme).toHaveTextContent('2 sessions · 4 calls · last 2026-11-06')
  })

  it('draws the mean as a bar with a text equivalent', () => {
    view(ready())
    const bar = within(row('acme')).getByRole('progressbar', { name: 'Verifier mean 70 of 100' })
    expect(bar).toHaveAttribute('aria-valuenow', '70')
  })

  it('shows a mean with one decimal when it has one, and the singular for one session and one call', () => {
    const w = one({ verifier: { ...workspaceKit.workspaces[0].verifier, mean: 71.5 }, ui: { status: 'ok', sessions: 1, calls: 1, last_utc: '2026-11-06T12:00:00Z' } })
    view(ready(withWorkspaces(w)))
    expect(row('acme')).toHaveTextContent('mean 71.5')
    expect(row('acme')).toHaveTextContent('1 session · 1 call · last 2026-11-06')
  })

  it('is a labelled list with a heading that carries the workspace count', () => {
    view(ready())
    expect(screen.getByRole('heading', { level: 2, name: 'Kit by workspace (2)' })).toBeInTheDocument()
    expect(screen.getByRole('list', { name: 'Kit by workspace' })).toBeInTheDocument()
  })
})

describe('no data (AC2)', () => {
  it('shows "no data" text and no bar or number for a workspace without a score, and "not used" for easy-ui', () => {
    view(ready())
    const beta = row('beta')
    expect(beta).toHaveTextContent('no data')
    expect(beta).toHaveTextContent('not used')
    expect(within(beta).queryByRole('progressbar')).toBeNull()
    expect(beta).not.toHaveTextContent(/mean|lowest|newest|\b0\b/)
  })

  it('draws a real mean of 0 as a number with a bar, unlike "no data"', () => {
    const verifier = { status: 'ok', repos_scored: 1, repos_total: 1, mean: 0, lowest: { project: 'zero', value: 0 }, newest_utc: '2026-11-04T12:00:00Z' }
    view(ready(withWorkspaces(one({ verifier }))))
    expect(row('acme')).toHaveTextContent('mean 0')
    expect(row('acme')).toHaveTextContent('lowest zero 0')
    expect(within(row('acme')).getByRole('progressbar')).toHaveAttribute('aria-valuenow', '0')
    expect(row('acme')).not.toHaveTextContent('no data')
  })
})

describe('easy-ui checks line (AC3)', () => {
  it('shows the one overall line, once, under the rows', () => {
    view(ready())
    const line = screen.getByText('easy-ui checks (all workspaces): 3 sessions, 2 passed, 1 failed — not split by workspace')
    expect(screen.getAllByText(/not split by workspace/)).toHaveLength(1)
    expect(within(row('acme')).queryByText(/not split by workspace/)).toBeNull()
    expect(screen.getByRole('list', { name: 'Kit by workspace' }).compareDocumentPosition(line) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
  })

  it('shows no line when the checks have no data', () => {
    view(ready({ ...workspaceKit, ui_checks: { status: 'no data' } }))
    expect(screen.queryByText(/easy-ui checks/)).toBeNull()
  })

  it('shows no line when ui_checks is missing', () => {
    const { ui_checks: _gone, ...rest } = workspaceKit
    view(ready(rest))
    expect(screen.queryByText(/easy-ui checks/)).toBeNull()
  })
})

describe('loading and error (AC4)', () => {
  it('uses the Loading pill while the read is pending', () => {
    view({ status: 'loading' })
    expect(screen.getByRole('status')).toHaveTextContent('Loading kit by workspace')
    expect(screen.getByRole('heading', { name: 'Kit by workspace' })).toBeInTheDocument()
  })

  it('shows a failed read inline, in the block only, with role="alert"', () => {
    view({ status: 'error', message: '/api/kit/workspaces failed with status 500' })
    expect(screen.getByRole('alert')).toHaveTextContent('Could not load kit by workspace: /api/kit/workspaces failed with status 500')
    expect(screen.queryByRole('status')).toBeNull()
  })
})

describe('selected workspace (AC6)', () => {
  it('marks only the selected row with aria-current and still lists every row', () => {
    view(ready(), { selected: 'beta' })
    expect(row('beta')).toHaveAttribute('aria-current', 'true')
    expect(row('acme')).not.toHaveAttribute('aria-current')
    expect(screen.getAllByRole('listitem')).toHaveLength(2)
  })
  it('marks nothing without a selection', () => {
    view(ready())
    expect(document.querySelector('[aria-current]')).toBeNull()
  })
})

describe('empty and sparse data (AC7)', () => {
  it('says "No workspaces yet." and links to the Workspaces page', () => {
    const open = vi.fn()
    view(ready({ workspaces: [], ui_checks: { status: 'no data' } }), { onOpenWorkspaces: open })
    expect(screen.getByText(/No workspaces yet\./)).toBeInTheDocument()
    expect(screen.queryByRole('list', { name: 'Kit by workspace' })).toBeNull()
    fireEvent.click(screen.getByRole('button', { name: 'Open Workspaces' }))
    expect(open).toHaveBeenCalledTimes(1)
  })

  it('does not crash when optional fields are missing', () => {
    const sparse = { workspaces: [{ name: 'bare' }, { name: 'half', verifier: { status: 'ok', mean: 55 }, ui: { status: 'ok', calls: 2 } }] }
    view(ready(sparse))
    expect(row('bare')).toHaveTextContent('no data')
    expect(row('bare')).toHaveTextContent('not used')
    expect(row('half')).toHaveTextContent('mean 55')
    expect(row('half')).not.toHaveTextContent(/undefined|NaN|null/)
    expect(row('half')).toHaveTextContent('2 calls')
  })
})

describe('edge cases', () => {
  it('ellipsises a very long name and keeps the full name in the title', () => {
    const long = 'client-with-a-very-long-workspace-name-x'
    view(ready(withWorkspaces(one({ name: long }))))
    const name = screen.getByText(long)
    expect(name).toHaveAttribute('title', long)
    expect(name).toHaveClass('ws-name')
  })

  it('renders a planted markup name as text, never as HTML', () => {
    const planted = '<img src=x onerror="window.__pwned=1"><b>CANARY</b>'
    const { container } = view(ready(withWorkspaces(one({ name: planted, color: '#3366cc' }))))
    expect(container.querySelector('img')).toBeNull()
    expect(container.querySelector('b')).toBeNull()
    expect(screen.getByText(planted)).toBeInTheDocument()
    expect(window.__pwned).toBeUndefined()
  })

  it('uses the workspace colour for the dot only when it is #rrggbb', () => {
    const { container } = view(ready(withWorkspaces(one({ color: '#3366cc' }), one({ name: 'odd', color: 'red; background: url(x)' }))))
    const dots = container.querySelectorAll('.ws-dot')
    expect(dots[0].style.background).toBe('rgb(51, 102, 204)')
    expect(dots[1].getAttribute('style')).toBeNull()
  })
})

describe('request path (AC5)', () => {
  const NOW = Date.UTC(2026, 10, 7, 12, 30, 45)
  it('carries since for a period with days, and nothing for All time', () => {
    expect(workspaceKitPath('7', '', NOW)).toBe(`/api/kit/workspaces?since=${encodeURIComponent('2026-10-31T12:30:00.000Z')}`)
    expect(workspaceKitPath('30', '', NOW)).toBe(`/api/kit/workspaces?since=${encodeURIComponent('2026-10-08T12:30:00.000Z')}`)
    expect(workspaceKitPath('all', '', NOW)).toBe('/api/kit/workspaces')
  })
  it('carries the account, never the workspace', () => {
    expect(workspaceKitPath('all', '3', NOW)).toBe('/api/kit/workspaces?account=3')
    expect(workspaceKitPath('7', '3', NOW)).toBe(`/api/kit/workspaces?since=${encodeURIComponent('2026-10-31T12:30:00.000Z')}&account=3`)
  })
  it('falls back to the first period for an unknown id', () => {
    expect(workspaceKitPath('zzz', '', NOW)).toContain('since=')
  })
})
