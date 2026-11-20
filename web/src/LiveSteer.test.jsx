import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import SessionPanel from './SessionPanel.jsx'
import { EFFORTS, MODELS } from './NewSession.jsx'

// T126 (FR-131): Model / Mode / Effort pickers and Context in a going app session's header. Each change waits for the server, which waits for
// the CLI's answer: on success the header takes the server's view, on a refusal the picker goes back and the CLI's line is shown.
const view = (over = {}) => ({
  id: 'a1',
  state: 'idle',
  reason: null,
  session_id: null,
  account: null,
  model: 'sonnet',
  mode: 'default',
  effort: 'default',
  folder: '/work/company',
  worktree: false,
  branch: null,
  changed_files: [],
  items: [],
  ...over,
})

const CONTEXT = { used: 27501, limit: 1000000, percent: 3, categories: [{ id: 'system_prompt', kind: 'used', tokens: 3543 }, { id: 'free_space', kind: 'free', tokens: 939499 }] }

/** A fake server for the steering routes: `refuse` maps an action to the CLI's line (409); `hold` keeps one action's answer until released. */
function server({ refuse = {}, hold = null } = {}) {
  const posts = []
  let current = view()
  let release = () => {}
  const fn = vi.fn(async (url, init = {}) => {
    const path = new URL(String(url), 'http://127.0.0.1').pathname
    const method = init.method ?? 'GET'
    const reply = (status, body) => ({ ok: status < 300, status, json: async () => body })
    if (method === 'GET' && path === '/api/app-sessions/a1/context') return reply(200, CONTEXT)
    const action = path.split('/').pop()
    if (method === 'POST' && ['model', 'mode', 'effort'].includes(action)) {
      const body = JSON.parse(init.body)
      posts.push({ action, body, token: init.headers['X-Dashboard-Token'] })
      const answer = () => {
        if (refuse[action]) return reply(409, { error: refuse[action] })
        current = { ...current, [action]: body[action] }
        return reply(200, current)
      }
      if (hold === action) return new Promise((resolve) => (release = () => resolve(answer())))
      return answer()
    }
    return reply(404, { error: 'no such route' })
  })
  return Object.assign(fn, { posts, release: () => release() })
}

let onChange
const mount = (app = view()) => {
  onChange = vi.fn()
  return render(<SessionPanel s={{ appOnly: true, attention: [] }} app={app} now={Date.now()} onChange={onChange} onClosed={vi.fn()} onBack={vi.fn()} />)
}
const picker = (name) => within(screen.getByRole('region', { name: /^Session in / })).getByRole('combobox', { name })

beforeEach(() => {
  window.location.hash = '#secret-token'
})
afterEach(() => vi.unstubAllGlobals())

