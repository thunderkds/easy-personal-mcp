import { act, fireEvent, render as rtlRender, screen, waitFor, within } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import SettingsView, { FirstOpenPrompt } from './SettingsView.jsx'
import { ToastProvider } from './Toast.jsx'
import { accountList, accountsStatus, agentRunError, agentRunOff, agentRunOn, fakeFetch } from './test/fixtures.js'

// The Settings view and the first-open prompt (T092, FR-104 parts 1 and 2). Fixtures only: nothing starts a run.
const render = (ui) => rtlRender(ui, { wrapper: ToastProvider })
const NOTE = "Agent runs use this account's Claude subscription through Claude Code on this machine. They count against its plan limits."
const MARK = 'easy-personal-agent-run-asked'
const click = fireEvent.click

// jsdom has no <dialog>.showModal(): a stand-in that opens it and records the call (as FolderPicker.test does).
const modalCalls = []
beforeEach(() => {
  window.location.hash = '#secret-token'
  window.localStorage.clear()
  modalCalls.length = 0
  HTMLDialogElement.prototype.showModal = function () {
    modalCalls.push(this)
    this.setAttribute('open', '')
  }
  HTMLDialogElement.prototype.close = function () {
    this.removeAttribute('open')
  }
})
afterEach(() => {
  vi.restoreAllMocks()
  vi.unstubAllGlobals()
})

// 6 confirmed Claude accounts (ids 1, 2, 3, 6, 7 and a Codex one), plus an ignored and a missing Claude account.
const extra = [
  { id: 8, provider: 'claude', label: 'Claude ignored', folder: '/home/o/.claude-ignored', status: 'ignored', workspaces: [] },
  { id: 9, provider: 'claude', label: 'Claude gone', folder: '/home/o/.claude-gone', status: 'missing', workspaces: [] },
]
const manyAccounts = { by: 'account', accounts: [...accountsStatus.accounts, ...extra] }
const oneAccount = { by: 'account', accounts: [accountList[1]] } // Claude personal only
const noClaude = { by: 'account', accounts: [accountList[3], accountList[4]] } // Codex + detected

const writes = (f) => f.calls.filter((c) => c.init.method).map((c) => ({ method: c.init.method, url: c.url, body: JSON.parse(c.init.body) }))

/** The agent-run state lives in `state.get`; a PUT of the setting changes it as the real route does. */
function stub({ get = agentRunOff, accounts = manyAccounts, put } = {}) {
  const state = { get }
  const f = fakeFetch({
    accounts,
    agentRun: (method, path, body) => {
      if (method === 'GET') return state.get
      if (put) return put(body)
      state.get = body.account === null ? agentRunOff : agentRunOn({ account: { id: Number(body.account), label: accounts.accounts.find((a) => String(a.id) === body.account)?.label, status: 'confirmed' } })
      return state.get
    },
  })
  vi.stubGlobal('fetch', f)
  return { f, state }
}

const open = async (setup) => {
  const made = stub(setup)
  const view = render(<SettingsView onOpenAgents={setup?.onOpenAgents ?? (() => {})} />)
  await screen.findByRole('heading', { name: 'Agent run account' })
  return { ...made, view }
}
const radio = (name) => screen.getByRole('radio', { name: new RegExp(name) })
const choose = (name) => {
  click(radio(name))
  click(screen.getByRole('button', { name: 'Choose' }))
}

