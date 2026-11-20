import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { render, screen, waitFor, within } from '@testing-library/react'
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

const CANNOT = /entries cannot be shown on this page/
const shownText = (e) => ['user', 'reply', 'assistant'].includes(e.kind)
const toolName = (e) => e.kind === 'tool' && typeof e.name === 'string' && e.name !== ''

describe('the page over the route\'s own answer', () => {
  it.each(['session', 'agent'])('draws every entry of the %s golden answer: its exact text and its tool names, and no cannot-show line', async (name) => {
    const answer = golden(name)
    const log = await show(answer)
    expect(log.textContent).not.toMatch(CANNOT)
    expect(answer.entries.some(shownText)).toBe(true)
    expect(answer.entries.some(toolName)).toBe(true)
    for (const e of answer.entries.filter(shownText)) expect(log).toHaveTextContent(e.text)
    for (const e of answer.entries.filter(toolName)) expect(within(log).getAllByText(new RegExp(`^${e.name}`)).length).toBeGreaterThan(0)
  })

  it('draws the skipped run of the main thread', async () => {
    const log = await show(golden('session'))
    expect(log).toHaveTextContent('2 other records')
  })
})

describe('entries that cannot be drawn (AC 5)', () => {
  const known = (text, k) => ({ kind: 'user', text, offset: k })

  it('a mixed list draws what it can and counts the rest', async () => {
    const log = await show({ entries: [known('first known', 1), { kind: 'brand-new', text: 'x', offset: 2 }, { kind: 'assistant', text: 'second known', offset: 3 }], next: 10, complete: true })
    expect(log).toHaveTextContent('first known')
    expect(log).toHaveTextContent('second known')
    expect(log).toHaveTextContent('1 of 3 entries cannot be shown on this page.')
  })

  it('a list where none can be drawn says M of M, never an empty box', async () => {
    const log = await show({ entries: [{ type: 'user', text: 'old shape' }, { kind: 'brand-new', text: 'x' }], next: 10, complete: true })
    expect(log).toHaveTextContent('2 of 2 entries cannot be shown on this page.')
    expect(log).not.toHaveTextContent('old shape')
  })
})
