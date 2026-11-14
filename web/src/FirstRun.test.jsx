import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import FirstRun from './FirstRun.jsx'
import { firstRun } from './test/fixtures.js'

const done = (...ids) => firstRun.steps.map((s) => ({ ...s, done: ids.includes(s.id) }))
const ALL = ['first_ingest', 'ingest_timer', 'accounts_confirmed', 'first_workspace']
const item = (name) => screen.getByRole('listitem', { name })
// The N of M count is two spans (N is in accent mono): match the outer span by its whole text.
const count = (text) => (_, el) => el.tagName === 'SPAN' && el.textContent === text
const stateOf = (name) => item(name).getAttribute('data-state')

let writeText
beforeEach(() => {
  writeText = vi.fn().mockResolvedValue(undefined)
  Object.defineProperty(navigator, 'clipboard', { value: { writeText }, configurable: true })
})
afterEach(() => {
  vi.useRealTimers()
})

it('lists the four steps in order, numbered 01 to 04', () => {
  render(<FirstRun steps={done('first_ingest')} />)
  const items = within(screen.getByRole('list')).getAllByRole('listitem')
  expect(items.map((li) => li.getAttribute('aria-label'))).toEqual([
    'First ingest',
    'Ingest timer installed',
    'Agent accounts confirmed',
    'First workspace',
  ])
  expect(items.map((li) => li.querySelector('.first-run-no').textContent)).toEqual(['01', '02', '03', '04'])
})

it('shows the intro line and, with 2 of 4 done, the count, the left count and the progress bar', () => {
  render(<FirstRun steps={done('first_ingest', 'accounts_confirmed')} />)
  expect(screen.getByText('Four steps to fill the dashboard with your own sessions. Everything stays on this machine.')).toBeInTheDocument()
  expect(screen.getByText(count('2 of 4 steps done'))).toBeInTheDocument()
  expect(screen.getByText('2 left')).toBeInTheDocument()
  const bar = screen.getByRole('progressbar', { name: 'Setup progress' })
  expect(bar).toHaveAttribute('aria-valuenow', '2')
  expect(bar).toHaveAttribute('aria-valuemin', '0')
  expect(bar).toHaveAttribute('aria-valuemax', '4')
})

it('marks the first not-done step "next step", done steps "done" and later steps "not done"', () => {
  render(<FirstRun steps={done('first_ingest', 'accounts_confirmed')} />)
  expect(stateOf('First ingest')).toBe('done')
  expect(stateOf('Ingest timer installed')).toBe('next')
  expect(stateOf('Agent accounts confirmed')).toBe('done')
  expect(stateOf('First workspace')).toBe('later')
  expect(within(item('First ingest')).getByText('done')).toBeInTheDocument()
  expect(within(item('Ingest timer installed')).getByText('next step')).toBeInTheDocument()
  expect(within(item('First workspace')).getByText('not done')).toBeInTheDocument()
})

it('says "next step" only once, on the first not-done step', () => {
  render(<FirstRun steps={done()} />)
  expect(screen.getAllByText('next step')).toHaveLength(1)
  expect(within(item('First ingest')).getByText('next step')).toBeInTheDocument()
  expect(screen.getAllByText('not done')).toHaveLength(3)
})

it('shows the server command with a "$ " prefix for the ingest and timer steps when not done', () => {
  render(<FirstRun steps={done()} />)
  expect(within(item('First ingest')).getByText('easy-personal ingest')).toBeInTheDocument()
  expect(within(item('First ingest')).getByText('$')).toBeInTheDocument()
  expect(within(item('Ingest timer installed')).getByText('easy-personal schedule install')).toBeInTheDocument()
})

it('shows the server hint, so a cron line replaces the install command when systemd is missing', () => {
  const steps = firstRun.steps.map((s) => (s.id === 'ingest_timer' ? { ...s, hint: 'no systemd user session: run crontab -e and add: 17 3 * * * easy-personal ingest' } : s))
  render(<FirstRun steps={steps} />)
  expect(within(item('Ingest timer installed')).getByText(/crontab -e and add/)).toBeInTheDocument()
})