describe('Settings, off (AC1)', () => {
  it('says Off and lists only the confirmed Claude accounts in a labelled radio group', async () => {
    await open()
    expect(screen.getByRole('heading', { name: 'Settings', level: 1 })).toBeInTheDocument()
    expect(screen.getByText('Off. The app starts no agent run.')).toBeInTheDocument()
    const group = screen.getByRole('radiogroup', { name: 'Claude accounts' })
    const names = within(group).getAllByRole('radio').map((r) => r.closest('label').querySelector('strong').textContent)
    expect(names).toEqual(['Claude default', 'Claude personal', 'Claude bare', 'Claude old', 'Claude skew'])
    // not listed: the Codex account, an ignored one, a missing one
    for (const hidden of ['Codex', 'Claude ignored', 'Claude gone']) expect(screen.queryByText(hidden)).toBeNull()
  })

  it('each row: label, folder name, and the plan limit line only when the answer has one', async () => {
    await open()
    const row = (name) => radio(name).closest('label')
    expect(row('Claude default')).toHaveTextContent('.claude')
    expect(row('Claude default')).not.toHaveTextContent('/home/o')
    expect(row('Claude default')).toHaveTextContent('5h 43% · 7d 75%')
    expect(row('Claude personal')).toHaveTextContent('5h 92% · 7d 10%')
    expect(row('Claude bare')).not.toHaveTextContent('%')
    expect(row('Claude old')).not.toHaveTextContent('%') // stale or empty windows say nothing
  })

  it('Choose stays off until an account is picked, and picking opens nothing by itself', async () => {
    await open()
    expect(screen.getByRole('button', { name: 'Choose' })).toBeDisabled()
    click(radio('Claude personal'))
    expect(screen.getByRole('button', { name: 'Choose' })).toBeEnabled()
    expect(screen.queryByRole('dialog')).toBeNull()
  })

  it('one Claude account only: the picker still shows it and the confirm step still comes', async () => {
    const { f } = await open({ accounts: oneAccount })
    expect(screen.getAllByRole('radio')).toHaveLength(1)
    choose('Claude personal')
    expect(await screen.findByRole('dialog', { name: 'Use Claude personal for agent runs?' })).toBeInTheDocument()
    expect(writes(f)).toEqual([])
  })

  it('no confirmed Claude account: the line, a link to Agents, no picker', async () => {
    const onOpenAgents = vi.fn()
    await open({ accounts: noClaude, onOpenAgents })
    expect(screen.getByText(/^No confirmed Claude account yet\./)).toBeInTheDocument()
    expect(screen.queryByRole('radiogroup')).toBeNull()
    click(screen.getByRole('button', { name: 'Agents' }))
    expect(onOpenAgents).toHaveBeenCalledTimes(1)
  })

  it('Off is a choice, not an error: no alert and no error colour class', async () => {
    const { view } = await open()
    expect(screen.queryByRole('alert')).toBeNull()
    expect(view.container.querySelector('.error')).toBeNull()
  })
})

describe('the confirm step (AC1, AC6)', () => {
  it('Choose opens a named modal dialog with the fixed note and the two buttons; nothing is sent yet', async () => {
    const { f } = await open()
    choose('Claude personal')
    const dialog = await screen.findByRole('dialog', { name: 'Use Claude personal for agent runs?' })
    expect(modalCalls).toEqual([dialog])
    expect(within(dialog).getByText(NOTE)).toBeInTheDocument()
    expect(within(dialog).getByRole('button', { name: 'Use this account' })).toBeInTheDocument()
    expect(within(dialog).getByRole('button', { name: 'Cancel' })).toBeInTheDocument()
    expect(dialog.contains(document.activeElement)).toBe(true)
    expect(writes(f)).toEqual([])
  })

  it('Use this account sends one PUT with the account and confirmed: true, then shows the chosen state', async () => {
    const { f } = await open()
    choose('Claude personal')
    const dialog = await screen.findByRole('dialog')
    click(within(dialog).getByRole('button', { name: 'Use this account' }))
    expect(await screen.findByText('The app uses this account for every agent run.')).toBeInTheDocument()
    expect(writes(f)).toEqual([{ method: 'PUT', url: '/api/agent-run/setting', body: { account: '2', confirmed: true } }])
    expect(screen.queryByRole('dialog')).toBeNull()
    expect(screen.queryByRole('radiogroup')).toBeNull()
  })

  it('a double click on Use this account sends one request', async () => {
    let release
    const { f } = await open({ put: () => new Promise((r) => (release = r)) })
    choose('Claude personal')
    const use = within(await screen.findByRole('dialog')).getByRole('button', { name: 'Use this account' })
    click(use)
    click(use)
    expect(writes(f)).toHaveLength(1)
    release(agentRunOn())
    await screen.findByText('The app uses this account for every agent run.')
    expect(writes(f)).toHaveLength(1)
  })

  it('Cancel sends nothing, closes the dialog and returns the focus to Choose', async () => {
    const { f } = await open()
    choose('Claude personal')
    click(within(await screen.findByRole('dialog')).getByRole('button', { name: 'Cancel' }))
    expect(screen.queryByRole('dialog')).toBeNull()
    expect(writes(f)).toEqual([])
    expect(screen.getByRole('button', { name: 'Choose' })).toHaveFocus()
    expect(screen.getByText('Off. The app starts no agent run.')).toBeInTheDocument()
  })

  it('Escape cancels the same way (the cancel event of the dialog)', async () => {
    const { f } = await open()
    choose('Claude personal')
    const dialog = await screen.findByRole('dialog')
    const cancel = new Event('cancel', { cancelable: true })
    act(() => void dialog.dispatchEvent(cancel))
    expect(cancel.defaultPrevented).toBe(true)
    expect(screen.queryByRole('dialog')).toBeNull()
    expect(writes(f)).toEqual([])
    expect(screen.getByRole('button', { name: 'Choose' })).toHaveFocus()
  })

  it.each([
    [400, 'This account cannot be used for agent runs. Pick another.'],
    [500, 'Could not save the setting. Try again.'],
  ])('a %i answer shows a fixed line in the dialog, never the body text', async (status, line) => {
    await open({ put: () => agentRunError(status) })
    choose('Claude personal')
    const dialog = await screen.findByRole('dialog')
    click(within(dialog).getByRole('button', { name: 'Use this account' }))
    expect(await within(dialog).findByRole('alert')).toHaveTextContent(line)
    expect(document.body.textContent).not.toContain('server text')
    expect(document.body.innerHTML).not.toContain('<script')
    expect(window.__pwned).toBeUndefined()
    expect(within(dialog).getByRole('button', { name: 'Use this account' })).toBeEnabled()
  })
})

