import { render, screen, within } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import JobsView from './JobsView.jsx'

// "Now" is fixed so the relative "last run" text is stable; only Date is faked.
const NOW = new Date('2026-10-31T12:00:00Z')

const jobs = {
  jobs: [
    { name: 'easy-personal-ingest.timer', kind: 'systemd', command: 'easy-personal', schedule: 'daily', next_run: '2026-10-31T13:00:00Z', last_run: '2026-10-31T11:48:00Z', last_exit: 0 },
    { name: 'tidy.timer', kind: 'systemd', command: 'claude', schedule: 'weekly', next_run: null, last_run: '2026-10-30T12:00:00Z', last_exit: 1 },
    { name: 'fresh.timer', kind: 'systemd', command: 'claude', schedule: 'hourly', next_run: '2026-10-31T13:00:00Z', last_run: null, last_exit: null },
    { name: '(crontab) claude', kind: 'cron', command: 'claude', schedule: '0 7 * * 1-5', next_run: '2026-11-02T00:00:00Z', last_run: null, last_exit: null },
  ],
  hidden: 1,
  sources: { systemd: 'ok', cron: 'ok' },
}

function stub(body, status = 200) {
  const fn = vi.fn(async () => ({ ok: status === 200, status, json: async () => body }))
  vi.stubGlobal('fetch', fn)
  return fn
}
const row = (name) => screen.getByRole('rowheader', { name }).closest('tr')
const cells = (name) => within(row(name)).getAllByRole('cell').map((c) => c.textContent)

beforeEach(() => {
  window.location.hash = '#secret-token'
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(NOW)
})
afterEach(() => {
  vi.useRealTimers()
  vi.unstubAllGlobals()
})

describe('Scheduled view (HUN-163)', () => {
  it('asks for /api/jobs with the token and shows the six columns', async () => {
    const fn = stub(jobs)
    render(<JobsView />)
    await screen.findByRole('table')
    expect(fn.mock.calls[0][0]).toBe('/api/jobs')
    expect(fn.mock.calls[0][1].headers['X-Dashboard-Token']).toBe('secret-token')
    const heads = within(screen.getByRole('table')).getAllByRole('columnheader').map((h) => h.textContent)
    expect(heads).toEqual(['Job', 'Kind', 'Schedule', 'Next run', 'Last run', 'Last exit'])
  })

  it('shows a row per job with kind, schedule, next run, last run and exit code', async () => {
    stub(jobs)
    render(<JobsView />)
    await screen.findByRole('table')
    const own = cells('easy-personal-ingest.timer')
    expect(own.slice(0, 2)).toEqual(['systemd', 'daily'])
    expect(own[2]).toBe(new Date('2026-10-31T13:00:00Z').toLocaleString())
    expect(own[3]).toBe('12 min ago')
    expect(own[4]).toBe('✓ 0')
    expect(cells('(crontab) claude').slice(0, 2)).toEqual(['cron', '0 7 * * 1-5'])
  })

  it('marks a failed run, a timer that never ran, and shows cron last run as not recorded', async () => {
    stub(jobs)
    render(<JobsView />)
    await screen.findByRole('table')
    expect(cells('tidy.timer')[3]).toBe('1 d ago')
    expect(cells('tidy.timer')[4]).toBe('✗ 1')
    expect(cells('tidy.timer')[2]).toBe('—')
    expect(cells('fresh.timer').slice(3)).toEqual(['never', '—'])
    expect(cells('(crontab) claude').slice(3)).toEqual(['not recorded', '—'])
    expect(within(row('tidy.timer')).getByText('✗ 1')).toHaveClass('error')
  })

  it('has the footer with the hidden count and the not-included note', async () => {
    stub(jobs)
    render(<JobsView />)
    await screen.findByRole('table')
    expect(screen.getByText('1 other job hidden · /loop jobs and cloud routines are not included')).toBeInTheDocument()
  })

  it('pluralizes the hidden count and says "no other jobs" when none is hidden', async () => {
    stub({ ...jobs, hidden: 3 })
    const { unmount } = render(<JobsView />)
    expect(await screen.findByText(/^3 other jobs hidden/)).toBeInTheDocument()
    unmount()
    stub({ ...jobs, hidden: 0 })
    render(<JobsView />)
    expect(await screen.findByText(/^No other jobs hidden · /)).toBeInTheDocument()
  })

  it('says "no data" for a kind that could not be read and still renders the other', async () => {
    stub({ jobs: [jobs.jobs[3]], hidden: 0, sources: { systemd: 'no data', cron: 'ok' } })
    render(<JobsView />)
    await screen.findByRole('table')
    expect(screen.getByText('systemd timers: no data (no systemctl or no user session)')).toBeInTheDocument()
    expect(screen.queryByText(/^crontab: no data/)).not.toBeInTheDocument()
    expect(row('(crontab) claude')).toBeInTheDocument()
  })

  it('shows an empty message instead of an empty table, keeping the footer', async () => {
    stub({ jobs: [], hidden: 2, sources: { systemd: 'ok', cron: 'no data' } })
    render(<JobsView />)
    expect(await screen.findByText('No agent jobs found.')).toBeInTheDocument()
    expect(screen.queryByRole('table')).not.toBeInTheDocument()
    expect(screen.getByText(/^2 other jobs hidden/)).toBeInTheDocument()
    expect(screen.getByText('crontab: no data (no crontab for this user)')).toBeInTheDocument()
  })

  it('shows an alert when the request fails', async () => {
    stub({}, 500)
    render(<JobsView />)
    expect(await screen.findByRole('alert')).toHaveTextContent('Could not load scheduled jobs')
  })
})
