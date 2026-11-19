import { fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import LiveView from './LiveView.jsx'
import { FakeEventSource, liveRow, liveServer, snapshot, stream } from './test/liveStream.js'
import { appServer, appSession } from './test/appSessions.js'

// The filters above the Live list (T119, FR-128): needs me / working / idle, project, kind; kept in localStorage (filters only); counts per filter.
const KEY = 'easy-personal-live-filters'
const waiting = { state: 'waiting', tool: 'AskUserQuestion', count: null, is_sidechain: false, since: 't', questions: ['Which?'] }
const OLD = new Date(Date.now() - 3_600_000).toISOString()

/** Five terminal sessions: one needs you, two work (one headless, in beta), two are idle (one by its process file, one by silence). */
const rows = () => [
  liveRow('aaaaaaaa-1', { project: 'alpha', attention: [waiting] }),
  liveRow('bbbbbbbb-2', { project: 'alpha', proc: { busy: true, name: null } }),
  liveRow('cccccccc-3', { project: 'beta', proc: { busy: true, name: null }, entrypoint: 'sdk-cli' }),
  liveRow('dddddddd-4', { project: 'beta', proc: { busy: false, name: null } }),
  liveRow('eeeeeeee-5', { project: 'alpha', last_ts: OLD }),
]
const rowEl = (id) => document.getElementById(`live-row-${id}`)
const listed = () => [...document.querySelectorAll('.live-row .live-id')].map((e) => e.textContent).sort()
const bar = () => screen.getByRole('group', { name: 'Filter sessions' })
const stateBtn = (name) => within(bar()).getByRole('button', { name })

beforeEach(() => {
  window.location.hash = '#secret-token'
  window.localStorage.clear()
  FakeEventSource.instances = []
  vi.stubGlobal('EventSource', FakeEventSource)
  vi.stubGlobal('fetch', liveServer())
})
afterEach(() => {
  vi.unstubAllGlobals()
  vi.restoreAllMocks()
})
const mount = (sessions = rows(), props = {}) => {
  const view = render(<LiveView {...props} />)
  stream().emit('snapshot', snapshot(sessions))
  return view
}

describe('counts and state filters (AC 1)', () => {
  it('shows a count on every state filter; the three states split the list', () => {
    mount()
    expect(stateBtn('All (5)')).toHaveAttribute('aria-pressed', 'true')
    expect(stateBtn('Needs me (1)')).toHaveAttribute('aria-pressed', 'false')
    expect(stateBtn('Working (2)')).toBeInTheDocument()
    expect(stateBtn('Idle (2)')).toBeInTheDocument()
  })

  it('Needs me lists the sessions that ask for you, Working the busy ones, Idle the rest', () => {
    mount()
    fireEvent.click(stateBtn('Needs me (1)'))
    expect(listed()).toEqual(['aaaaaaaa'])
    expect(stateBtn('Needs me (1)')).toHaveAttribute('aria-pressed', 'true')
    fireEvent.click(stateBtn('Working (2)'))
    expect(listed()).toEqual(['bbbbbbbb', 'cccccccc'])
    fireEvent.click(stateBtn('Idle (2)'))
    expect(listed()).toEqual(['dddddddd', 'eeeeeeee'])
    fireEvent.click(stateBtn('All (5)'))
    expect(listed()).toHaveLength(5)
  })

  it('a session with no process file is working while it is active and idle after the window', () => {
    mount([liveRow('aaaaaaaa-1'), liveRow('bbbbbbbb-2', { last_ts: OLD })])
    expect(stateBtn('Working (1)')).toBeInTheDocument()
    expect(stateBtn('Idle (1)')).toBeInTheDocument()
  })

  it('counts follow the other filters: a project narrows the state counts, a state narrows the project and kind counts', () => {
    mount()
    fireEvent.change(screen.getByLabelText('Project'), { target: { value: 'beta' } })
    expect(listed()).toEqual(['cccccccc', 'dddddddd'])
    expect(stateBtn('All (2)')).toBeInTheDocument()
    expect(stateBtn('Working (1)')).toBeInTheDocument()
    expect(stateBtn('Needs me (0)')).toBeInTheDocument()
    fireEvent.click(stateBtn('Working (1)'))
    expect(within(screen.getByLabelText('Kind')).getByRole('option', { name: 'Headless (1)' })).toBeInTheDocument()
    expect(within(screen.getByLabelText('Kind')).getByRole('option', { name: 'Terminal (0)' })).toBeInTheDocument()
    expect(within(screen.getByLabelText('Project')).getByRole('option', { name: 'alpha (1)' })).toBeInTheDocument() // a project's count is what choosing it would list under the state and kind chosen
  })
})

describe('project and kind (AC 1)', () => {
  it('the project list holds every project once, in name order, with its count', () => {
    mount()
    const options = within(screen.getByLabelText('Project')).getAllByRole('option').map((o) => o.textContent)
    expect(options).toEqual(['All projects', 'alpha (3)', 'beta (2)'])
  })

  it('Kind: terminal, app and headless, with counts', () => {
    mount()
    const options = within(screen.getByLabelText('Kind')).getAllByRole('option').map((o) => o.textContent)
    expect(options).toEqual(['All kinds', 'Terminal (4)', 'App (0)', 'Headless (1)'])
    fireEvent.change(screen.getByLabelText('Kind'), { target: { value: 'headless' } })
    expect(listed()).toEqual(['cccccccc'])
  })

  it('a filter\'s own count ignores itself: with Headless chosen, Terminal still counts what choosing it would list', () => {
    mount()
    fireEvent.change(screen.getByLabelText('Kind'), { target: { value: 'headless' } })
    expect(within(screen.getByLabelText('Kind')).getByRole('option', { name: 'Terminal (4)' })).toBeInTheDocument()
    expect(within(screen.getByLabelText('Kind')).getByRole('option', { name: 'Headless (1)' })).toBeInTheDocument()
    expect(within(screen.getByLabelText('Project')).getByRole('option', { name: 'beta (1)' })).toBeInTheDocument()
    expect(stateBtn('Working (1)')).toBeInTheDocument()
  })

  it('a session of the app (listed from the app session list) is kind app', async () => {
    vi.stubGlobal('fetch', appServer({ sessions: [appSession({ id: 'a1', session_id: 'bbbbbbbb-2' })] }))
    mount(rows(), { appSessions: true })
    await waitFor(() => expect(within(screen.getByLabelText('Kind')).getByRole('option', { name: 'App (1)' })).toBeInTheDocument())
    fireEvent.change(screen.getByLabelText('Kind'), { target: { value: 'app' } })
    expect(listed()).toEqual(['bbbbbbbb'])
  })

  it('filters combine, and a filter that matches nothing says so and offers Clear filters', () => {
    mount()
    fireEvent.change(screen.getByLabelText('Project'), { target: { value: 'beta' } })
    fireEvent.click(stateBtn('Needs me (0)'))
    expect(listed()).toEqual([])
    expect(screen.getByText('No sessions match these filters.')).toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: 'Clear filters' }))
    expect(listed()).toHaveLength(5)
    expect(screen.queryByRole('button', { name: 'Clear filters' })).toBeNull()
  })

  it('an open panel stays when its row no longer matches', async () => {
    mount()
    fireEvent.click(rowEl('aaaaaaaa-1'))
    await screen.findByRole('region', { name: /^Session in / })
    fireEvent.click(stateBtn('Idle (2)'))
    expect(screen.getByRole('region', { name: 'Session in alpha' })).toBeInTheDocument()
    expect(listed()).toContain('aaaaaaaa')
  })
})