describe('Settings, an account chosen (AC1)', () => {
  const chosen = () => agentRunOn({ account: { id: 2, label: 'Claude personal', status: 'confirmed' } })

  it('shows the label, the folder name, the sentence, Change and Turn off', async () => {
    await open({ get: chosen() })
    expect(screen.getByText('Claude personal')).toBeInTheDocument()
    expect(screen.getByText('.claude-personal')).toBeInTheDocument()
    expect(screen.getByText('The app uses this account for every agent run.')).toBeInTheDocument()
    expect(screen.getByText('5h 92% · 7d 10%')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Change' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Turn off' })).toBeInTheDocument()
    expect(screen.queryByRole('radiogroup')).toBeNull()
  })

  it('Turn off sends one PUT with account null and shows Off', async () => {
    const { f } = await open({ get: chosen() })
    const off = screen.getByRole('button', { name: 'Turn off' })
    click(off)
    click(off)
    expect(await screen.findByText('Off. The app starts no agent run.')).toBeInTheDocument()
    expect(writes(f)).toEqual([{ method: 'PUT', url: '/api/agent-run/setting', body: { account: null } }])
  })

  it('Change shows the picker again, still behind the confirm step; Cancel in the block goes back', async () => {
    const { f } = await open({ get: chosen() })
    click(screen.getByRole('button', { name: 'Change' }))
    expect(screen.getByRole('radiogroup', { name: 'Claude accounts' })).toBeInTheDocument()
    click(screen.getByRole('button', { name: 'Cancel' }))
    expect(screen.getByText('The app uses this account for every agent run.')).toBeInTheDocument()
    click(screen.getByRole('button', { name: 'Change' }))
    choose('Claude bare')
    click(within(await screen.findByRole('dialog')).getByRole('button', { name: 'Use this account' }))
    await waitFor(() => expect(writes(f)).toHaveLength(1))
    expect(writes(f)[0].body).toEqual({ account: '3', confirmed: true })
    expect(await screen.findByText('Claude bare')).toBeInTheDocument()
  })

  it('a default-account flag (set in a terminal): says so and still offers Change and Turn off', async () => {
    await open({ get: agentRunOn({ account: null }) })
    expect(screen.getByText('The app uses the default Claude account for every agent run.')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Change' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Turn off' })).toBeInTheDocument()
  })

  it('an account whose folder is gone: the note, Change and Turn off', async () => {
    await open({ get: agentRunOn({ account: { id: 9, label: 'Claude gone', status: 'missing' } }) })
    expect(screen.getByText("Claude gone")).toBeInTheDocument()
    expect(screen.getByText(/folder is gone/)).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Turn off' })).toBeInTheDocument()
  })

  it('a Turn off that fails shows a fixed line and keeps the chosen state', async () => {
    await open({ get: chosen(), put: () => agentRunError(500) })
    click(screen.getByRole('button', { name: 'Turn off' }))
    expect(await screen.findByRole('alert')).toHaveTextContent('Could not save the setting. Try again.')
    expect(screen.getByText('The app uses this account for every agent run.')).toBeInTheDocument()
    expect(document.body.textContent).not.toContain('server text')
  })
})

