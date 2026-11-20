import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import Conversation from './SubAgents.jsx'

// T129: the contract between the route and the page. The page reads what the Rust route really answers: the golden answers of `rust/crates/core/tests/transcript_golden/`
// (written by the parser itself), not entries built by a test. The first version of the page read `kind` while the route sent `type`, and every conversation was an empty box.
const golden = (name) => JSON.parse(readFileSync(resolve(process.cwd(), `../rust/crates/core/tests/transcript_golden/${name}.golden.json`), 'utf8'))
const reply = (body) => ({ ok: true, status: 200, json: async () => body })

beforeEach(() => {
  window.location.hash = '#secret-token'
})
afterEach(() => vi.unstubAllGlobals())

async function show(read) {
  vi.stubGlobal('fetch', async (url) => (String(url).endsWith('/agents') ? reply({ agents: [], total: 0 }) : reply(read)))
  render(<Conversation key="s" sessionId="aaaaaaaa-1111" />)
  await waitFor(() => expect(screen.queryByText('Loading the conversation')).toBeNull())
  return screen.getByRole('log', { name: 'Conversation' })
}

describe('the page over the route\'s own answer', () => {
  it('draws the main thread: the typed text, the assistant text, the reply, the tool calls and the skipped run', async () => {
    const answer = golden('session')
    const log = await show(answer)
    for (const e of answer.entries.filter((x) => x.kind === 'user' || x.kind === 'assistant' || x.kind === 'reply')) expect(log).toHaveTextContent(e.text)
    expect(log).toHaveTextContent('Bash')
    expect(log).toHaveTextContent('Read')
    expect(log).toHaveTextContent('2 other records')
    expect(log.children.length).toBeGreaterThan(0)
  })

  it('draws a sub-agent log', async () => {
    const answer = golden('agent')
    const log = await show(answer)
    for (const e of answer.entries.filter((x) => x.kind === 'user' || x.kind === 'assistant')) expect(log).toHaveTextContent(e.text)
    expect(log).toHaveTextContent('Agent')
  })
})

describe('entries that cannot be drawn (AC 5)', () => {
  it('say so in one line, never an empty box', async () => {
    const log = await show({ entries: [{ type: 'user', text: 'old shape' }, { kind: 'brand-new', text: 'x' }], next: 10, complete: true })
    expect(log).toHaveTextContent('This page cannot show these entries')
  })
})
