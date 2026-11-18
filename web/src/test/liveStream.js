// T118: a hand-driven EventSource for the Live view tests, and the snapshot rows around it.
import { act } from '@testing-library/react'

export class FakeEventSource {
  static instances = []
  constructor(url) {
    this.url = url
    this.readyState = 0
    this.listeners = {}
    FakeEventSource.instances.push(this)
  }
  addEventListener(type, fn) {
    ;(this.listeners[type] ??= []).push(fn)
  }
  close() {
    this.readyState = 2
  }
  emit(type, data) {
    act(() => (this.listeners[type] ?? []).forEach((fn) => fn({ type, data: data === undefined ? undefined : JSON.stringify(data), lastEventId: '' })))
  }
}
export const stream = () => FakeEventSource.instances.at(-1)

/** One row of the `/api/live` snapshot. */
export const liveRow = (id, over = {}) => ({ session_id: id, account: 'Claude work', project: 'alpha', last_ts: new Date().toISOString(), tool_calls: 0, tokens_in: 0, tokens_out: 0, cost_usd: 0, ...over })
export const snapshot = (sessions) => ({ active_window_s: 300, last_id: 0, sessions })

/** A transcript answer (T115): `{entries, next, complete}`. */
export const page = (entries, over = {}) => ({ entries, next: 100, complete: true, ...over })
export const entryUser = (text, over = {}) => ({ kind: 'user', text, ...over })
export const entryAssistant = (text, over = {}) => ({ kind: 'assistant', text, ...over })
export const entryReply = (text) => ({ kind: 'reply', text })
export const entryTool = (id, over = {}) => ({ kind: 'tool', id, name: 'Bash', label: 'ls', input: 'ls -la', result: 'total 0', is_error: false, ...over })
export const entrySkipped = (count) => ({ kind: 'skipped', count })

const reply = (status, body) => ({ ok: status < 400, status, json: async () => body })

/**
 * A fetch for the terminal side of the panel: the transcript route (`state.pages`: answers in order, the last repeats; or `state.transcript`
 * as a function of the query), the Stop route and the queue route. Every call is in `calls`; anything else is a 404.
 */
export function liveServer({ pages = [page([])], stop = () => ({}), queue = () => ({}), transcriptStatus = null } = {}) {
  const state = { pages, calls: [], stop, queue, transcriptStatus, reads: 0 }
  const fn = async (url, init = {}) => {
    const u = new URL(String(url), 'http://127.0.0.1')
    const method = init.method ?? 'GET'
    const body = init.body ? JSON.parse(init.body) : undefined
    state.calls.push({ method, path: u.pathname, search: u.search, url: String(url), init, body })
    if (method === 'GET' && /^\/api\/live\/sessions\/[^/]+\/transcript$/.test(u.pathname)) {
      if (state.transcriptStatus) return reply(state.transcriptStatus.status, state.transcriptStatus.body)
      const answer = typeof state.pages === 'function' ? state.pages(u.searchParams) : state.pages[Math.min(state.reads, state.pages.length - 1)]
      state.reads += 1
      return reply(200, answer)
    }
    if (method === 'POST' && /^\/api\/live\/sessions\/[^/]+\/stop$/.test(u.pathname)) return answer(state.stop(body))
    if (method === 'POST' && u.pathname === '/api/reply/queue') return answer(state.queue(body))
    return reply(404, { error: 'no such route' })
  }
  const answer = (a) => (a && typeof a.status === 'number' ? reply(a.status, a.body) : reply(200, a))
  fn.state = state
  fn.calls = state.calls
  fn.posts = (suffix) => state.calls.filter((c) => c.method === 'POST' && c.path.endsWith(suffix))
  fn.reads = () => state.calls.filter((c) => c.method === 'GET' && c.path.endsWith('/transcript'))
  return fn
}
