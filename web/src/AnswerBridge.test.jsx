import { fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import AnswerBridge, { NOT_LISTENING } from './AnswerBridge.jsx'
import { accountList } from './test/fixtures.js'

// The Settings block *Answer terminal sessions from the app* (T107, FR-113). Fixtures only: no file is written.
const ENTRIES = '{\n  "hooks": {\n    "PreToolUse": [{"matcher": "AskUserQuestion"}],\n    "PermissionRequest": [{}]\n  }\n}'
const accounts = accountList.filter((a) => a.provider === 'claude' && a.status === 'confirmed') // ids 1 and 2
const state = (status1, status2 = 'off') => ({
  command: '/home/o/.local/bin/easy-personal-rs hook ask',
  entries_text: ENTRIES,
  accounts: [{ id: 1, label: 'Claude default', status: status1 }, { id: 2, label: 'Claude personal', status: status2 }],
})
const click = fireEvent.click

/** GET /api/hooks answers `get` (a body, or a status number); PUT answers `put(body)` (a row, or a status number). Records every request. */
function stub({ get = state('off'), put } = {}) {
  const calls = []
  vi.stubGlobal(
    'fetch',
    vi.fn(async (url, init = {}) => {
      const body = init.body ? JSON.parse(init.body) : undefined
      calls.push({ url: String(url), method: init.method ?? 'GET', body, headers: init.headers })
      const answer = init.method === 'PUT' ? put?.(body) ?? { id: Number(body.account), label: body.account === '1' ? 'Claude default' : 'Claude personal', status: body.on ? 'on' : 'off' } : get
      if (typeof answer === 'number') return { ok: false, status: answer, json: async () => ({ error: { 409: 'an entry of ours was changed by hand; fix or remove it yourself, then try again', 503: 'no data folder' }[answer] }) }
      return { ok: true, status: 200, json: async () => answer }
    }),
  )
  return calls
}
const open = async (setup) => {
  const calls = stub(setup)
  render(<AnswerBridge accounts={accounts} />)
  await screen.findByRole('heading', { name: 'Answer terminal sessions from the app' })
  await screen.findAllByRole('listitem')
  return calls
}
const row = (label) => screen.getByText(label).closest('li')

beforeEach(() => {
  window.location.hash = '#secret-token'
})
afterEach(() => {
  vi.restoreAllMocks()
  vi.unstubAllGlobals()
})

describe('AC1: the block', () => {
  it('says it is off by default and that earlier sessions are not covered', async () => {
    await open()
    expect(screen.getByText(/Off by default/)).toBeInTheDocument()
    expect(screen.getByText('Sessions that were started before you turn this on are not covered. Start a new session.')).toBeInTheDocument()
  })

  it('lists each confirmed Claude account with its state: on, off, changed by hand, binary moved', async () => {
    await open({ get: { ...state('on'), accounts: [{ id: 1, label: 'Claude default', status: 'on' }, { id: 2, label: 'Claude personal', status: 'off' }, { id: 3, label: 'Third', status: 'changed_by_hand' }, { id: 4, label: 'Fourth', status: 'binary_moved' }] } })
    expect(within(row('Claude default')).getByText('On')).toBeInTheDocument()
    expect(within(row('Claude personal')).getByText('Off')).toBeInTheDocument()
    expect(within(row('Third')).getByText('Changed by hand')).toBeInTheDocument()
    expect(within(row('Fourth')).getByText('Binary moved')).toBeInTheDocument()
    expect(within(row('Fourth')).getByText(/has moved/)).toBeInTheDocument()
    expect(within(row('Third')).getByText(/changed by hand/)).toBeInTheDocument()
  })

  it('offers Turn on for off, Turn off for on, and both for a changed or moved account', async () => {
    await open({ get: { ...state('on'), accounts: [{ id: 1, label: 'A', status: 'on' }, { id: 2, label: 'B', status: 'off' }, { id: 3, label: 'C', status: 'changed_by_hand' }, { id: 4, label: 'D', status: 'binary_moved' }, { id: 5, label: 'E', status: 'unreadable' }] } })
    const buttons = (label) => within(row(label)).queryAllByRole('button').map((b) => b.textContent)
    expect(buttons('A')).toEqual(['Turn off'])
    expect(buttons('B')).toEqual(['Turn on'])
    expect(buttons('C')).toEqual(['Turn on', 'Turn off'])
    expect(buttons('D')).toEqual(['Turn on', 'Turn off'])
    expect(buttons('E')).toEqual([])
  })

  it('Turn on shows the exact entry text and the file, and writes nothing until it is confirmed', async () => {
    const calls = await open()
    click(within(row('Claude default')).getByRole('button', { name: 'Turn on' }))
    const confirm = screen.getByRole('group', { name: 'Turn on for Claude default?' })
    expect(confirm.querySelector('pre').textContent).toBe(ENTRIES) // the server's text, as it came
    expect(confirm).toHaveTextContent('/home/o/.claude/settings.json')
    expect(confirm).toHaveTextContent('~/.easy-personal/backups/')
    expect(calls.filter((c) => c.method === 'PUT')).toHaveLength(0)
    expect(document.activeElement).toBe(within(confirm).getByRole('heading', { name: 'Turn on for Claude default?' }))
    click(within(confirm).getByRole('button', { name: 'Cancel' }))
    expect(screen.queryByRole('group', { name: /Turn on for/ })).toBeNull()
    expect(calls.filter((c) => c.method === 'PUT')).toHaveLength(0)
  })

  it('Add these entries sends the account, on and confirmed, then shows On', async () => {
    const calls = await open({ put: () => ({ id: 1, label: 'Claude default', status: 'on' }) })
    click(within(row('Claude default')).getByRole('button', { name: 'Turn on' }))
    click(screen.getByRole('button', { name: 'Add these entries' }))
    await waitFor(() => expect(within(row('Claude default')).getByText('On')).toBeInTheDocument())
    const puts = calls.filter((c) => c.method === 'PUT')
    expect(puts).toHaveLength(1)
    expect(puts[0].url).toBe('/api/hooks')
    expect(puts[0].body).toEqual({ account: '1', on: true, confirmed: true })
    expect(puts[0].headers['X-Dashboard-Token']).toBe('secret-token')
    expect(screen.queryByRole('group', { name: /Turn on for/ })).toBeNull()
  })

  it('Turn off sends on false and shows Off', async () => {
    const calls = await open({ get: state('on') })
    click(within(row('Claude default')).getByRole('button', { name: 'Turn off' }))
    await waitFor(() => expect(within(row('Claude default')).getByText('Off')).toBeInTheDocument())
    expect(calls.find((c) => c.method === 'PUT').body).toEqual({ account: '1', on: false, confirmed: true })
  })

  it('a refused change (409) shows the server\'s words in the confirm step and keeps the state', async () => {
    await open({ get: state('changed_by_hand'), put: () => 409 })
    click(within(row('Claude default')).getByRole('button', { name: 'Turn on' }))
    click(screen.getByRole('button', { name: 'Add these entries' }))
    expect(await screen.findByRole('alert')).toHaveTextContent('an entry of ours was changed by hand; fix or remove it yourself, then try again')
    expect(within(row('Claude default')).getByText('Changed by hand')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Add these entries' })).toBeEnabled()
  })

  it('a failed Turn off says so and keeps the buttons', async () => {
    await open({ get: state('on'), put: () => 500 })
    click(within(row('Claude default')).getByRole('button', { name: 'Turn off' }))
    expect(await screen.findByRole('alert')).toHaveTextContent('Could not change the setting. Try again.')
    expect(within(row('Claude default')).getByText('On')).toBeInTheDocument()
  })

  it('says so when an account is on but the app is not listening, and not otherwise', async () => {
    await open({ get: { ...state('on'), listening: false } })
    expect(within(row('Claude default')).getByText(NOT_LISTENING)).toBeInTheDocument()
    expect(within(row('Claude personal')).queryByText(NOT_LISTENING)).toBeNull() // off: nothing to listen for
  })

  it('shows no such note when the app is listening', async () => {
    await open({ get: { ...state('on'), listening: true } })
    expect(screen.queryByText(NOT_LISTENING)).toBeNull()
  })

  it('shows no such note from an older server that sends no listening key', async () => {
    await open({ get: state('on') })
    expect(screen.queryByText(NOT_LISTENING)).toBeNull()
  })

  it('a turn on that answers listening: false shows the note at once', async () => {
    await open({ put: () => ({ id: 1, label: 'Claude default', status: 'on', listening: false }) })
    click(within(row('Claude default')).getByRole('button', { name: 'Turn on' }))
    click(screen.getByRole('button', { name: 'Add these entries' }))
    expect(await screen.findByText(NOT_LISTENING)).toBeInTheDocument()
  })

  it('draws nothing when the server has no such route, and names a failed load', async () => {
    stub({ get: 404 })
    const { container, unmount } = render(<AnswerBridge accounts={accounts} />)
    await waitFor(() => expect(container.querySelector('.card')).toBeNull())
    unmount()
    stub({ get: 500 })
    render(<AnswerBridge accounts={accounts} />)
    expect(await screen.findByRole('alert')).toHaveTextContent('Could not load the answer bridge')
  })

  it('says to confirm an account first when none is listed', async () => {
    stub({ get: { ...state('off'), accounts: [] } })
    render(<AnswerBridge accounts={[]} />)
    expect(await screen.findByText(/No confirmed Claude account yet/)).toBeInTheDocument()
  })

  it('keeps nothing in localStorage', async () => {
    const setItem = vi.spyOn(Storage.prototype, 'setItem')
    await open()
    click(within(row('Claude default')).getByRole('button', { name: 'Turn on' }))
    click(screen.getByRole('button', { name: 'Add these entries' }))
    await waitFor(() => expect(within(row('Claude default')).getByText('On')).toBeInTheDocument())
    expect(setItem).not.toHaveBeenCalled()
  })
})
