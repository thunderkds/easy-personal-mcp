import { render as rtlRender, screen, fireEvent, waitFor, within } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import CopyCommand, { COMMANDS, shellLine } from './CopyCommand.jsx'
import WorkspacesView from './WorkspacesView.jsx'
import { ToastProvider } from './Toast.jsx'
import { fakeFetch, workspaceOverrides } from './test/fixtures.js'

const render = (ui) => rtlRender(ui)
const click = (el) => fireEvent.click(el)
/** Choose an option of a select by the text the user reads. */
const pickIn = (scope, name, text) => {
  const select = scope.getByLabelText(name)
  const option = within(select).getAllByRole('option').find((o) => o.textContent === text)
  fireEvent.change(select, { target: { value: option.value } })
}
const pick = (name, text) => pickIn(screen, name, text)
const clip = (impl = async () => {}) => {
  const writeText = vi.fn(impl)
  Object.defineProperty(navigator, 'clipboard', { value: { writeText }, configurable: true })
  return writeText
}

describe('shellLine (AC1)', () => {
  const wake = 'claude "/wake"'
  it('wraps a plain path in single quotes', () => {
    expect(shellLine('/work/acme', wake)).toBe(`cd -- '/work/acme' && claude "/wake"`)
  })
  it('keeps a trailing slash as stored', () => {
    expect(shellLine('/work/acme/', wake)).toBe(`cd -- '/work/acme/' && claude "/wake"`)
  })
  it('keeps a path with a space inside the quotes', () => {
    expect(shellLine('/work/my repo', wake)).toBe(`cd -- '/work/my repo' && claude "/wake"`)
  })
  it("writes each ' inside the path as '\\''", () => {
    expect(shellLine("/work/it's", wake)).toBe(`cd -- '/work/it'\\''s' && claude "/wake"`)
    expect(shellLine("/a'b'c", wake)).toBe(`cd -- '/a'\\''b'\\''c' && claude "/wake"`)
    expect(shellLine("'", wake)).toBe(`cd -- ''\\''' && claude "/wake"`)
  })
  it('leaves shell syntax in a path inside the quotes, byte for byte', () => {
    expect(shellLine('/x/$(rm -rf ~)', wake)).toBe(`cd -- '/x/$(rm -rf ~)' && claude "/wake"`)
    expect(shellLine('/x/`id`', wake)).toBe("cd -- '/x/`id`' && claude \"/wake\"")
    expect(shellLine('/x;reboot', wake)).toBe(`cd -- '/x;reboot' && claude "/wake"`)
    expect(shellLine('/x && rm -rf /', wake)).toBe(`cd -- '/x && rm -rf /' && claude "/wake"`)
    expect(shellLine('/x/$HOME\\n"q"', wake)).toBe(`cd -- '/x/$HOME\\n"q"' && claude "/wake"`)
  })
  it('puts -- after cd so a path that starts with - is never read as an option', () => {
    expect(shellLine('-x', wake)).toBe(`cd -- '-x' && claude "/wake"`)
    expect(shellLine('--help', wake)).toBe(`cd -- '--help' && claude "/wake"`)
  })
  it('keeps non-ASCII text', () => {
    expect(shellLine('/work/café/日本', wake)).toBe(`cd -- '/work/café/日本' && claude "/wake"`)
  })
  it.each([
    ['newline', '/a\nb'],
    ['carriage return', '/a\rb'],
    ['NUL', '/a\0b'],
    ['tab', '/a\tb'],
    ['escape', '/a\x1bb'],
    ['U+001F', '/a\x1fb'],
    ['DEL (U+007F)', '/a\x7fb'],
  ])('refuses a path with a %s', (_name, path) => {
    expect(shellLine(path, wake)).toBeNull()
  })
  it('does not refuse U+0080 and above', () => {
    expect(shellLine('/a\u0080b', wake)).toBe(`cd -- '/a\u0080b' && claude "/wake"`)
  })
  it('refuses an empty or missing path', () => {
    expect(shellLine('', wake)).toBeNull()
    expect(shellLine(undefined, wake)).toBeNull()
  })
})

describe('allow-list', () => {
  it('holds exactly the three decided commands', () => {
    expect(COMMANDS.map((c) => [c.label, c.command])).toEqual([
      ['Start a session', 'claude "/wake"'],
      ['Continue the last session', 'claude --continue'],
      ['Verify', 'claude "/verify"'],
    ])
  })
})

