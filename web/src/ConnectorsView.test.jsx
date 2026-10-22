import { render, screen, fireEvent, waitFor, within } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import App from './App.jsx'
import ConnectorsView from './ConnectorsView.jsx'
import { connectors, fakeFetch } from './test/fixtures.js'

const SECRET = 'xoxb-PLANTED-secret-123'
const PLAN = ['will change (Slack):', '  keychain: service easy-personal/slack, fields SLACK_MCP_XOXB_TOKEN=***', '  claude user scope: mcp add-json slack {}']

/** GET list from the fixture; POST/DELETE answered by `write(body, init)`; every call recorded. */
function stub(write) {
  const read = fakeFetch()
  const fn = vi.fn(async (url, init = {}) => {
    if (init.method) {
      const body = JSON.parse(init.body)
      return { ok: true, status: 200, json: async () => write(body, init, String(url)) }
    }
    return read(url, init)
  })
  vi.stubGlobal('fetch', fn)
  return fn
}
const writes = (fn) => fn.mock.calls.filter(([, init]) => init?.method)

beforeEach(() => {
  window.location.hash = '#secret-token'
})
afterEach(() => vi.unstubAllGlobals())

describe('catalog and form (AC1, AC2)', () => {
  it('groups by category, shows status yes/no and the claude.ai link first only when flagged', async () => {
    stub(() => ({}))
    render(<ConnectorsView />)
    expect(await screen.findByRole('heading', { name: 'Messaging' })).toBeInTheDocument()
    const slack = screen.getByRole('region', { name: 'Slack' })
    const discord = screen.getByRole('region', { name: 'Discord' })
    expect(within(slack).getByText('Configured: no')).toBeInTheDocument()
    expect(within(discord).getByText('Configured: yes')).toBeInTheDocument()
    const link = within(slack).getByRole('link')
    expect(link).toHaveAttribute('href', 'https://claude.ai/settings/connectors')
    expect(link).toHaveAttribute('rel', 'noopener noreferrer')
    expect(within(discord).queryByRole('link')).toBeNull()
    // the link is rendered before the Set up button
    expect(link.compareDocumentPosition(within(slack).getByRole('button')) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
  })

  it('builds the form from the catalog fields; secret fields are password inputs', async () => {
    stub(() => ({}))
    render(<ConnectorsView />)
    const section = await screen.findByRole('region', { name: 'Slack' })
    fireEvent.click(within(section).getByRole('button', { name: 'Set up Slack' }))
    expect(within(section).getByLabelText('Bot token (xoxb-...)')).toHaveAttribute('type', 'password')
    expect(within(section).getByLabelText('Bot token (xoxb-...)')).toHaveAttribute('autocomplete', 'off')
  })

  it('renders plain fields as text and secret fields as password', () => {
    // Discord is configured so it offers Remove; render its fields through the add form by flipping the flag
    stub(() => ({}))
    const flipped = { ...connectors, connectors: connectors.connectors.map((c) => ({ ...c, configured: false })) }
    vi.stubGlobal('fetch', fakeFetch({ connectors: flipped }))
    render(<ConnectorsView />)
    return screen.findByRole('region', { name: 'Discord' }).then((d) => {
      fireEvent.click(within(d).getByRole('button', { name: 'Set up Discord' }))
      expect(within(d).getByLabelText('Bot name')).toHaveAttribute('type', 'text')
      expect(within(d).getByLabelText('Bot token')).toHaveAttribute('type', 'password')
    })
  })
})

describe('confirmation and write (AC6)', () => {
  async function fillSlack(fn) {
    render(<ConnectorsView />)
    const section = await screen.findByRole('region', { name: 'Slack' })
    fireEvent.click(within(section).getByRole('button', { name: 'Set up Slack' }))
    fireEvent.change(within(section).getByLabelText('Bot token (xoxb-...)'), { target: { value: SECRET } })
    return section
  }

  it('previews first (confirm:false), writes only after Apply, then clears the secret', async () => {
    const fn = stub((body) =>
      body.confirm ? { status: 0, pending: false, lines: ["connector 'slack' is configured."] } : { status: 1, pending: true, lines: PLAN },
    )
    const section = await fillSlack(fn)
    fireEvent.click(within(section).getByRole('button', { name: 'Review changes' }))
    expect(await within(section).findByText(/mcp add-json slack/)).toBeInTheDocument()
    expect(writes(fn)).toHaveLength(1)
    const [url, init] = writes(fn)[0]
    expect(url).toBe('/api/connectors')
    expect(init.method).toBe('POST')
    expect(init.headers['Content-Type']).toBe('application/json')
    expect(init.headers['X-Dashboard-Token']).toBe('secret-token')
    expect(JSON.parse(init.body)).toEqual({ provider: 'slack', values: { SLACK_MCP_XOXB_TOKEN: SECRET }, confirm: false })
    fireEvent.click(within(section).getByRole('button', { name: 'Apply changes' }))
    expect(await within(section).findByText(/is configured/)).toBeInTheDocument()
    expect(JSON.parse(writes(fn)[1][1].body).confirm).toBe(true)
    // after submit: no input, no typed secret anywhere in the page
    expect(within(section).queryByLabelText('Bot token (xoxb-...)')).toBeNull()
    expect(document.body.innerHTML).not.toContain(SECRET)
    // re-opening the form starts empty
    await waitFor(() => expect(within(section).getByRole('button', { name: /Set up Slack|Remove Slack/ })).toBeInTheDocument())
  })

  it('cancel at the confirmation writes nothing and clears the value', async () => {
    const fn = stub(() => ({ status: 1, pending: true, lines: PLAN }))
    const section = await fillSlack(fn)
    fireEvent.click(within(section).getByRole('button', { name: 'Review changes' }))
    await within(section).findByText(/mcp add-json slack/)
    fireEvent.click(within(section).getByRole('button', { name: 'Cancel' }))
    expect(writes(fn)).toHaveLength(1) // only the preview
    fireEvent.click(within(section).getByRole('button', { name: 'Set up Slack' }))
    expect(within(section).getByLabelText('Bot token (xoxb-...)')).toHaveValue('')
  })

  it('shows a refusal (no confirmation step) and clears the value', async () => {
    const guidance = 'no usable OS keychain (Secret Service); unlock or install one, e.g. gnome-keyring.'
    const fn = stub(() => ({ status: 1, pending: false, lines: [guidance] }))
    const section = await fillSlack(fn)
    fireEvent.click(within(section).getByRole('button', { name: 'Review changes' }))
    expect(await within(section).findByText(/no usable OS keychain/)).toBeInTheDocument()
    expect(within(section).queryByRole('button', { name: 'Apply changes' })).toBeNull()
    expect(document.body.innerHTML).not.toContain(SECRET)
  })

  it('shows the claude snippet fallback text as returned', async () => {
    const snippet = 'claude mcp add-json slack \'{"type":"stdio"}\' --scope user'
    stub(() => ({ status: 0, pending: false, lines: ['the `claude` CLI is not on PATH; nothing was written.', snippet] }))
    const section = await fillSlack()
    fireEvent.click(within(section).getByRole('button', { name: 'Review changes' }))
    expect(await within(section).findByText(/claude mcp add-json slack/)).toBeInTheDocument()
  })

  it('disables the buttons while a request runs, so a double click sends one request', async () => {
    let release
    const gate = new Promise((r) => (release = r))
    const read = fakeFetch()
    const fn = vi.fn(async (url, init = {}) => {
      if (!init.method) return read(url, init)
      await gate
      return { ok: true, status: 200, json: async () => ({ status: 1, pending: true, lines: PLAN }) }
    })
    vi.stubGlobal('fetch', fn)
    const section = await fillSlack()
    const review = within(section).getByRole('button', { name: 'Review changes' })
    fireEvent.click(review)
    fireEvent.click(review)
    expect(review).toBeDisabled()
    release()
    await within(section).findByText(/mcp add-json slack/)
    expect(writes(fn)).toHaveLength(1)
  })

  it('shows the server error text on a failed request', async () => {
    const read = fakeFetch()
    vi.stubGlobal('fetch', async (url, init = {}) =>
      init.method ? { ok: false, status: 415, json: async () => ({ error: 'content-type must be application/json' }) } : read(url, init),
    )
    const section = await fillSlack()
    fireEvent.click(within(section).getByRole('button', { name: 'Review changes' }))
    expect(await within(section).findByRole('alert')).toHaveTextContent('content-type must be application/json')
  })
})

describe('remove', () => {
  it('previews then removes a configured connector through DELETE', async () => {
    const fn = stub((body) =>
      body.confirm ? { status: 0, pending: false, lines: ["connector 'discord' removed."] } : { status: 1, pending: true, lines: ['will change: delete keychain service easy-personal/discord'] },
    )
    render(<ConnectorsView />)
    const section = await screen.findByRole('region', { name: 'Discord' })
    fireEvent.click(within(section).getByRole('button', { name: 'Remove Discord' }))
    fireEvent.click(within(section).getByRole('button', { name: 'Review changes' }))
    expect(await within(section).findByText(/delete keychain service/)).toBeInTheDocument()
    expect(writes(fn)).toHaveLength(1)
    fireEvent.click(within(section).getByRole('button', { name: 'Apply removal' }))
    expect(await within(section).findByText(/removed/)).toBeInTheDocument()
    const [url, init] = writes(fn)[1]
    expect(url).toBe('/api/connectors/discord')
    expect(init.method).toBe('DELETE')
    expect(JSON.parse(init.body)).toEqual({ confirm: true })
  })
})

describe('load states and tab', () => {
  it('shows an error when the list cannot load', async () => {
    vi.stubGlobal('fetch', fakeFetch({ status: 403 }))
    render(<ConnectorsView />)
    expect(await screen.findByRole('alert')).toHaveTextContent('Could not load connectors')
  })

  it('is reachable as the Connectors tab', async () => {
    vi.stubGlobal('fetch', fakeFetch())
    render(<App />)
    fireEvent.click(screen.getByRole('tab', { name: 'Connectors' }))
    expect(await screen.findByRole('heading', { name: 'Connectors' })).toBeInTheDocument()
    expect(window.location.hash).toBe('#secret-token')
  })
})