it('hides the command, Copy and Check again of a done step', () => {
  render(<FirstRun steps={done('first_ingest')} />)
  const card = item('First ingest')
  expect(within(card).queryByText('easy-personal ingest')).not.toBeInTheDocument()
  expect(within(card).queryByRole('button')).not.toBeInTheDocument()
})

it('shows the command text as text, never as HTML', () => {
  const steps = firstRun.steps.map((s) => (s.id === 'first_ingest' ? { ...s, hint: '<img src=x onerror=alert(1)> ingest' } : s))
  const { container } = render(<FirstRun steps={steps} />)
  expect(container.querySelector('img')).toBeNull()
  expect(within(item('First ingest')).getByText(/<img src=x/)).toBeInTheDocument()
})

it('Copy writes the server command to the clipboard and the label turns to Copied', async () => {
  render(<FirstRun steps={done('first_ingest')} />)
  const card = item('Ingest timer installed')
  fireEvent.click(within(card).getByRole('button', { name: 'Copy' }))
  await waitFor(() => expect(within(card).getByRole('button', { name: 'Copied' })).toBeInTheDocument())
  expect(writeText).toHaveBeenCalledWith('easy-personal schedule install')
})

it('the Copied label goes back to Copy after about 2 seconds', async () => {
  vi.useFakeTimers()
  render(<FirstRun steps={done('first_ingest')} />)
  const card = item('Ingest timer installed')
  await act(async () => {
    fireEvent.click(within(card).getByRole('button', { name: 'Copy' }))
  })
  expect(within(card).getByRole('button', { name: 'Copied' })).toBeInTheDocument()
  act(() => {
    vi.advanceTimersByTime(1900)
  })
  expect(within(card).getByRole('button', { name: 'Copied' })).toBeInTheDocument()
  act(() => {
    vi.advanceTimersByTime(200)
  })
  expect(within(card).getByRole('button', { name: 'Copy' })).toBeInTheDocument()
})

it('pressing Copy twice fast keeps one timer: the second press restarts the 2 seconds', async () => {
  vi.useFakeTimers()
  render(<FirstRun steps={done('first_ingest')} />)
  const card = item('Ingest timer installed')
  await act(async () => {
    fireEvent.click(within(card).getByRole('button', { name: 'Copy' }))
  })
  act(() => {
    vi.advanceTimersByTime(1500)
  })
  await act(async () => {
    fireEvent.click(within(card).getByRole('button', { name: 'Copied' }))
  })
  expect(vi.getTimerCount()).toBe(1)
  act(() => {
    vi.advanceTimersByTime(1500) // 3 s after the first press, 1.5 s after the second
  })
  expect(within(card).getByRole('button', { name: 'Copied' })).toBeInTheDocument()
  act(() => {
    vi.advanceTimersByTime(600)
  })
  expect(within(card).getByRole('button', { name: 'Copy' })).toBeInTheDocument()
  expect(vi.getTimerCount()).toBe(0)
})

it('a clipboard failure shows a short error and no crash', async () => {
  writeText.mockRejectedValue(new Error('denied'))
  render(<FirstRun steps={done('first_ingest')} />)
  const card = item('Ingest timer installed')
  fireEvent.click(within(card).getByRole('button', { name: 'Copy' }))
  expect(await within(card).findByRole('alert')).toHaveTextContent('Copy failed — select and copy')
  expect(within(card).getByRole('button', { name: 'Copy' })).toBeInTheDocument()
})

it('a missing clipboard (no secure context) shows the same short error', async () => {
  Object.defineProperty(navigator, 'clipboard', { value: undefined, configurable: true })
  render(<FirstRun steps={done('first_ingest')} />)
  fireEvent.click(within(item('Ingest timer installed')).getByRole('button', { name: 'Copy' }))
  expect(await screen.findByRole('alert')).toHaveTextContent('Copy failed — select and copy')
})