describe('kept in localStorage (AC 1)', () => {
  it('stores the filters and nothing else, and the next page load starts with them', () => {
    const first = mount()
    fireEvent.click(stateBtn('Working (2)'))
    fireEvent.change(screen.getByLabelText('Project'), { target: { value: 'beta' } })
    fireEvent.change(screen.getByLabelText('Kind'), { target: { value: 'headless' } })
    expect(JSON.parse(window.localStorage.getItem(KEY))).toEqual({ state: 'working', project: 'beta', kind: 'headless' })
    expect(window.localStorage.length).toBe(1)
    first.unmount()
    mount()
    expect(listed()).toEqual(['cccccccc'])
    expect(screen.getByLabelText('Project')).toHaveValue('beta')
    expect(stateBtn('Working (1)')).toHaveAttribute('aria-pressed', 'true')
  })

  it('a stored project that no session has now is still shown, with a count of 0, so the filter can be seen and cleared', () => {
    window.localStorage.setItem(KEY, JSON.stringify({ state: 'all', project: 'gone', kind: '' }))
    mount()
    expect(screen.getByLabelText('Project')).toHaveValue('gone')
    expect(within(screen.getByLabelText('Project')).getByRole('option', { name: 'gone (0)' })).toBeInTheDocument()
    expect(screen.getByText('No sessions match these filters.')).toBeInTheDocument()
  })

  it('a stored value of the wrong shape is ignored, field by field', () => {
    window.localStorage.setItem(KEY, JSON.stringify({ state: 'sideways', project: 7, kind: 'headless', extra: 'x' }))
    mount()
    expect(stateBtn('All (1)')).toBeInTheDocument()
    expect(screen.getByLabelText('Kind')).toHaveValue('headless')
    expect(screen.getByLabelText('Project')).toHaveValue('')
  })

  it.each(['not json', '"a string"', 'null', '[1]'])('stored %s does not break the page', (raw) => {
    window.localStorage.setItem(KEY, raw)
    mount()
    expect(listed()).toHaveLength(5)
  })

  it('works when localStorage throws on read and on write', () => {
    vi.spyOn(Storage.prototype, 'getItem').mockImplementation(() => {
      throw new Error('denied')
    })
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
      throw new Error('denied')
    })
    mount()
    fireEvent.click(stateBtn('Working (2)'))
    expect(listed()).toEqual(['bbbbbbbb', 'cccccccc'])
  })

  it('Clear filters removes the stored choice', () => {
    mount()
    fireEvent.click(stateBtn('Idle (2)'))
    fireEvent.click(screen.getByRole('button', { name: 'Clear filters' }))
    expect(JSON.parse(window.localStorage.getItem(KEY))).toEqual({ state: 'all', project: '', kind: '' })
  })
})

describe('one rule for who needs you (Stage 4 review)', () => {
  const permission = { id: 'p2', kind: 'permission', tool: 'Bash', input: 'git status', complete: true }
  it('a session with a prompt waiting in its panel counts in the heading and in the Needs me filter alike', () => {
    mount([liveRow('aaaaaaaa-1', { pending_answer: permission }), liveRow('bbbbbbbb-2', { proc: { busy: true, name: null } })])
    expect(stateBtn('Needs me (1)')).toBeInTheDocument()
    expect(screen.getByText('1 session needs you')).toBeInTheDocument()
  })
})