describe('CopyCommand (AC2)', () => {
  beforeEach(() => clip())
  afterEach(() => {
    delete navigator.clipboard
    vi.unstubAllGlobals()
  })

  it('shows nothing when the workspace has no repo', () => {
    const { container } = render(<CopyCommand repos={[]} />)
    expect(container).toBeEmptyDOMElement()
    expect(screen.queryByRole('button', { name: 'Copy' })).toBeNull()
  })

  it('shows the command select and Copy, but no repo select, for one repo', () => {
    render(<CopyCommand repos={['/work/a']} />)
    expect(screen.getByText('Copy a Claude command')).toBeInTheDocument()
    expect(screen.getByLabelText('Command')).toBeInTheDocument()
    expect(screen.queryByLabelText('Repo')).toBeNull()
    expect(screen.getByRole('button', { name: 'Copy' })).toBeInTheDocument()
    expect(within(screen.getByLabelText('Command')).getAllByRole('option').map((o) => o.textContent)).toEqual([
      'Start a session',
      'Continue the last session',
      'Verify',
    ])
  })

  it('shows a repo select over the stored repos for two repos', () => {
    render(<CopyCommand repos={['/work/a', '/work/b']} />)
    expect(within(screen.getByLabelText('Repo')).getAllByRole('option').map((o) => o.textContent)).toEqual(['/work/a', '/work/b'])
  })

  it('copies the exact line for each allow-list entry', async () => {
    const writeText = clip()
    render(<CopyCommand repos={['/work/a']} />)
    for (const [label, line] of [
      ['Start a session', `cd -- '/work/a' && claude "/wake"`],
      ['Continue the last session', `cd -- '/work/a' && claude --continue`],
      ['Verify', `cd -- '/work/a' && claude "/verify"`],
    ]) {
      pick('Command', label)
      click(screen.getByRole('button', { name: 'Copy' }))
      await waitFor(() => expect(writeText).toHaveBeenLastCalledWith(line))
    }
    expect(writeText).toHaveBeenCalledTimes(3)
  })

  it('copies the repo that is picked, quoted for the shell', async () => {
    const writeText = clip()
    render(<CopyCommand repos={['/work/a', "/work/it's a repo"]} />)
    pick('Repo', "/work/it's a repo")
    click(screen.getByRole('button', { name: 'Copy' }))
    await waitFor(() => expect(writeText).toHaveBeenCalledWith(`cd -- '/work/it'\\''s a repo' && claude "/wake"`))
  })

  it('says "Copied" in a polite live region after Copy, and clears it when a choice changes', async () => {
    render(<CopyCommand repos={['/work/a']} />)
    const status = screen.getByTestId('copy-status')
    expect(status).toHaveAttribute('aria-live', 'polite')
    expect(status).toHaveTextContent('')
    click(screen.getByRole('button', { name: 'Copy' }))
    await waitFor(() => expect(status).toHaveTextContent('Copied'))
    pick('Command', 'Verify')
    expect(status).toHaveTextContent('')
  })

  it('offers no Copy for a repo path with a control character, and says why', () => {
    render(<CopyCommand repos={['/work/a', '/work/b\nc']} />)
    expect(screen.getByRole('button', { name: 'Copy' })).toBeInTheDocument()
    pick('Repo', '/work/b\nc')
    expect(screen.queryByRole('button', { name: 'Copy' })).toBeNull()
    expect(screen.getByText('This repo path has a control character; copy is off.')).toBeInTheDocument()
    expect(navigator.clipboard.writeText).not.toHaveBeenCalled()
    pick('Repo', '/work/a')
    expect(screen.getByRole('button', { name: 'Copy' })).toBeInTheDocument()
    expect(screen.queryByText(/control character/)).toBeNull()
  })

  it('keeps each block state its own', async () => {
    const writeText = clip()
    render(
      <>
        <section aria-label="one"><CopyCommand repos={['/one']} /></section>
        <section aria-label="two"><CopyCommand repos={['/two']} /></section>
      </>,
    )
    const one = screen.getByRole('region', { name: 'one' })
    const two = screen.getByRole('region', { name: 'two' })
    pickIn(within(two), 'Command', 'Verify')
    click(within(one).getByRole('button', { name: 'Copy' }))
    await waitFor(() => expect(within(one).getByTestId('copy-status')).toHaveTextContent('Copied'))
    expect(within(two).getByTestId('copy-status')).toHaveTextContent('')
    expect(within(two).getByRole('option', { name: 'Verify' }).selected).toBe(true)
    expect(writeText).toHaveBeenCalledWith(`cd -- '/one' && claude "/wake"`)
  })
})

