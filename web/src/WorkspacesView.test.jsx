import { render, screen, fireEvent, waitFor, within } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import App from './App.jsx'
import WorkspacesView from './WorkspacesView.jsx'
import { fakeFetch, legacyOne, workspaceOverrides, workspacesFull } from './test/fixtures.js'

const SECRET = 'xoxb-PLANTED-secret-123'

/** Reads from the fixtures; writes go to `write(method, path, body)` (an Error answers HTTP 400) and are recorded. */
function stub({ write = () => ({}), overrides = workspaceOverrides(), hold } = {}) {
  const read = fakeFetch(overrides)
  const fn = vi.fn(async (url, init = {}) => {
    if (!init.method) return read(url, init)
    if (hold) await hold
    const out = write(init.method, new URL(String(url), 'http://127.0.0.1').pathname, JSON.parse(init.body))
    if (out instanceof Error) return { ok: false, status: 400, json: async () => ({ error: out.message }) }
    return { ok: true, status: 200, json: async () => out }
  })
  vi.stubGlobal('fetch', fn)
  return fn
}
const writes = (fn) => fn.mock.calls.filter(([, init]) => init?.method).map(([url, init]) => [init.method, url, JSON.parse(init.body), init])
/** A plan answer until `confirm` is true, then the applied answer. */
const planned = (lines, done = 'done.') => (_m, _p, b) =>
  b.confirm ? { status: 0, pending: false, lines: [done] } : { status: 1, pending: true, lines }
const card = (name) => screen.findByRole('region', { name })
const row = (name) => screen.getByRole('listitem', { name })
const click = (el) => fireEvent.click(el)
const type = (el, value) => fireEvent.change(el, { target: { value } })

beforeEach(() => {
  window.location.hash = '#secret-token'
})
afterEach(() => vi.unstubAllGlobals())

describe('cards (AC1)', () => {
  it('shows one card per workspace with color, email, repo count, connectors by alias and used-by accounts', async () => {
    stub()
    render(<WorkspacesView />)
    const company = await card('company')
    expect(within(company).getByText('me@company.test')).toBeInTheDocument()
    expect(within(company).getByText('2 repos')).toBeInTheDocument()
    expect(within(company).getByText('company-slack')).toBeInTheDocument()
    expect(within(company).getByText(/Used by: Claude company/)).toBeInTheDocument()
    expect(within(company).getByTestId('swatch')).toHaveStyle({ backgroundColor: 'rgb(11, 95, 214)' })
    const side = await card('side')
    expect(within(side).getByText('0 repos')).toBeInTheDocument()
    expect(within(side).getByText(/Used by: no agent account/)).toBeInTheDocument()
    expect(within(side).getByText(/No connectors/)).toBeInTheDocument()
    expect(screen.getByRole('button', { name: '+ Add workspace' })).toBeInTheDocument()
  })

  it('shows the orphan card as not active, with Assign for each of its connectors', async () => {
    stub()
    render(<WorkspacesView />)
    const orphan = await card('orphan')
    expect(within(orphan).getByText(/Not active in any agent account/)).toBeInTheDocument()
    expect(within(orphan).getAllByRole('button', { name: 'Assign' })).toHaveLength(2)
    expect(within(orphan).queryByRole('button', { name: /Open/ })).toBeNull()
  })

  it('with no workspace shows only the orphan card and the Add button', async () => {
    stub({ overrides: workspaceOverrides({ workspaces: { workspaces: [workspacesFull.workspaces[0]] }, connectors: { connectors: [], workspace_connectors: [] } }) })
    render(<WorkspacesView />)
    expect(await card('orphan')).toBeInTheDocument()
    expect(screen.getByText(/No workspaces yet/)).toBeInTheDocument()
    expect(screen.getByRole('button', { name: '+ Add workspace' })).toBeInTheDocument()
  })

  it('shows an error when the lists cannot load', async () => {
    vi.stubGlobal('fetch', fakeFetch({ status: 403 }))
    render(<WorkspacesView />)
    expect(await screen.findByRole('alert')).toHaveTextContent('Could not load workspaces')
  })

  it('replaces the Connectors tab with a Workspaces tab', async () => {
    stub()
    render(<App />)
    expect(screen.queryByRole('tab', { name: 'Connectors' })).toBeNull()
    click(screen.getByRole('tab', { name: 'Workspaces' }))
    expect(await screen.findByRole('heading', { name: 'Workspaces' })).toBeInTheDocument()
    expect(window.location.hash).toBe('#secret-token')
  })
})

