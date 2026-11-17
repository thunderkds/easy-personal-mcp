import { fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import NewSession, { BYPASS, BYPASS_NOTE, FORK_NOTE, MODELS, MODES, NOTE, STILL_ACTIVE, repoChoices } from './NewSession.jsx'
import { appServer, workspaces } from './test/appSessions.js'

const accounts = [
  { id: 1, label: 'Claude default', provider: 'claude', status: 'confirmed' },
  { id: 2, label: 'Claude personal', provider: 'claude', status: 'confirmed' },
]
let server
beforeEach(() => {
  window.location.hash = '#secret-token'
  server = appServer({ routes: { 'POST /api/app-sessions': () => ({ status: 202, body: { id: 'a9' } }) } })
  vi.stubGlobal('fetch', server)
})
afterEach(() => {
  vi.unstubAllGlobals()
})

const setup = (props = {}) => {
  const onStarted = vi.fn()
  const onCancel = vi.fn()
  render(<NewSession workspaces={workspaces.workspaces} accounts={accounts} defaultAccount="2" onStarted={onStarted} onCancel={onCancel} {...props} />)
  return { onStarted, onCancel }
}
const options = (label) => within(screen.getByLabelText(label)).getAllByRole('option').map((o) => o.textContent)
const fill = (text = 'fix the build') => fireEvent.change(screen.getByLabelText('First prompt'), { target: { value: text } })

describe('the form (AC 1)', () => {
  it('has the fixed model and mode lists of the server, nothing typed freely', () => {
    setup()
    expect(options('Model')).toEqual(['default', 'opus', 'sonnet', 'haiku'])
    expect(options('Permission mode')).toEqual(['default', 'acceptEdits', 'auto', 'plan', 'dontAsk', 'bypassPermissions'])
    expect(MODELS).toEqual(['default', 'opus', 'sonnet', 'haiku'])
    expect(MODES).toEqual(['default', 'acceptEdits', 'auto', 'plan', 'dontAsk', 'bypassPermissions'])
    expect(screen.getByLabelText('Model').tagName).toBe('SELECT')
    expect(screen.getByLabelText('Permission mode').tagName).toBe('SELECT')
    expect(screen.getByLabelText('Repo').tagName).toBe('SELECT') // no free path field
    expect(screen.getByLabelText(/New worktree/)).not.toBeChecked()
    expect(screen.getByLabelText('Permission mode')).toHaveValue('default')
  })

  it('lists stored repos only: the orphan workspace and a workspace with none are left out', () => {
    setup()
    expect(options('Repo')).toEqual(['/work/company', '/work/company-web', '/home/o/notes'])
    expect(repoChoices(workspaces.workspaces).map((r) => r.workspace)).toEqual(['company', 'company', 'home'])
    expect(screen.getByRole('group', { name: 'company' })).toBeInTheDocument()
    expect(screen.queryByRole('group', { name: 'orphan' })).toBeNull()
  })

  it('offers the confirmed Claude accounts and starts on the agent run account', () => {
    setup()
    expect(options('Account')).toEqual(['Claude default', 'Claude personal'])
    expect(screen.getByLabelText('Account')).toHaveValue('2')
  })

  it('with the default Claude Code account as the agent run account, that is the first choice', () => {
    setup({ defaultAccount: '' })
    expect(options('Account')).toEqual(['Default Claude Code account', 'Claude default', 'Claude personal'])
    expect(screen.getByLabelText('Account')).toHaveValue('')
  })

  it('starts on the first repo of the workspace it was opened for', () => {
    setup({ initialWorkspace: 'home' })
    expect(screen.getByLabelText('Repo')).toHaveDisplayValue('/home/o/notes')
  })

  it('Review stays off until there is a first prompt, and a blank one does not count', () => {
    setup()
    const review = screen.getByRole('button', { name: 'Review' })
    expect(review).toBeDisabled()
    fill('   ')
    expect(review).toBeDisabled()
    fill('go')
    expect(review).toBeEnabled()
  })

  it('the size limit is in bytes, not characters: 2- and 3-byte characters count in full', () => {
    setup()
    const review = screen.getByRole('button', { name: 'Review' })
    fill('\u00e9'.repeat(50_000)) // 50,000 characters, exactly 100,000 bytes
    expect(review).toBeEnabled()
    expect(screen.queryByRole('alert')).toBeNull()
    fill('\u00e9'.repeat(50_001)) // 100,002 bytes
    expect(review).toBeDisabled()
    expect(screen.getByRole('alert')).toHaveTextContent('Too long')
    fill('\u3042'.repeat(33_334)) // 33,334 characters, 100,002 bytes
    expect(review).toBeDisabled()
    fill('\u3042'.repeat(33_333)) // 99,999 bytes
    expect(review).toBeEnabled()
  })

  it('the hint words the limit as a size', () => {
    setup()
    expect(screen.getByText(/size limit is 100,000 bytes/)).toBeInTheDocument()
  })

  it('says what to do when no repo is stored', () => {
    const { onCancel } = setup({ workspaces: [workspaces.workspaces[0], workspaces.workspaces[2]] })
    expect(screen.getByText(/No repo is stored yet/)).toBeInTheDocument()
    expect(screen.queryByLabelText('First prompt')).toBeNull()
    fireEvent.click(screen.getByRole('button', { name: 'Cancel' }))
    expect(onCancel).toHaveBeenCalled()
  })
})

describe('the confirm step', () => {
  it('names the repo, the account, the mode and the model, shows the fixed note, and sends nothing before Start session', () => {
    setup()
    fireEvent.change(screen.getByLabelText('Account'), { target: { value: '1' } })
    fireEvent.change(screen.getByLabelText('Permission mode'), { target: { value: 'plan' } })
    fill()
    fireEvent.click(screen.getByRole('button', { name: 'Review' }))
    expect(screen.getByRole('heading', { name: 'Start this session?' })).toHaveFocus()
    const facts = screen.getByText('Account', { selector: 'dt' }).closest('dl')
    expect(facts).toHaveTextContent('Claude default')
    expect(facts).toHaveTextContent('plan')
    expect(facts).toHaveTextContent('/work/company')
    expect(screen.getByText(NOTE)).toBeInTheDocument()
    expect(server.calls.filter((c) => c.method === 'POST')).toHaveLength(0)
    fireEvent.click(screen.getByRole('button', { name: 'Back' }))
    expect(screen.getByLabelText('First prompt')).toHaveValue('fix the build') // the form keeps what was typed
  })

  it('Start session posts exactly the form fields with confirmed: true, once, and reports the new id', async () => {
    const { onStarted } = setup({ initialWorkspace: 'company' })
    fireEvent.change(screen.getByLabelText('Model'), { target: { value: 'opus' } })
    fireEvent.click(screen.getByLabelText(/New worktree/))
    fill('hello <b>there</b>')
    fireEvent.click(screen.getByRole('button', { name: 'Review' }))
    const start = screen.getByRole('button', { name: 'Start session' })
    fireEvent.click(start)
    fireEvent.click(start) // a second click while the request is out is dropped
    await waitFor(() => expect(onStarted).toHaveBeenCalledWith('a9'))
    const posts = server.posts('/api/app-sessions')
    expect(posts).toHaveLength(1)
    expect(posts[0].body).toEqual({ repo: '/work/company', workspace: 'company', account: '2', model: 'opus', mode: 'default', worktree: true, prompt: 'hello <b>there</b>', confirmed: true })
    expect(posts[0].init.headers['X-Dashboard-Token']).toBe('secret-token')
    expect(posts[0].url).toBe('/api/app-sessions') // no token, no text in the URL
  })

  it('the default Claude Code account goes as account null', async () => {
    const { onStarted } = setup({ defaultAccount: '' })
    fill()
    fireEvent.click(screen.getByRole('button', { name: 'Review' }))
    fireEvent.click(screen.getByRole('button', { name: 'Start session' }))
    await waitFor(() => expect(onStarted).toHaveBeenCalled())
    expect(server.posts('/api/app-sessions')[0].body.account).toBeNull()
  })

  it('a refusal shows the server\'s words and keeps the owner on the step; a bare failure says to try again', async () => {
    server = appServer({ routes: { 'POST /api/app-sessions': () => ({ status: 409, body: { error: '4 app sessions are going; stop one first' } }) } })
    vi.stubGlobal('fetch', server)
    const { onStarted } = setup()
    fill()
    fireEvent.click(screen.getByRole('button', { name: 'Review' }))
    fireEvent.click(screen.getByRole('button', { name: 'Start session' }))
    expect(await screen.findByRole('alert')).toHaveTextContent('4 app sessions are going; stop one first')
    expect(onStarted).not.toHaveBeenCalled()
    expect(screen.getByRole('button', { name: 'Start session' })).toBeEnabled()
    vi.stubGlobal('fetch', async () => {
      throw new TypeError('network down')
    })
    fireEvent.click(screen.getByRole('button', { name: 'Start session' }))
    await waitFor(() => expect(screen.getByRole('alert')).toHaveTextContent('Could not start the session. Try again.'))
  })
})

describe('bypass permissions needs a second, explicit confirm', () => {
  const toBypass = () => {
    fireEvent.change(screen.getByLabelText('Permission mode'), { target: { value: BYPASS } })
    fill()
    fireEvent.click(screen.getByRole('button', { name: 'Review' }))
  }

  it('the first confirm only continues; nothing is posted until the second step is confirmed', async () => {
    const { onStarted } = setup()
    toBypass()
    expect(screen.queryByRole('button', { name: 'Start session' })).toBeNull()
    expect(screen.getByText('Mode', { selector: 'dt' }).nextSibling).toHaveTextContent(BYPASS)
    fireEvent.click(screen.getByRole('button', { name: 'Continue' }))
    expect(server.calls.filter((c) => c.method === 'POST')).toHaveLength(0)
    expect(screen.getByRole('heading', { name: 'Skip every permission prompt?' })).toHaveFocus()
    expect(screen.getByText(BYPASS_NOTE)).toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: 'Start without permission prompts' }))
    await waitFor(() => expect(onStarted).toHaveBeenCalledWith('a9'))
    expect(server.posts('/api/app-sessions')[0].body.mode).toBe(BYPASS)
  })

  it('Back from the second step returns to the first, and a changed mode needs no second step', async () => {
    const { onStarted } = setup()
    toBypass()
    fireEvent.click(screen.getByRole('button', { name: 'Continue' }))
    fireEvent.click(screen.getByRole('button', { name: 'Back' }))
    expect(screen.getByRole('heading', { name: 'Start this session?' })).toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: 'Back' }))
    fireEvent.change(screen.getByLabelText('Permission mode'), { target: { value: 'acceptEdits' } })
    fireEvent.click(screen.getByRole('button', { name: 'Review' }))
    fireEvent.click(screen.getByRole('button', { name: 'Start session' }))
    await waitFor(() => expect(onStarted).toHaveBeenCalled())
    expect(server.posts('/api/app-sessions')[0].body.mode).toBe('acceptEdits')
  })
})