describe('Settings, loading and old servers', () => {
  it('shows no block when GET /api/agent-run answers 404 (the Python server)', async () => {
    vi.stubGlobal('fetch', fakeFetch({ accounts: manyAccounts }))
    render(<SettingsView onOpenAgents={() => {}} />)
    expect(await screen.findByText('No settings are available in this version of the app.')).toBeInTheDocument()
    expect(screen.queryByRole('heading', { name: 'Agent run account' })).toBeNull()
  })

  it('a failed read: one quiet error line, no picker, no second request', async () => {
    const f = fakeFetch({ agentRun: () => agentRunError(500), accounts: manyAccounts })
    vi.stubGlobal('fetch', f)
    render(<SettingsView onOpenAgents={() => {}} />)
    expect(await screen.findByText(/^Could not load the agent run account/)).toBeInTheDocument()
    expect(screen.queryByRole('radiogroup')).toBeNull()
    await act(async () => void (await new Promise((r) => setTimeout(r, 50))))
    expect(f.calls.filter((c) => c.url === '/api/agent-run')).toHaveLength(1)
    expect(document.body.textContent).not.toContain('server text')
  })
})

describe('first-open prompt (AC2, AC6)', () => {
  const claude = accountsStatus.accounts.filter((a) => a.provider === 'claude')
  const prompt = (props = {}) => render(<FirstOpenPrompt accounts={claude} suppressed={false} {...props} />)
  const agentRunCalls = (f) => f.calls.filter((c) => c.url.startsWith('/api/agent-run'))

  it('shows once when the setting is off and the browser holds no mark: a named dialog with the picker, Choose and Not now', async () => {
    stub()
    prompt()
    const dialog = await screen.findByRole('dialog', { name: 'Choose an account for agent runs' })
    expect(modalCalls).toEqual([dialog])
    expect(within(dialog).getByRole('radiogroup', { name: 'Claude accounts' })).toBeInTheDocument()
    expect(within(dialog).getByRole('button', { name: 'Choose' })).toBeInTheDocument()
    expect(within(dialog).getByRole('button', { name: 'Not now' })).toBeInTheDocument()
    expect(dialog.contains(document.activeElement)).toBe(true)
    expect(window.localStorage.getItem(MARK)).toBeNull() // shown is not answered
  })

  it('Not now sets the mark, closes, and the prompt does not come back (no request at all next time)', async () => {
    const { f } = stub()
    const { unmount } = prompt()
    click(within(await screen.findByRole('dialog')).getByRole('button', { name: 'Not now' }))
    expect(screen.queryByRole('dialog')).toBeNull()
    expect(window.localStorage.getItem(MARK)).toBe('1')
    expect(writes(f)).toEqual([])
    unmount()
    f.calls.length = 0
    prompt()
    await act(async () => {})
    expect(screen.queryByRole('dialog')).toBeNull()
    expect(agentRunCalls(f)).toEqual([])
  })

  it('Escape is Not now', async () => {
    stub()
    prompt()
    const dialog = await screen.findByRole('dialog')
    act(() => void dialog.dispatchEvent(new Event('cancel', { cancelable: true })))
    expect(screen.queryByRole('dialog')).toBeNull()
    expect(window.localStorage.getItem(MARK)).toBe('1')
  })

  it('a choice goes through the same confirm step, then sends one PUT, sets the mark and closes', async () => {
    const { f } = stub()
    prompt()
    const dialog = await screen.findByRole('dialog')
    click(within(dialog).getByRole('radio', { name: /Claude personal/ }))
    click(within(dialog).getByRole('button', { name: 'Choose' }))
    expect(await screen.findByRole('dialog', { name: 'Use Claude personal for agent runs?' })).toBeInTheDocument()
    expect(screen.getByText(NOTE)).toBeInTheDocument()
    expect(writes(f)).toEqual([])
    expect(window.localStorage.getItem(MARK)).toBeNull()
    click(screen.getByRole('button', { name: 'Use this account' }))
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull())
    expect(writes(f)).toEqual([{ method: 'PUT', url: '/api/agent-run/setting', body: { account: '2', confirmed: true } }])
    expect(window.localStorage.getItem(MARK)).toBe('1')
    expect(await screen.findByText('Agent runs now use Claude personal.')).toBeInTheDocument()
  })

  it('Cancel in the confirm step goes back to the picker and sets no mark', async () => {
    const { f } = stub()
    prompt()
    const dialog = await screen.findByRole('dialog')
    click(within(dialog).getByRole('radio', { name: /Claude personal/ }))
    click(within(dialog).getByRole('button', { name: 'Choose' }))
    click(await screen.findByRole('button', { name: 'Cancel' }))
    expect(await screen.findByRole('dialog', { name: 'Choose an account for agent runs' })).toBeInTheDocument()
    expect(writes(f)).toEqual([])
    expect(window.localStorage.getItem(MARK)).toBeNull()
  })

  it('Cancel in the confirm step keeps the last chosen account selected and puts the focus on Choose', async () => {
    stub()
    prompt()
    const dialog = await screen.findByRole('dialog')
    click(within(dialog).getByRole('radio', { name: /Claude personal/ }))
    click(within(dialog).getByRole('button', { name: 'Choose' }))
    click(await screen.findByRole('button', { name: 'Cancel' }))
    const back = await screen.findByRole('dialog', { name: 'Choose an account for agent runs' })
    expect(within(back).getByRole('radio', { name: /Claude personal/ })).toBeChecked()
    expect(within(back).getByRole('button', { name: 'Choose' })).toBeEnabled()
    expect(document.activeElement).toBe(within(back).getByRole('button', { name: 'Choose' }))
  })

  it('Escape in the confirm step does the same: the picker with the choice kept, the focus on Choose', async () => {
    stub()
    prompt()
    const dialog = await screen.findByRole('dialog')
    click(within(dialog).getByRole('radio', { name: /Claude personal/ }))
    click(within(dialog).getByRole('button', { name: 'Choose' }))
    fireEvent(await screen.findByRole('dialog', { name: /^Use Claude personal/ }), new Event('cancel', { cancelable: true }))
    const back = await screen.findByRole('dialog', { name: 'Choose an account for agent runs' })
    expect(within(back).getByRole('radio', { name: /Claude personal/ })).toBeChecked()
    expect(document.activeElement).toBe(within(back).getByRole('button', { name: 'Choose' }))
  })

  it.each([
    ['the setting is on', () => stub({ get: agentRunOn() }), {}],
    ['the First run view is open', () => stub(), { suppressed: true }],
    ['the Settings tab is open', () => stub(), { suppressed: true }],
    ['the route answers 404', () => vi.stubGlobal('fetch', fakeFetch()), {}],
    ['the read fails', () => stub({ get: agentRunError(500) }), {}],
    ['there is no confirmed Claude account', () => stub(), { accounts: [] }],
    ['the accounts have not loaded', () => stub(), { accounts: undefined }],
    ['the accounts failed to load', () => stub(), { accounts: null }],
  ])('is not shown when %s', async (_, install, props) => {
    install()
    prompt(props)
    await act(async () => void (await new Promise((r) => setTimeout(r, 30))))
    expect(screen.queryByRole('dialog')).toBeNull()
    expect(modalCalls).toEqual([])
  })

  it('shows when the First run view is left, if it has not been answered', async () => {
    stub()
    const { rerender } = prompt({ suppressed: true })
    await act(async () => void (await new Promise((r) => setTimeout(r, 30))))
    expect(screen.queryByRole('dialog')).toBeNull()
    rerender(<FirstOpenPrompt accounts={claude} suppressed={false} />)
    expect(await screen.findByRole('dialog')).toBeInTheDocument()
  })

  it('storage that throws does not break the page: the prompt may show, and Not now still closes it', async () => {
    stub()
    vi.spyOn(Storage.prototype, 'getItem').mockImplementation(() => {
      throw new Error('blocked')
    })
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
      throw new Error('blocked')
    })
    prompt()
    click(within(await screen.findByRole('dialog')).getByRole('button', { name: 'Not now' }))
    expect(screen.queryByRole('dialog')).toBeNull()
  })

  it('a choice with storage that throws still saves the setting', async () => {
    const { f } = stub()
    vi.spyOn(Storage.prototype, 'getItem').mockImplementation(() => {
      throw new Error('blocked')
    })
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
      throw new Error('blocked')
    })
    prompt()
    const dialog = await screen.findByRole('dialog')
    click(within(dialog).getByRole('radio', { name: /Claude personal/ }))
    click(within(dialog).getByRole('button', { name: 'Choose' }))
    click(await screen.findByRole('button', { name: 'Use this account' }))
    await waitFor(() => expect(writes(f)).toHaveLength(1))
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull())
  })

  it('a failed save keeps the dialog open with a fixed line and sets no mark', async () => {
    stub({ put: () => agentRunError(500) })
    prompt()
    const dialog = await screen.findByRole('dialog')
    click(within(dialog).getByRole('radio', { name: /Claude personal/ }))
    click(within(dialog).getByRole('button', { name: 'Choose' }))
    click(await screen.findByRole('button', { name: 'Use this account' }))
    expect(await screen.findByRole('alert')).toHaveTextContent('Could not save the setting. Try again.')
    expect(window.localStorage.getItem(MARK)).toBeNull()
  })
})