it('Check again asks for the checklist again, through the callback', async () => {
  const onCheckAgain = vi.fn().mockResolvedValue(undefined)
  render(<FirstRun steps={done('first_ingest')} onCheckAgain={onCheckAgain} />)
  const card = item('Ingest timer installed')
  expect(within(card).getByText('Ran it in your terminal?')).toBeInTheDocument()
  fireEvent.click(within(card).getByRole('button', { name: 'Check again' }))
  await waitFor(() => expect(onCheckAgain).toHaveBeenCalledTimes(1))
})

it('Check again keeps the old state and shows an error line when the request fails', async () => {
  const onCheckAgain = vi.fn().mockRejectedValue(new Error('500'))
  render(<FirstRun steps={done('first_ingest')} onCheckAgain={onCheckAgain} />)
  fireEvent.click(within(item('Ingest timer installed')).getByRole('button', { name: 'Check again' }))
  expect(await screen.findByRole('alert')).toHaveTextContent('Could not check again')
  expect(stateOf('Ingest timer installed')).toBe('next')
  expect(screen.getByText(count('1 of 4 steps done'))).toBeInTheDocument()
})

it('links the account and workspace steps to their views with buttons', () => {
  const onOpenAgents = vi.fn()
  const onOpenWorkspaces = vi.fn()
  render(<FirstRun steps={done()} onOpenAgents={onOpenAgents} onOpenWorkspaces={onOpenWorkspaces} />)
  expect(within(item('Agent accounts confirmed')).getByText('Only confirmed accounts are read.')).toBeInTheDocument()
  fireEvent.click(within(item('Agent accounts confirmed')).getByRole('button', { name: 'Open Agents' }))
  fireEvent.click(within(item('First workspace')).getByRole('button', { name: 'Open Workspaces' }))
  expect(onOpenAgents).toHaveBeenCalledTimes(1)
  expect(onOpenWorkspaces).toHaveBeenCalledTimes(1)
})

it('with all four done: a completion banner, Go to Usage, no step buttons and a full bar', () => {
  const onOpenUsage = vi.fn()
  render(<FirstRun steps={done(...ALL)} onOpenUsage={onOpenUsage} />)
  expect(screen.getByText('Setup is complete. Your usage appears after the next ingest.')).toBeInTheDocument()
  expect(screen.getByText(count('4 of 4 steps done'))).toBeInTheDocument()
  expect(screen.getByRole('progressbar')).toHaveAttribute('aria-valuenow', '4')
  expect(screen.getAllByRole('button')).toHaveLength(1)
  fireEvent.click(screen.getByRole('button', { name: 'Go to Usage' }))
  expect(onOpenUsage).toHaveBeenCalledTimes(1)
})

it('shows no completion banner while a step is open', () => {
  render(<FirstRun steps={done('first_ingest')} />)
  expect(screen.queryByText(/Setup is complete/)).not.toBeInTheDocument()
  expect(screen.queryByRole('button', { name: 'Go to Usage' })).not.toBeInTheDocument()
})

it('keeps the real step count: the intro and the count follow the server list', () => {
  const steps = [...done('first_ingest'), { id: 'fifth_thing', done: false, hint: 'x' }]
  render(<FirstRun steps={steps} />)
  expect(screen.getByText(/^5 steps to fill the dashboard/)).toBeInTheDocument()
  expect(screen.getByText(count('1 of 5 steps done'))).toBeInTheDocument()
  expect(screen.getByRole('progressbar')).toHaveAttribute('aria-valuemax', '5')
})

it('draws a generic card, with its title and no action, for a step id the page does not know', () => {
  const steps = [...done(...ALL), { id: 'sign_in', done: false, hint: 'x' }]
  render(<FirstRun steps={steps} />)
  const card = item('Sign in')
  expect(stateOf('Sign in')).toBe('next')
  expect(within(card).queryByRole('button')).not.toBeInTheDocument()
  expect(screen.getByText(count('4 of 5 steps done'))).toBeInTheDocument()
})

it('shows "no data" when the checklist could not be read', () => {
  render(<FirstRun steps={null} />)
  expect(screen.getByRole('alert')).toHaveTextContent('Could not load the checklist')
})

it('shows Loading before the first answer', () => {
  render(<FirstRun steps={undefined} />)
  expect(screen.getByRole('status')).toHaveTextContent('Loading')
})