// T105 (FR-112): the same form for a stored Claude session (resume, fork) and for a repo's newest conversation (continue).
describe('resume mode: Resume in app and Fork in app', () => {
  const session = { id: 'abcdef12-3456-7890', project: 'alpha', account: 'Claude default', fork: false }
  const next = () => {
    fill('carry on')
    fireEvent.click(screen.getByRole('button', { name: 'Review' }))
  }

  it('names the session (short id and project), has no repo or worktree field, and asks for account, model, mode and a first prompt', () => {
    setup({ resume: session })
    expect(screen.getByRole('heading', { name: 'Resume session' })).toBeInTheDocument()
    expect(screen.getByText('abcdef12')).toBeInTheDocument()
    expect(screen.getByText(/alpha/)).toBeInTheDocument()
    expect(screen.queryByLabelText('Repo')).toBeNull()
    expect(screen.queryByLabelText(/New worktree/)).toBeNull()
    for (const label of ['Account', 'Model', 'Permission mode', 'First prompt']) expect(screen.getByLabelText(label)).toBeInTheDocument()
    expect(screen.queryByText(FORK_NOTE)).toBeNull() // no fork, no note
    expect(screen.getByLabelText('Account')).toHaveValue('1') // the session's own account, found by its name
  })

  it('Resume posts resume: <id> with the form fields and confirmed: true, and no repo, workspace, worktree or fork', async () => {
    const { onStarted } = setup({ resume: session })
    fireEvent.change(screen.getByLabelText('Model'), { target: { value: 'opus' } })
    next()
    expect(screen.getByRole('heading', { name: 'Start this session?' })).toHaveFocus()
    expect(screen.getByText('Session', { selector: 'dt' }).closest('dl')).toHaveTextContent('alpha')
    expect(screen.getByText('Resume', { selector: 'dt' }).nextSibling).toHaveTextContent('the same session')
    expect(screen.queryByText('Worktree', { selector: 'dt' })).toBeNull()
    expect(server.posts('/api/app-sessions')).toHaveLength(0)
    fireEvent.click(screen.getByRole('button', { name: 'Start session' }))
    await waitFor(() => expect(onStarted).toHaveBeenCalledWith('a9'))
    const body = server.posts('/api/app-sessions')[0].body
    expect(body).toEqual({ resume: 'abcdef12-3456-7890', account: '1', model: 'opus', mode: 'default', prompt: 'carry on', confirmed: true })
  })

  it('a session of an account that is not confirmed here starts on the agent run account', () => {
    setup({ resume: { ...session, account: 'Gone account' } })
    expect(screen.getByLabelText('Account')).toHaveValue('2')
  })

  it('Fork shows the fixed note on the form and on the confirm step, and posts fork: true', async () => {
    const { onStarted } = setup({ resume: { ...session, fork: true } })
    expect(screen.getByRole('heading', { name: 'Fork session' })).toBeInTheDocument()
    expect(screen.getByText('The open window does not see a fork: it becomes a new session.')).toBeInTheDocument()
    next()
    expect(screen.getByRole('heading', { name: 'Start a fork of this session?' })).toBeInTheDocument()
    expect(screen.getByText(FORK_NOTE)).toBeInTheDocument()
    expect(screen.getByText('Resume', { selector: 'dt' }).nextSibling).toHaveTextContent('a fork')
    fireEvent.click(screen.getByRole('button', { name: 'Start session' }))
    await waitFor(() => expect(onStarted).toHaveBeenCalled())
    expect(server.posts('/api/app-sessions')[0].body).toMatchObject({ resume: 'abcdef12-3456-7890', fork: true, confirmed: true })
  })

  it('a 409 "still active" switches to Fork with the note, then Start session sends fork: true', async () => {
    let calls = 0
    server = appServer({
      routes: { 'POST /api/app-sessions': (body) => (++calls === 1 ? { status: 409, body: { error: STILL_ACTIVE } } : body.fork ? { status: 202, body: { id: 'f1' } } : { status: 409, body: { error: STILL_ACTIVE } }) },
    })
    vi.stubGlobal('fetch', server)
    const { onStarted } = setup({ resume: session })
    next()
    fireEvent.click(screen.getByRole('button', { name: 'Start session' }))
    expect(await screen.findByRole('alert')).toHaveTextContent(STILL_ACTIVE)
    expect(screen.getByRole('heading', { name: 'Start a fork of this session?' })).toBeInTheDocument()
    expect(screen.getByText(FORK_NOTE)).toBeInTheDocument()
    expect(onStarted).not.toHaveBeenCalled()
    fireEvent.click(screen.getByRole('button', { name: 'Start session' }))
    await waitFor(() => expect(onStarted).toHaveBeenCalledWith('f1'))
    const posts = server.posts('/api/app-sessions')
    expect(posts[0].body.fork).toBeUndefined()
    expect(posts[1].body).toMatchObject({ resume: 'abcdef12-3456-7890', fork: true })
  })

  it('the bypass mode still needs its second step after a switch to Fork', async () => {
    server = appServer({ routes: { 'POST /api/app-sessions': () => ({ status: 409, body: { error: STILL_ACTIVE } }) } })
    vi.stubGlobal('fetch', server)
    setup({ resume: session })
    fireEvent.change(screen.getByLabelText('Permission mode'), { target: { value: BYPASS } })
    next()
    fireEvent.click(screen.getByRole('button', { name: 'Continue' }))
    fireEvent.click(screen.getByRole('button', { name: 'Start without permission prompts' }))
    await screen.findByRole('alert')
    expect(screen.getByRole('button', { name: 'Continue' })).toBeInTheDocument() // back on the first confirm step
    expect(screen.queryByRole('button', { name: 'Start without permission prompts' })).toBeNull()
  })

  it.each([
    [404, 'no such Claude session'],
    [404, 'the folder of this session is not stored yet; try again after the next ingest'],
    [409, 'this session is running in an app session; stop that one first'],
    [400, 'the folder of this session no longer exists'],
  ])('a %i shows the server\'s words: %s (no switch to Fork)', async (status, error) => {
    server = appServer({ routes: { 'POST /api/app-sessions': () => ({ status, body: { error } }) } })
    vi.stubGlobal('fetch', server)
    const { onStarted } = setup({ resume: session })
    next()
    fireEvent.click(screen.getByRole('button', { name: 'Start session' }))
    expect(await screen.findByRole('alert')).toHaveTextContent(error)
    expect(screen.queryByText(FORK_NOTE)).toBeNull()
    expect(screen.getByRole('heading', { name: 'Start this session?' })).toBeInTheDocument()
    expect(onStarted).not.toHaveBeenCalled()
  })

  it('a 409 on a fork that was asked for does not loop: the words are shown and the owner stays', async () => {
    server = appServer({ routes: { 'POST /api/app-sessions': () => ({ status: 409, body: { error: STILL_ACTIVE } }) } })
    vi.stubGlobal('fetch', server)
    setup({ resume: { ...session, fork: true } })
    next()
    fireEvent.click(screen.getByRole('button', { name: 'Start session' }))
    expect(await screen.findByRole('alert')).toHaveTextContent(STILL_ACTIVE)
    expect(server.posts('/api/app-sessions')).toHaveLength(1)
  })

  it('the text is React text: markup in the project or the prompt is not parsed', () => {
    const { container } = render(<NewSession workspaces={[]} accounts={accounts} defaultAccount="2" resume={{ ...session, project: '<img src=x onerror=alert(1)>' }} onStarted={() => {}} onCancel={() => {}} />)
    expect(container.querySelector('img')).toBeNull()
    expect(screen.getByText(/<img src=x/)).toBeInTheDocument()
  })
})