describe('add and edit a workspace (AC2)', () => {
  async function openAdd() {
    render(<WorkspacesView />)
    await card('company')
    click(screen.getByRole('button', { name: '+ Add workspace' }))
    return screen.findByRole('region', { name: 'Workspace form' })
  }

  it('creates a workspace from the form with repos added and removed', async () => {
    const fn = stub({ write: () => ({ workspace: { id: 9, name: 'new-one', is_orphan: false, repos: ['/a'] } }) })
    const form = await openAdd()
    type(within(form).getByLabelText('Name'), 'new-one')
    type(within(form).getByLabelText('Color (#rrggbb)'), '#112233')
    type(within(form).getByLabelText('Email'), 'a@b.test')
    type(within(form).getByLabelText('Description'), 'hello')
    click(within(form).getByRole('button', { name: '+ Add repo' }))
    type(within(form).getByLabelText('Repo 1'), '/a')
    click(within(form).getByRole('button', { name: '+ Add repo' }))
    type(within(form).getByLabelText('Repo 2'), '/b')
    click(within(form).getByRole('button', { name: 'Remove repo 2' }))
    expect(within(form).queryByLabelText('Repo 2')).toBeNull()
    click(within(form).getByRole('button', { name: 'Save' }))
    await waitFor(() => expect(writes(fn)).toHaveLength(1))
    const [method, url, body, init] = writes(fn)[0]
    expect([method, url]).toEqual(['POST', '/api/workspaces'])
    expect(init.headers['X-Dashboard-Token']).toBe('secret-token')
    expect(body).toEqual({ name: 'new-one', color: '#112233', icon: null, email: 'a@b.test', description: 'hello', repos: ['/a'] })
    expect(await screen.findByRole('status')).toHaveTextContent("Workspace 'new-one' created")
  })

  it('shows the API message for an invalid name and keeps the form', async () => {
    stub({ write: () => new Error('name must match [a-z0-9][a-z0-9-]{0,39}') })
    const form = await openAdd()
    type(within(form).getByLabelText('Name'), 'Bad Name')
    click(within(form).getByRole('button', { name: 'Save' }))
    expect(await within(form).findByRole('alert')).toHaveTextContent('name must match')
    expect(within(form).getByLabelText('Name')).toHaveValue('Bad Name')
  })

  it('edits a workspace without a rename at once (no plan)', async () => {
    const fn = stub({ write: () => ({ status: 0, pending: false, lines: [], workspace: workspacesFull.workspaces[1] }) })
    render(<WorkspacesView />)
    click(within(await card('company')).getByRole('button', { name: 'Open' }))
    const form = await screen.findByRole('region', { name: 'Workspace form' })
    expect(within(form).getByLabelText('Name')).toHaveValue('company')
    expect(within(form).getByLabelText('Repo 2')).toHaveValue('/work/company-web')
    type(within(form).getByLabelText('Email'), '')
    click(within(form).getByRole('button', { name: 'Save' }))
    await waitFor(() => expect(writes(fn)).toHaveLength(1))
    const [method, url, body] = writes(fn)[0]
    expect([method, url]).toEqual(['PATCH', '/api/workspaces/2'])
    expect(body).toMatchObject({ name: 'company', email: null, repos: ['/work/company', '/work/company-web'], confirm: false })
    expect(screen.queryByRole('button', { name: 'Confirm' })).toBeNull()
  })

  it('shows the plan for a rename, then sends the same edit with confirm', async () => {
    const fn = stub({ write: planned(['will change (workspace 2, now acme):', '  connector c1a2b3c4: alias company-slack -> acme-slack'], 'renamed.') })
    render(<WorkspacesView />)
    click(within(await card('company')).getByRole('button', { name: 'Open' }))
    const form = await screen.findByRole('region', { name: 'Workspace form' })
    type(within(form).getByLabelText('Name'), 'acme')
    click(within(form).getByRole('button', { name: 'Save' }))
    expect(await screen.findByText(/alias company-slack -> acme-slack/)).toBeInTheDocument()
    expect(writes(fn)).toHaveLength(1)
    click(screen.getByRole('button', { name: 'Confirm' }))
    expect(await screen.findByText('renamed.')).toBeInTheDocument()
    expect(writes(fn)).toHaveLength(2)
    expect(writes(fn)[1][2]).toMatchObject({ name: 'acme', confirm: true })
  })

  it('does not offer a form for the orphan workspace', async () => {
    stub()
    render(<WorkspacesView />)
    const orphan = await card('orphan')
    expect(within(orphan).queryByRole('button', { name: 'Open' })).toBeNull()
  })
})