describe('clipboard refused (AC3)', () => {
  afterEach(() => {
    delete navigator.clipboard
  })
  const line = `cd -- '/work/a' && claude "/wake"`
  const expectFallback = () => {
    const field = screen.getByRole('textbox', { name: 'Command to copy' })
    expect(field).toHaveAttribute('readonly')
    expect(field).toHaveValue(line)
    expect(field).toHaveFocus()
    expect(field.selectionStart).toBe(0)
    expect(field.selectionEnd).toBe(line.length)
    expect(screen.getByText('Copy it with Ctrl+C.')).toBeInTheDocument()
    expect(screen.getByTestId('copy-status')).not.toHaveTextContent('Copied')
  }

  it('shows the line selected in a read-only input when writeText rejects', async () => {
    clip(async () => {
      throw new Error('denied')
    })
    render(<CopyCommand repos={['/work/a']} />)
    expect(screen.queryByRole('textbox')).toBeNull()
    click(screen.getByRole('button', { name: 'Copy' }))
    await waitFor(expectFallback)
  })

  it('shows it when writeText throws at once', async () => {
    clip(() => {
      throw new Error('denied')
    })
    render(<CopyCommand repos={['/work/a']} />)
    click(screen.getByRole('button', { name: 'Copy' }))
    await waitFor(expectFallback)
  })

  it('shows it when navigator.clipboard is missing', async () => {
    Object.defineProperty(navigator, 'clipboard', { value: undefined, configurable: true })
    render(<CopyCommand repos={['/work/a']} />)
    click(screen.getByRole('button', { name: 'Copy' }))
    await waitFor(expectFallback)
  })

  it('shows it when navigator.clipboard has no writeText', async () => {
    Object.defineProperty(navigator, 'clipboard', { value: {}, configurable: true })
    render(<CopyCommand repos={['/work/a']} />)
    click(screen.getByRole('button', { name: 'Copy' }))
    await waitFor(expectFallback)
  })

  it('hides the field again when a choice changes', async () => {
    clip(async () => {
      throw new Error('denied')
    })
    render(<CopyCommand repos={['/work/a']} />)
    click(screen.getByRole('button', { name: 'Copy' }))
    await waitFor(expectFallback)
    pick('Command', 'Verify')
    expect(screen.queryByRole('textbox')).toBeNull()
    expect(screen.queryByText('Copy it with Ctrl+C.')).toBeNull()
  })
})

describe('on the Workspaces page (AC2, AC4)', () => {
  const wrapper = ToastProvider
  const stub = () => {
    const read = fakeFetch(workspaceOverrides())
    const fn = vi.fn(async (url, init = {}) => read(url, init))
    vi.stubGlobal('fetch', fn)
    return fn
  }
  beforeEach(() => {
    window.location.hash = '#secret-token'
    vi.spyOn(Storage.prototype, 'setItem')
  })
  afterEach(() => {
    delete navigator.clipboard
    vi.unstubAllGlobals()
    vi.restoreAllMocks()
  })

  it('gives a block to a workspace with repos only: two repos get a repo select, none and orphan get no block', async () => {
    stub()
    clip()
    rtlRender(<WorkspacesView />, { wrapper })
    const company = await screen.findByRole('region', { name: 'company' })
    expect(within(company).getByText('Copy a Claude command')).toBeInTheDocument()
    expect(within(company).getByLabelText('Repo')).toBeInTheDocument()
    expect(within(await screen.findByRole('region', { name: 'side' })).queryByText('Copy a Claude command')).toBeNull()
    expect(within(await screen.findByRole('region', { name: 'orphan' })).queryByText('Copy a Claude command')).toBeNull()
  })

  it('sends nothing to the server and stores nothing on Copy', async () => {
    const fn = stub()
    const writeText = clip()
    rtlRender(<WorkspacesView />, { wrapper })
    const company = await screen.findByRole('region', { name: 'company' })
    const before = fn.mock.calls.length
    const setItem = vi.spyOn(Storage.prototype, 'setItem')
    const logs = [vi.spyOn(console, 'log'), vi.spyOn(console, 'info'), vi.spyOn(console, 'warn'), vi.spyOn(console, 'error')]
    click(within(company).getByRole('button', { name: 'Copy' }))
    await waitFor(() => expect(writeText).toHaveBeenCalledWith(`cd -- '/work/company' && claude "/wake"`))
    await waitFor(() => expect(within(company).getByTestId('copy-status')).toHaveTextContent('Copied'))
    expect(fn.mock.calls.length).toBe(before)
    expect(setItem).not.toHaveBeenCalled()
    expect(localStorage.length).toBe(0)
    logs.forEach((l) => expect(l).not.toHaveBeenCalled())
  })

  it('also sends nothing when the clipboard is refused', async () => {
    const fn = stub()
    clip(async () => {
      throw new Error('denied')
    })
    rtlRender(<WorkspacesView />, { wrapper })
    const company = await screen.findByRole('region', { name: 'company' })
    const before = fn.mock.calls.length
    click(within(company).getByRole('button', { name: 'Copy' }))
    await waitFor(() => expect(within(company).getByRole('textbox', { name: 'Command to copy' })).toBeInTheDocument())
    expect(fn.mock.calls.length).toBe(before)
    expect(localStorage.length).toBe(0)
  })
})