describe('continue mode: Continue in app', () => {
  const where = { workspace: 'company', repo: '/work/company-web' }

  it('fixes the repo (no picker, no worktree) and posts continue: true with the repo and workspace', async () => {
    const { onStarted } = setup({ continueIn: where })
    expect(screen.getByRole('heading', { name: 'Continue session' })).toBeInTheDocument()
    expect(screen.getByText('/work/company-web')).toBeInTheDocument()
    expect(screen.queryByLabelText('Repo')).toBeNull()
    expect(screen.queryByLabelText(/New worktree/)).toBeNull()
    fill('go on')
    fireEvent.click(screen.getByRole('button', { name: 'Review' }))
    const facts = screen.getByText('Repo', { selector: 'dt' }).closest('dl')
    expect(facts).toHaveTextContent('/work/company-web')
    expect(facts).toHaveTextContent('the newest conversation')
    expect(screen.queryByText('Worktree', { selector: 'dt' })).toBeNull()
    fireEvent.click(screen.getByRole('button', { name: 'Start session' }))
    await waitFor(() => expect(onStarted).toHaveBeenCalledWith('a9'))
    expect(server.posts('/api/app-sessions')[0].body).toEqual({ continue: true, repo: '/work/company-web', workspace: 'company', account: '2', model: 'default', mode: 'default', prompt: 'go on', confirmed: true })
  })

  it('a 409 for a folder that is still active shows the server\'s words and stays on Continue (the fix is to Fork from Live)', async () => {
    const error = 'a session in this folder is still active in another window; resume it with Fork instead'
    server = appServer({ routes: { 'POST /api/app-sessions': () => ({ status: 409, body: { error } }) } })
    vi.stubGlobal('fetch', server)
    setup({ continueIn: where })
    fill()
    fireEvent.click(screen.getByRole('button', { name: 'Review' }))
    fireEvent.click(screen.getByRole('button', { name: 'Start session' }))
    expect(await screen.findByRole('alert')).toHaveTextContent(error)
    expect(screen.queryByText(FORK_NOTE)).toBeNull()
    expect(screen.getByRole('heading', { name: 'Start this session?' })).toBeInTheDocument()
  })

  it('the new session form is unchanged: no resume field, no continue field', async () => {
    const { onStarted } = setup()
    fill()
    fireEvent.click(screen.getByRole('button', { name: 'Review' }))
    fireEvent.click(screen.getByRole('button', { name: 'Start session' }))
    await waitFor(() => expect(onStarted).toHaveBeenCalled())
    const body = server.posts('/api/app-sessions')[0].body
    expect(body).not.toHaveProperty('resume')
    expect(body).not.toHaveProperty('continue')
    expect(body).not.toHaveProperty('fork')
  })
})
