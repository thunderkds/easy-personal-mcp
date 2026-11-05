import { render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import AgentsView from './AgentsView.jsx'
import BoardView from './BoardView.jsx'
import FirstRun from './FirstRun.jsx'
import JobsView from './JobsView.jsx'
import KitView from './KitView.jsx'
import Loading from './Loading.jsx'
import { LOADING_LINES } from './loadingLines.js'
import MemoryView from './MemoryView.jsx'
import UsageView from './UsageView.jsx'
import WarningsPlanView from './WarningsPlanView.jsx'
import WorkspacesView from './WorkspacesView.jsx'
import App from './App.jsx'
import { ToastProvider } from './Toast.jsx'

afterEach(() => {
  vi.unstubAllGlobals()
  vi.restoreAllMocks()
})

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

  it('keeps the label first in the pill text and adds exactly one funny line from the list (HUN-229 AC4)', () => {
    const { container } = render(<Loading label="Loading usage" />)
    const pill = screen.getByRole('status')
    const lines = container.querySelectorAll('.loading-line')
    expect(lines).toHaveLength(1)
    expect(LOADING_LINES).toContain(lines[0].textContent)
    expect(pill.textContent.startsWith('Loading usage')).toBe(true)
    expect(pill.textContent).toBe(`Loading usage${lines[0].textContent}`)
  })

  it('picks the line with the random source: 0 is the first line, just under 1 is the last (HUN-229 AC4)', () => {
    const random = vi.spyOn(Math, 'random')
    random.mockReturnValue(0)
    const first = render(<Loading label="A" />)
    expect(first.container.querySelector('.loading-line')).toHaveTextContent(LOADING_LINES[0])
    first.unmount()
    random.mockReturnValue(0.999999)
    const last = render(<Loading label="A" />)
    expect(last.container.querySelector('.loading-line')).toHaveTextContent(LOADING_LINES.at(-1))
  })

  it('keeps the same line while the indicator stays on screen, even when it re-renders (HUN-229 AC4)', () => {
    const random = vi.spyOn(Math, 'random')
    random.mockReturnValue(0)
    const view = render(<Loading label="A" />)
    random.mockReturnValue(0.5)
    view.rerender(<Loading label="B" />)
    view.rerender(<Loading label="B" />)
    expect(view.container.querySelector('.loading-line')).toHaveTextContent(LOADING_LINES[0])
    expect(random).toHaveBeenCalledTimes(1)
  })

  it('has about a dozen short, different lines (HUN-229 AC4)', () => {
    expect(LOADING_LINES.length).toBeGreaterThanOrEqual(10)
    expect(LOADING_LINES.length).toBeLessThanOrEqual(14)
    expect(new Set(LOADING_LINES).size).toBe(LOADING_LINES.length)
    for (const line of LOADING_LINES) {
      expect(line.length).toBeGreaterThan(0)
      expect(line.length).toBeLessThanOrEqual(40)
      expect(line).toBe(line.trim())
    }
  })

  it('styles the line from tokens only: no colour literal in the .loading-line rule (HUN-229)', async () => {
    const { readFileSync } = await import('node:fs')
    const { resolve } = await import('node:path')
    const css = readFileSync(resolve(process.cwd(), 'src/app.css'), 'utf8')
    const rule = css.match(/\n\.loading-line\s*\{[^}]*\}/)
    expect(rule).not.toBeNull()
    expect(rule[0]).not.toMatch(/#[0-9a-f]{3,8}\b|rgba?\(|hsla?\(/i)
    expect(rule[0]).toMatch(/var\(--color-/)
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
    // Warnings & plan also holds the Saved reports list, which shows its own pill (HUN-229), so look for this view's label among the pills.
    const statuses = screen.getAllByRole('status')
    for (const s of statuses) expect(s).toHaveClass('loading-pill')
    const status = statuses.find((s) => s.textContent.startsWith(label))
    expect(status).toBeDefined()
    expect(status).toHaveTextContent(label)
    expect(status.querySelectorAll('.loading-line')).toHaveLength(1)
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
