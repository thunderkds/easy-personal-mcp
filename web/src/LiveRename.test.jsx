import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import LiveView from './LiveView.jsx'
import { NAME_MAX } from './SessionName.jsx'
import { appServer, appSession } from './test/appSessions.js'
import { FakeEventSource, liveRow, pastServer, snapshot, stream } from './test/liveStream.js'

// Own names in the panel header (T119, FR-127): an inline field, Enter saves through PUT /api/sessions/{id}/name, Esc cancels, empty removes; the shown
// name falls back to Claude's own, then to the project and the short id. The route is mocked as T117 answers it.
const SID = 'aaaaaaaa-1111-2222'
const rowEl = () => document.getElementById(`live-row-${SID}`)
const panel = () => screen.getByRole('region', { name: /^Session in / })
const heading = () => within(panel()).getByRole('heading', { level: 2 })
const renameButton = () => within(panel()).getByRole('button', { name: 'Rename' })
const field = () => within(panel()).getByLabelText('Name')

beforeEach(() => {
  window.location.hash = '#secret-token'
  window.localStorage.clear()
  FakeEventSource.instances = []
  vi.stubGlobal('EventSource', FakeEventSource)
})
afterEach(() => vi.unstubAllGlobals())

/** Render the Live view with one terminal session and open its panel. */
async function open(server, over = {}) {
  vi.stubGlobal('fetch', server)
  render(<LiveView />)
  stream().emit('snapshot', snapshot([liveRow(SID, over)]))
  fireEvent.click(rowEl())
  await screen.findByRole('log', { name: 'Conversation' })
}
const type = (text) => fireEvent.change(field(), { target: { value: text } })
const enter = () => fireEvent.keyDown(field(), { key: 'Enter' })
const escape = () => fireEvent.keyDown(field(), { key: 'Escape' })

describe('the shown name (AC 2)', () => {
  it('is the owner\'s name, else Claude\'s own, else the project and the short id', async () => {
    await open(pastServer(), { name: 'mine', proc: { busy: false, name: 'claude name' } })
    expect(heading()).toHaveTextContent('mine')
    expect(rowEl()).toHaveTextContent('mine')
  })

  it('Claude\'s own name when the owner gave none', async () => {
    await open(pastServer(), { proc: { busy: false, name: 'claude name' } })
    expect(heading()).toHaveTextContent('claude name')
    expect(rowEl()).toHaveTextContent('claude name')
  })

  it('the project and the short session id when neither is there', async () => {
    await open(pastServer(), { project: 'alpha' })
    expect(heading()).toHaveTextContent('alpha aaaaaaaa')
    expect(heading()).not.toHaveTextContent(SID)
  })
})

