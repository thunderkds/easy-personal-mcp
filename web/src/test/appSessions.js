// T104: fixtures and a hand-driven server for the app session page, as `rust/crates/app/src/app_session_routes.rs` answers it.
import { accountsByAccount, agentRunOn } from './fixtures.js'

export const workspaces = {
  workspaces: [
    { id: 1, name: 'orphan', is_orphan: true, repos: ['/orphan/repo'] },
    { id: 2, name: 'company', is_orphan: false, repos: ['/work/company', '/work/company-web'] },
    { id: 3, name: 'side', is_orphan: false, repos: [] },
    { id: 4, name: 'home', is_orphan: false, repos: ['/home/o/notes'] },
  ],
}

/** One session as `GET /api/app-sessions/{id}` answers it. */
export const appSession = (over = {}) => ({
  id: 'a1', state: 'idle', reason: null, session_id: 'sess-1111', account: { id: '2', label: 'Claude personal' }, model: 'default', mode: 'default',
  folder: '/work/company', worktree: false, branch: null, changed_files: [], items: [], ...over,
})
/** One terminal session as `GET /api/term-sessions` lists it (T133's contract; `session_id` is the Claude id once its process file exists). */
export const termSession = (over = {}) => ({
  id: 't1', repo: '/work/company', account: { id: '2', label: 'Claude personal' }, started: '2026-11-22T09:00:00Z', state: 'running', code: null, pages: 0, session_id: null, ...over,
})
export const user = (text) => ({ kind: 'user', text })
export const assistant = (text) => ({ kind: 'assistant', text })
export const tool = (name, label = null, outcome = 'ok') => ({ kind: 'tool', name, label, outcome })
export const toolPrompt = (over = {}) => ({ kind: 'prompt', prompt_id: 'p1', status: 'pending', type: 'tool', tool: 'Bash', input: 'ls -la', complete: true, ...over })
export const questionPrompt = (over = {}) => ({
  kind: 'prompt', prompt_id: 'p2', status: 'pending', type: 'question', tool: 'AskUserQuestion',
  questions: [{ question: 'Which colour?', header: 'Colour', multiSelect: false, options: [{ label: 'Red', description: 'warm' }, { label: 'Blue', description: 'cold' }] }],
  ...over,
})
export const planPrompt = (over = {}) => ({ kind: 'prompt', prompt_id: 'p3', status: 'pending', type: 'plan', tool: 'ExitPlanMode', plan: '1. do it', complete: true, ...over })

const reply = (status, body) => ({ ok: status < 400, status, json: async () => body })

/**
 * A server for the page: `state.sessions` is the list the server holds; a test changes it between polls. `routes` maps `'METHOD path'` to
 * `(body, call) => body | {status, body}`; a route it does not name answers like the server for an unknown id. Every call is in `calls`.
 */
export function appServer({ sessions = [], max = 4, agentRun = agentRunOn(), routes = {}, list = undefined, terms = undefined } = {}) {
  const state = { sessions, max, agentRun, list, terms } // `terms`: the terminal sessions (T134); undefined = a server with no such route (404)
  const calls = []
  const fn = async (url, init = {}) => {
    const method = init.method ?? 'GET'
    const path = new URL(String(url), 'http://127.0.0.1').pathname
    const body = init.body ? JSON.parse(init.body) : undefined
    calls.push({ method, path, url: String(url), body, init })
    const named = routes[`${method} ${path}`]
    if (named) {
      const answer = named(body, state)
      return answer && typeof answer.status === 'number' ? reply(answer.status, answer.body) : reply(200, answer)
    }
    if (method === 'GET' && path === '/api/app-sessions') return state.fail ? reply(500, {}) : state.list === null ? reply(404, {}) : reply(200, { sessions: state.sessions, max: state.max })
    if (method === 'GET' && path === '/api/term-sessions') return state.terms === undefined ? reply(404, {}) : reply(200, { sessions: state.terms })
    if (method === 'GET' && path === '/api/agent-run') return state.agentRun === null ? reply(404, {}) : reply(200, state.agentRun)
    if (method === 'GET' && path === '/api/workspaces') return reply(200, workspaces)
    if (method === 'GET' && path === '/api/accounts') return reply(200, accountsByAccount)
    const m = path.match(/^\/api\/app-sessions\/([^/]+)(?:\/(\w+))?$/)
    if (method === 'GET' && m && !m[2]) {
      const found = state.sessions.find((s) => s.id === decodeURIComponent(m[1]))
      return found ? reply(200, found) : reply(404, { error: 'no such session' })
    }
    return reply(404, { error: 'no such route' })
  }
  fn.calls = calls
  fn.state = state
  fn.posts = (suffix) => calls.filter((c) => c.method === 'POST' && c.path.endsWith(suffix))
  return fn
}