describe('add a connector (AC3, AC7)', () => {
  async function openCompanyAndPickSlack() {
    render(<WorkspacesView />)
    click(within(await card('company')).getByRole('button', { name: 'Open' }))
    const form = await screen.findByRole('region', { name: 'Workspace form' })
    click(within(form).getByRole('button', { name: 'Add connector' }))
    return form
  }
  async function fillDiscord(form) {
    fireEvent.change(within(form).getByLabelText('Provider'), { target: { value: 'discord' } })
    type(within(form).getByLabelText('Bot name'), 'bot')
    type(within(form).getByLabelText('Bot token'), SECRET)
  }

  it('offers only providers the workspace does not hold, builds the form from catalog fields, claude.ai link first', async () => {
    stub()
    const form = await openCompanyAndPickSlack()
    const options = within(within(form).getByLabelText('Provider')).getAllByRole('option').map((o) => o.value)
    expect(options).toEqual(['', 'discord']) // slack is already in company
    fireEvent.change(within(form).getByLabelText('Provider'), { target: { value: 'discord' } })
    expect(within(form).getByLabelText('Bot name')).toHaveAttribute('type', 'text')
    expect(within(form).getByLabelText('Bot token')).toHaveAttribute('type', 'password')
    expect(within(form).getByLabelText('Bot token')).toHaveAttribute('autocomplete', 'off')
    expect(within(form).queryByRole('link')).toBeNull() // discord has no official connector
  })

  it('shows the claude.ai link before the fields when the provider has one', async () => {
    stub({ overrides: workspaceOverrides({ workspaces: { workspaces: workspacesFull.workspaces } }) })
    render(<WorkspacesView />)
    click(within(await card('side')).getByRole('button', { name: 'Open' }))
    const form = await screen.findByRole('region', { name: 'Workspace form' })
    click(within(form).getByRole('button', { name: 'Add connector' }))
    fireEvent.change(within(form).getByLabelText('Provider'), { target: { value: 'slack' } })
    const link = within(form).getByRole('link')
    expect(link).toHaveAttribute('href', 'https://claude.ai/settings/connectors')
    expect(link).toHaveAttribute('rel', 'noopener noreferrer')
    expect(link.compareDocumentPosition(within(form).getByLabelText('Bot token (xoxb-...)')) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
  })

  it('previews first, writes only after Confirm, then clears the secret and never renders it back', async () => {
    const fn = stub({ write: planned(['will change (Discord):', '  keychain: DISCORD_TOKEN=***'], "connector 'c9' is configured.") })
    const form = await openCompanyAndPickSlack()
    await fillDiscord(form)
    type(within(form).getByLabelText('Alias (optional)'), 'my-bot')
    click(within(form).getByRole('button', { name: 'Review changes' }))
    expect(await screen.findByText(/keychain: DISCORD_TOKEN=\*\*\*/)).toBeInTheDocument()
    expect(writes(fn)).toHaveLength(1)
    const [method, url, body] = writes(fn)[0]
    expect([method, url]).toEqual(['POST', '/api/connectors'])
    expect(body).toEqual({ provider: 'discord', workspace: 'company', alias: 'my-bot', values: { DISCORD_BOT_NAME: 'bot', DISCORD_TOKEN: SECRET }, confirm: false })
    expect(document.body.innerHTML).not.toContain(SECRET)
    click(screen.getByRole('button', { name: 'Confirm' }))
    expect(await screen.findByText(/is configured/)).toBeInTheDocument()
    expect(writes(fn)[1][2].confirm).toBe(true)
    expect(document.body.innerHTML).not.toContain(SECRET)
    // reopening the form starts empty
    click(await within(screen.getByRole('region', { name: 'Workspace form' })).findByRole('button', { name: 'Add connector' }))
    fireEvent.change(screen.getByLabelText('Provider'), { target: { value: 'discord' } })
    expect(screen.getByLabelText('Bot token')).toHaveValue('')
  })

  it('omits the alias when left empty', async () => {
    const fn = stub({ write: planned(['plan']) })
    const form = await openCompanyAndPickSlack()
    await fillDiscord(form)
    click(within(form).getByRole('button', { name: 'Review changes' }))
    await screen.findByText('plan')
    expect(writes(fn)[0][2]).not.toHaveProperty('alias')
  })

  it('cancel at the confirmation writes nothing more and clears the secret', async () => {
    const fn = stub({ write: planned(['plan']) })
    const form = await openCompanyAndPickSlack()
    await fillDiscord(form)
    click(within(form).getByRole('button', { name: 'Review changes' }))
    await screen.findByText('plan')
    click(screen.getByRole('button', { name: 'Cancel' }))
    expect(writes(fn)).toHaveLength(1)
    expect(document.body.innerHTML).not.toContain(SECRET)
    click(await within(screen.getByRole('region', { name: 'Workspace form' })).findByRole('button', { name: 'Add connector' }))
    fireEvent.change(screen.getByLabelText('Provider'), { target: { value: 'discord' } })
    expect(screen.getByLabelText('Bot token')).toHaveValue('')
  })

  it('shows a refusal as the final answer (no Confirm) and clears the secret', async () => {
    stub({ write: () => ({ status: 1, pending: false, lines: ['no usable OS keychain (Secret Service); unlock or install one.'] }) })
    const form = await openCompanyAndPickSlack()
    await fillDiscord(form)
    click(within(form).getByRole('button', { name: 'Review changes' }))
    expect(await screen.findByText(/no usable OS keychain/)).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Confirm' })).toBeNull()
    expect(document.body.innerHTML).not.toContain(SECRET)
    click(await within(screen.getByRole('region', { name: 'Workspace form' })).findByRole('button', { name: 'Add connector' }))
    fireEvent.change(screen.getByLabelText('Provider'), { target: { value: 'discord' } })
    expect(screen.getByLabelText('Bot token')).toHaveValue('')
  })

  it('shows the server error text when the request fails, without echoing the secret', async () => {
    stub({ write: () => new Error('workspace is required (a workspace name)') })
    const form = await openCompanyAndPickSlack()
    await fillDiscord(form)
    click(within(form).getByRole('button', { name: 'Review changes' }))
    const alert = await screen.findByRole('alert')
    expect(alert).toHaveTextContent('workspace is required')
    expect(alert).not.toHaveTextContent(SECRET)
  })

  it('disables the buttons while a request runs, so a double click sends one request', async () => {
    let release
    const hold = new Promise((r) => (release = r))
    const fn = stub({ write: planned(['plan']), hold })
    const form = await openCompanyAndPickSlack()
    await fillDiscord(form)
    const review = within(form).getByRole('button', { name: 'Review changes' })
    click(review)
    click(review)
    expect(review).toBeDisabled()
    release()
    await screen.findByText('plan')
    expect(writes(fn)).toHaveLength(1)
  })
})

describe('connector move, alias and delete with a plan (AC4)', () => {
  async function openCompany() {
    render(<WorkspacesView />)
    click(within(await card('company')).getByRole('button', { name: 'Open' }))
    await screen.findByRole('region', { name: 'Workspace form' })
    return row('company-slack')
  }

  it('moves a connector: shows the plan, sends confirm only after Confirm', async () => {
    const fn = stub({ write: planned(["will change (connector c1a2b3c4): move to workspace 'side', alias company-slack -> side-slack"], 'moved.') })
    const r = await openCompany()
    fireEvent.change(within(r).getByLabelText('Move to…'), { target: { value: 'side' } })
    click(within(r).getByRole('button', { name: 'Move' }))
    expect(await screen.findByText(/move to workspace 'side'/)).toBeInTheDocument()
    expect(writes(fn)).toHaveLength(1)
    expect(writes(fn)[0].slice(0, 3)).toEqual(['POST', '/api/connectors/c1a2b3c4/move', { workspace: 'side', confirm: false }])
    click(screen.getByRole('button', { name: 'Confirm' }))
    expect(await screen.findByText('moved.')).toBeInTheDocument()
    expect(writes(fn)[1][2]).toEqual({ workspace: 'side', confirm: true })
  })

  it('offers orphan as a target ("unassign") but not the current workspace', async () => {
    stub()
    const r = await openCompany()
    const targets = within(within(r).getByLabelText('Move to…')).getAllByRole('option').map((o) => o.value)
    expect(targets).toEqual(['', 'orphan', 'side'])
  })

  it('shows the API message when a move is refused and changes nothing', async () => {
    const fn = stub({ write: () => new Error("workspace 'side' already has a slack connector (c1, alias side-slack); nothing was changed") })
    const r = await openCompany()
    fireEvent.change(within(r).getByLabelText('Move to…'), { target: { value: 'side' } })
    click(within(r).getByRole('button', { name: 'Move' }))
    expect(await screen.findByRole('alert')).toHaveTextContent('already has a slack connector')
    expect(screen.queryByRole('button', { name: 'Confirm' })).toBeNull()
    expect(writes(fn)).toHaveLength(1)
  })

  it('changes an alias through PUT, plan first', async () => {
    const fn = stub({ write: planned(['will change (connector c1a2b3c4): alias company-slack -> work-slack'], 'aliased.') })
    const r = await openCompany()
    expect(within(r).getByRole('button', { name: 'Save alias' })).toBeDisabled()
    type(within(r).getByLabelText('Alias'), 'work-slack')
    click(within(r).getByRole('button', { name: 'Save alias' }))
    expect(await screen.findByText(/alias company-slack -> work-slack/)).toBeInTheDocument()
    expect(writes(fn)[0].slice(0, 3)).toEqual(['PUT', '/api/connectors/c1a2b3c4/alias', { alias: 'work-slack', confirm: false }])
    click(screen.getByRole('button', { name: 'Confirm' }))
    expect(await screen.findByText('aliased.')).toBeInTheDocument()
    expect(writes(fn)[1][2]).toEqual({ alias: 'work-slack', confirm: true })
  })

  it('sends alias null when the alias is emptied (default)', async () => {
    const fn = stub({ write: planned(['plan']) })
    const r = await openCompany()
    type(within(r).getByLabelText('Alias'), '')
    click(within(r).getByRole('button', { name: 'Save alias' }))
    await screen.findByText('plan')
    expect(writes(fn)[0][2]).toEqual({ alias: null, confirm: false })
  })

  it('deletes one connector through DELETE, plan first', async () => {
    const fn = stub({ write: planned(['will change: remove company-slack from Claude company; delete keychain service easy-personal/c1a2b3c4'], 'deleted.') })
    const r = await openCompany()
    click(within(r).getByRole('button', { name: 'Delete' }))
    expect(await screen.findByText(/delete keychain service/)).toBeInTheDocument()
    expect(writes(fn)[0].slice(0, 3)).toEqual(['DELETE', '/api/connectors/c1a2b3c4', { confirm: false }])
    click(screen.getByRole('button', { name: 'Confirm' }))
    expect(await screen.findByText('deleted.')).toBeInTheDocument()
    expect(writes(fn)[1][2]).toEqual({ confirm: true })
  })

  it('assigns an orphan connector to a workspace from the orphan card', async () => {
    const fn = stub({ write: planned(['assign plan'], 'assigned.') })
    render(<WorkspacesView />)
    const orphan = await card('orphan')
    const discord = within(orphan).getByRole('listitem', { name: 'discord' })
    fireEvent.change(within(discord).getByLabelText('Assign to…'), { target: { value: 'company' } })
    click(within(discord).getByRole('button', { name: 'Assign' }))
    expect(await screen.findByText('assign plan')).toBeInTheDocument()
    expect(writes(fn)[0].slice(0, 3)).toEqual(['POST', '/api/connectors/discord/move', { workspace: 'company', confirm: false }])
    click(screen.getByRole('button', { name: 'Confirm' }))
    expect(await screen.findByText('assigned.')).toBeInTheDocument()
  })

  it('reloads the lists after a confirmed change', async () => {
    const fn = stub({ write: planned(['plan'], 'ok.') })
    const r = await openCompany()
    click(within(r).getByRole('button', { name: 'Delete' }))
    await screen.findByText('plan')
    const before = fn.mock.calls.filter(([u]) => u === '/api/workspaces').length
    click(screen.getByRole('button', { name: 'Confirm' }))
    await screen.findByText('ok.')
    await waitFor(() => expect(fn.mock.calls.filter(([u]) => u === '/api/workspaces').length).toBe(before + 1))
  })
})

describe('delete a workspace (AC5)', () => {
  it('explains that connectors go to orphan, shows the plan, and returns to the cards after Confirm', async () => {
    const fn = stub({ write: (_m, _p, b) =>
        b.confirm
          ? { status: 0, pending: false, lines: ['workspace deleted.'], deleted: 2 }
          : { status: 1, pending: true, lines: ["will change (delete workspace 'company'):", '  connector c1a2b3c4 -> orphan'] },
    })
    render(<WorkspacesView />)
    click(within(await card('company')).getByRole('button', { name: 'Open' }))
    const form = await screen.findByRole('region', { name: 'Workspace form' })
    expect(within(form).getByText(/connectors move to orphan/i)).toBeInTheDocument()
    click(within(form).getByRole('button', { name: 'Delete workspace' }))
    expect(await screen.findByText(/connector c1a2b3c4 -> orphan/)).toBeInTheDocument()
    expect(writes(fn)[0].slice(0, 3)).toEqual(['DELETE', '/api/workspaces/2', { confirm: false }])
    // the form stays until the owner confirms
    expect(screen.getByRole('region', { name: 'Workspace form' })).toBeInTheDocument()
    fn.mockClear()
    click(screen.getByRole('button', { name: 'Confirm' }))
    expect(await screen.findByText('workspace deleted.')).toBeInTheDocument()
    expect(writes(fn)[0][2]).toEqual({ confirm: true })
    await waitFor(() => expect(screen.queryByRole('region', { name: 'Workspace form' })).toBeNull())
    expect(screen.getByText('workspace deleted.')).toBeInTheDocument()
  })
})

describe('legacy connectors banner (AC6)', () => {
  it('is absent when the API lists none', async () => {
    stub()
    render(<WorkspacesView />)
    await card('company')
    expect(screen.queryByRole('region', { name: 'Legacy connectors' })).toBeNull()
  })

  it('is absent when the legacy check fails (e.g. no keychain)', async () => {
    const read = fakeFetch(workspaceOverrides())
    vi.stubGlobal('fetch', async (url, init = {}) => (String(url) === '/api/connectors/legacy' ? { ok: false, status: 503, json: async () => ({ error: 'no keychain' }) } : read(url, init)))
    render(<WorkspacesView />)
    await card('company')
    expect(screen.queryByRole('region', { name: 'Legacy connectors' })).toBeNull()
  })

  it('offers adoption, shows the plan, then adopts only after Confirm', async () => {
    const fn = stub({
      overrides: workspaceOverrides({ legacy: legacyOne }),
      write: planned(["will change: adopt 'slack' into orphan"], "adopted 'slack'."),
    })
    render(<WorkspacesView />)
    const banner = await screen.findByRole('region', { name: 'Legacy connectors' })
    expect(within(banner).getByText(/slack/)).toBeInTheDocument()
    expect(banner).toHaveTextContent('1 connector was set up before workspaces: slack. Adopt it into the orphan workspace to manage it here.')
    click(within(banner).getByRole('button', { name: 'Adopt' }))
    expect(await screen.findByText(/adopt 'slack' into orphan/)).toBeInTheDocument()
    expect(writes(fn)[0].slice(0, 3)).toEqual(['POST', '/api/connectors/legacy', { confirm: false }])
    click(screen.getByRole('button', { name: 'Confirm' }))
    expect(await screen.findByText("adopted 'slack'.")).toBeInTheDocument()
    expect(writes(fn)[1][2]).toEqual({ confirm: true })
  })

  it('says "Adopt them" for more than one', async () => {
    const two = { legacy: [...legacyOne.legacy, { id: 'discord', keychain: true, claude: false, foreign: false }] }
    stub({ overrides: workspaceOverrides({ legacy: two }) })
    render(<WorkspacesView />)
    const banner = await screen.findByRole('region', { name: 'Legacy connectors' })
    expect(banner).toHaveTextContent('2 connectors were set up before workspaces: slack, discord. Adopt them into the orphan workspace to manage them here.')
  })

  it('notes a same-name entry that is not ours will be left alone', async () => {
    stub({ overrides: workspaceOverrides({ legacy: { legacy: [{ id: 'slack', keychain: true, claude: false, foreign: true }] } }) })
    render(<WorkspacesView />)
    const banner = await screen.findByRole('region', { name: 'Legacy connectors' })
    expect(within(banner).getByText(/not created by easy-personal/)).toBeInTheDocument()
  })
})