describe('rename in the panel header (AC 2)', () => {
  it('Rename opens an inline field with the focus; Enter saves through the route, the token in the header, and the header and the row show it at once', async () => {
    const server = pastServer()
    await open(server, { proc: { busy: false, name: 'claude name' } })
    fireEvent.click(renameButton())
    expect(field()).toHaveFocus()
    expect(field()).toHaveValue('') // the owner has no name yet: the field is empty, Claude's is only the hint
    expect(field()).toHaveAttribute('maxlength', String(NAME_MAX))
    type('  my review  ')
    enter()
    await waitFor(() => expect(heading()).toHaveTextContent('my review'))
    const [call] = server.renames()
    expect(call.path).toBe(`/api/sessions/${SID}/name`)
    expect(call.body).toEqual({ name: 'my review', confirmed: true })
    expect(call.init.headers['X-Dashboard-Token']).toBe('secret-token')
    expect(rowEl()).toHaveTextContent('my review')
    expect(screen.queryByLabelText('Name')).toBeNull()
    expect(renameButton()).toHaveFocus()
  })

  it('Esc cancels without a request and the panel stays open (an empty field does not close it)', async () => {
    const server = pastServer()
    await open(server, { name: 'old' })
    fireEvent.click(renameButton())
    expect(field()).toHaveValue('old')
    type('')
    escape()
    expect(server.renames()).toHaveLength(0)
    expect(screen.queryByLabelText('Name')).toBeNull()
    expect(heading()).toHaveTextContent('old')
    expect(panel()).toBeInTheDocument()
    expect(renameButton()).toHaveFocus()
  })

  it('an empty field removes the name: null goes to the route, and the shown name falls back', async () => {
    const server = pastServer()
    await open(server, { name: 'old', proc: { busy: false, name: 'claude name' } })
    fireEvent.click(renameButton())
    type('')
    enter()
    await waitFor(() => expect(heading()).toHaveTextContent('claude name'))
    expect(server.renames()[0].body).toEqual({ name: null, confirmed: true })
    expect(rowEl()).toHaveTextContent('claude name')
  })

  it('a field of spaces removes the name too; an unchanged name sends nothing', async () => {
    const server = pastServer()
    await open(server, { name: 'old' })
    fireEvent.click(renameButton())
    enter() // unchanged
    expect(server.renames()).toHaveLength(0)
    expect(screen.queryByLabelText('Name')).toBeNull()
    fireEvent.click(renameButton())
    type('   ')
    enter()
    await waitFor(() => expect(heading()).toHaveTextContent('alpha aaaaaaaa'))
    expect(server.renames()).toHaveLength(1)
    expect(server.renames()[0].body.name).toBeNull()
  })

  it('a refusal shows the server\'s one line, keeps the field and the text, and changes nothing shown', async () => {
    const server = pastServer({ rename: () => ({ status: 400, body: { error: 'name must be 1 to 80 characters after cleaning' } }) })
    await open(server, { name: 'old' })
    fireEvent.click(renameButton())
    type('new')
    enter()
    expect(await within(panel()).findByRole('alert')).toHaveTextContent('name must be 1 to 80 characters after cleaning')
    expect(field()).toHaveValue('new')
    expect(field()).toHaveAttribute('aria-invalid', 'true')
    fireEvent.click(within(panel()).getByText('Name')) // still editing
    escape()
    expect(heading()).toHaveTextContent('old')
  })

  it('a server that is not there says so in plain words', async () => {
    await open(pastServer({ rename: () => ({ status: 500, body: {} }) }), { name: 'old' })
    fireEvent.click(renameButton())
    type('new')
    enter()
    expect(await within(panel()).findByRole('alert')).toHaveTextContent('Could not save the name. Try again.')
  })

  it('one request at a time: a second Enter while one is out sends nothing more', async () => {
    let release
    const gate = new Promise((r) => (release = r))
    const server = pastServer({ rename: async (id, body) => (await gate, { id, name: body.name }) })
    await open(server)
    fireEvent.click(renameButton())
    type('slow')
    enter()
    enter()
    expect(server.renames()).toHaveLength(1)
    expect(field()).toBeDisabled()
    await act(async () => release())
    await waitFor(() => expect(heading()).toHaveTextContent('slow'))
    expect(server.renames()).toHaveLength(1)
  })

  it('an Enter that ends an input method composition does not save', async () => {
    const server = pastServer()
    await open(server)
    fireEvent.click(renameButton())
    type('にほん')
    fireEvent.keyDown(field(), { key: 'Enter', isComposing: true })
    fireEvent.keyDown(field(), { key: 'Enter', keyCode: 229 })
    expect(server.renames()).toHaveLength(0)
  })

  it('shows the name as text, never as HTML, and keeps it out of localStorage and the URL', async () => {
    const server = pastServer()
    await open(server)
    fireEvent.click(renameButton())
    type('<img src=x onerror=alert(1)>')
    enter()
    await waitFor(() => expect(heading()).toHaveTextContent('<img src=x onerror=alert(1)>'))
    expect(heading().querySelector('img')).toBeNull()
    expect(window.localStorage.length).toBe(0)
    expect(server.renames()[0].url).toBe(`/api/sessions/${SID}/name`)
  })

  it('the snapshot takes over again: the same name is no longer held, and a later change from elsewhere shows', async () => {
    await open(pastServer())
    fireEvent.click(renameButton())
    type('x')
    enter()
    await waitFor(() => expect(heading()).toHaveTextContent('x'))
    stream().emit('snapshot', snapshot([liveRow(SID, { name: 'x' })]))
    expect(heading()).toHaveTextContent('x')
    stream().emit('snapshot', snapshot([liveRow(SID, { name: 'y' })]))
    expect(heading()).toHaveTextContent('y')
  })

  it('a held name is released when its session leaves the list: it does not come back with the session', async () => {
    await open(pastServer())
    fireEvent.click(renameButton())
    type('temporary')
    enter()
    await waitFor(() => expect(rowEl()).toHaveTextContent('temporary'))
    stream().emit('snapshot', snapshot([])) // the session is gone
    stream().emit('snapshot', snapshot([liveRow(SID)])) // it is back, and the server has no name for it
    expect(rowEl()).not.toHaveTextContent('temporary')
    expect(rowEl()).toHaveTextContent('alpha')
  })

  it('a session with no Claude session id yet has no Rename (the route needs one)', async () => {
    const app = appServer({ sessions: [appSession({ id: 'a9', session_id: null, state: 'working' })] })
    vi.stubGlobal('fetch', async (url, init) => (String(url).startsWith('/api/app-sessions') ? app(url, init) : { ok: false, status: 404, json: async () => ({}) }))
    render(<LiveView appSessions />)
    stream().emit('snapshot', snapshot([]))
    const row = await waitFor(() => {
      const el = document.getElementById('live-row-app:a9')
      expect(el).not.toBeNull()
      return el
    })
    fireEvent.click(row)
    expect(within(panel()).queryByRole('button', { name: 'Rename' })).toBeNull()
  })
})