describe('the steering pickers (AC 5, U1)', () => {
  it('show the current model, mode and effort in the header with the fixed lists', () => {
    vi.stubGlobal('fetch', server())
    mount(view({ model: 'opus[1m]', mode: 'plan', effort: 'high' }))
    expect(picker('Model')).toHaveValue('opus[1m]')
    expect(picker('Mode')).toHaveValue('plan')
    expect(picker('Effort')).toHaveValue('high')
    expect([...picker('Model').options].map((o) => o.value)).toEqual(MODELS)
    expect(MODELS).toEqual(['default', 'opus', 'sonnet', 'haiku', 'opusplan', 'opus[1m]', 'sonnet[1m]'])
    expect([...picker('Effort').options].map((o) => o.value)).toEqual(EFFORTS)
  })

  it('a change sends the value with confirmed and the token, and the header takes the server answer', async () => {
    const fake = server()
    vi.stubGlobal('fetch', fake)
    mount()
    fireEvent.change(picker('Model'), { target: { value: 'opus' } })
    await waitFor(() => expect(onChange).toHaveBeenCalledWith(expect.objectContaining({ model: 'opus' })))
    fireEvent.change(picker('Effort'), { target: { value: 'max' } })
    await waitFor(() => expect(onChange).toHaveBeenCalledWith(expect.objectContaining({ effort: 'max' })))
    expect(fake.posts).toEqual([
      { action: 'model', body: { model: 'opus', confirmed: true }, token: 'secret-token' },
      { action: 'effort', body: { effort: 'max', confirmed: true }, token: 'secret-token' },
    ])
  })

  it('a refusal puts the picker back and shows the CLI line in one line (U2)', async () => {
    const line = "Model 'opus' not found"
    const fake = server({ refuse: { model: line }, hold: 'model' })
    vi.stubGlobal('fetch', fake)
    mount()
    fireEvent.change(picker('Model'), { target: { value: 'opus' } })
    expect(picker('Model')).toHaveValue('opus') // shown while the CLI answers
    expect(picker('Model')).toBeDisabled()
    await act(async () => fake.release())
    expect(await screen.findByRole('alert')).toHaveTextContent(`Model not changed: ${line}`)
    expect(picker('Model')).toHaveValue('sonnet')
    expect(onChange).not.toHaveBeenCalled()
  })

  it('bypassPermissions asks twice and sends confirmed_bypass only after both', async () => {
    const fake = server()
    vi.stubGlobal('fetch', fake)
    mount()
    fireEvent.change(picker('Mode'), { target: { value: 'bypassPermissions' } })
    expect(fake.posts).toEqual([]) // first ask: nothing sent yet
    const yes = screen.getByRole('button', { name: 'Switch to bypassPermissions' })
    expect(yes).toBeDisabled()
    fireEvent.click(screen.getByRole('checkbox', { name: /I understand/ }))
    fireEvent.click(yes)
    await waitFor(() => expect(fake.posts).toHaveLength(1))
    expect(fake.posts[0].body).toEqual({ mode: 'bypassPermissions', confirmed: true, confirmed_bypass: true })
  })

  it('any other mode is sent at once, with no bypass confirm in the body', async () => {
    const fake = server()
    vi.stubGlobal('fetch', fake)
    mount()
    fireEvent.change(picker('Mode'), { target: { value: 'plan' } })
    await waitFor(() => expect(onChange).toHaveBeenCalledWith(expect.objectContaining({ mode: 'plan' })))
    expect(fake.posts).toEqual([{ action: 'mode', body: { mode: 'plan', confirmed: true }, token: 'secret-token' }])
  })

  it('cancelling the bypass question sends nothing and keeps the mode', () => {
    const fake = server()
    vi.stubGlobal('fetch', fake)
    mount()
    fireEvent.change(picker('Mode'), { target: { value: 'bypassPermissions' } })
    fireEvent.click(screen.getByRole('button', { name: 'Cancel' }))
    expect(picker('Mode')).toHaveValue('default')
    expect(screen.queryByRole('button', { name: 'Switch to bypassPermissions' })).toBeNull()
    expect(fake.posts).toEqual([])
  })

  it('an ended session shows the values as text, with no picker and no Context', () => {
    vi.stubGlobal('fetch', server())
    mount(view({ state: 'ended', reason: 'stopped', model: 'opus', effort: 'low' }))
    expect(screen.queryByRole('combobox', { name: 'Model' })).toBeNull()
    expect(screen.getByText('Effort', { selector: 'dt' }).nextSibling).toHaveTextContent('low')
    expect(screen.queryByRole('button', { name: 'Context' })).toBeNull()
  })
})

describe('Context (AC 5)', () => {
  it('shows the numbers of the CLI answer on request', async () => {
    vi.stubGlobal('fetch', server())
    mount()
    fireEvent.click(screen.getByRole('button', { name: 'Context' }))
    const box = await screen.findByRole('region', { name: 'Context use' })
    expect(box).toHaveTextContent('27,501 of 1,000,000 tokens (3%)')
    expect(box).toHaveTextContent('system prompt')
    expect(box).toHaveTextContent('3,543')
  })
})
