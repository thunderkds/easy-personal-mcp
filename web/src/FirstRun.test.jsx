import { fireEvent, render, screen, within } from '@testing-library/react'
import { expect, it, vi } from 'vitest'
import FirstRun from './FirstRun.jsx'
import { firstRun } from './test/fixtures.js'

const done = (...ids) => firstRun.steps.map((s) => ({ ...s, done: ids.includes(s.id) }))
const item = (name) => screen.getByRole('listitem', { name })

it('lists the four steps in order, each with its done state', () => {
  render(<FirstRun steps={done('first_ingest')} />)
  const items = within(screen.getByRole('list')).getAllByRole('listitem')
  expect(items.map((li) => li.getAttribute('aria-label'))).toEqual([
    'First ingest',
    'Ingest timer installed',
    'Agent accounts confirmed',
    'First workspace',
  ])
  expect(within(item('First ingest')).getByText('Done')).toBeInTheDocument()
  expect(within(item('Ingest timer installed')).getByText('Not done')).toBeInTheDocument()
})

it('shows the exact CLI command for the ingest and timer steps when not done', () => {
  render(<FirstRun steps={done()} />)
  expect(within(item('First ingest')).getByText('easy-personal ingest')).toBeInTheDocument()
  expect(within(item('Ingest timer installed')).getByText('easy-personal schedule install')).toBeInTheDocument()
})

it('shows the server hint, so a cron line replaces the install command when systemd is missing', () => {
  const steps = firstRun.steps.map((s) => (s.id === 'ingest_timer' ? { ...s, hint: 'no systemd user session: run crontab -e and add: 17 3 * * * easy-personal ingest' } : s))
  render(<FirstRun steps={steps} />)
  expect(within(item('Ingest timer installed')).getByText(/crontab -e and add/)).toBeInTheDocument()
})

it('hides the command of a done step', () => {
  render(<FirstRun steps={done('first_ingest')} />)
  expect(within(item('First ingest')).queryByText('easy-personal ingest')).not.toBeInTheDocument()
})

it('links the account and workspace steps to their views with buttons', () => {
  const onOpenAgents = vi.fn()
  const onOpenWorkspaces = vi.fn()
  render(<FirstRun steps={done()} onOpenAgents={onOpenAgents} onOpenWorkspaces={onOpenWorkspaces} />)
  fireEvent.click(within(item('Agent accounts confirmed')).getByRole('button', { name: 'Open Agents' }))
  fireEvent.click(within(item('First workspace')).getByRole('button', { name: 'Open Workspaces' }))
  expect(onOpenAgents).toHaveBeenCalledTimes(1)
  expect(onOpenWorkspaces).toHaveBeenCalledTimes(1)
})

it('shows no action for done steps and a success line when all four are done', () => {
  render(<FirstRun steps={done('first_ingest', 'ingest_timer', 'accounts_confirmed', 'first_workspace')} />)
  expect(screen.queryByRole('button')).not.toBeInTheDocument()
  expect(screen.getByText('Setup is complete.')).toBeInTheDocument()
})

it('shows "no data" when the checklist could not be read', () => {
  render(<FirstRun steps={null} />)
  expect(screen.getByRole('alert')).toHaveTextContent('Could not load the checklist')
})

it('shows Loading before the first answer', () => {
  render(<FirstRun steps={undefined} />)
  expect(screen.getByRole('status')).toHaveTextContent('Loading')
})
