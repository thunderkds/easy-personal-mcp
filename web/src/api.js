// Guarded API client: the per-launch token comes from the URL fragment (#<token>) and is
// sent as X-Dashboard-Token. Only same-origin /api/ paths are requested.
export function readToken() {
  const token = window.location.hash.replace(/^#/, '')
  return token
}

/** An Error that also carries the HTTP `status`, so a caller can tell 404 (no such route) from the rest without reading the message. */
const failure = (message, status) => Object.assign(new Error(message), { status })

export async function getJson(path) {
  const response = await fetch(path, { headers: { 'X-Dashboard-Token': readToken() } })
  if (!response.ok) throw failure(`${path} failed with status ${response.status}`, response.status)
  return response.json()
}

/** Guarded GET of a file's bytes (FR-090): the token goes in the header only. Returns {bytes, type}; a non-2xx answer throws. */
export async function getBytes(path) {
  const response = await fetch(path, { headers: { 'X-Dashboard-Token': readToken() } })
  if (!response.ok) throw new Error(`${path} failed with status ${response.status}`)
  return { bytes: await response.arrayBuffer(), type: response.headers.get('Content-Type') || '' }
}

/** Guarded JSON write (POST/DELETE). A non-2xx answer throws with the server's `error` text. */
export async function sendJson(method, path, body) {
  const response = await fetch(path, {
    method,
    headers: { 'X-Dashboard-Token': readToken(), 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  })
  const data = await response.json().catch(() => ({}))
  if (!response.ok) throw failure(data.error || `${path} failed with status ${response.status}`, response.status)
  return data
}

/** Saved investigate reports (FR-078): the list, newest first, and one report with its before / now per rule. */
export const getReports = () => getJson('/api/reports')
export const getReport = (id) => getJson(`/api/reports/${id}`)

/** Agent run (FR-103). The state, or null when the server has no such route (404: the Python server). */
export const getAgentRun = () =>
  getJson('/api/agent-run').catch((error) => {
    if (error.status === 404) return null
    throw error
  })
/** Choose the agent run account (the caller has shown the note and been confirmed), or turn the feature off with null. Answers the new state. */
export const setAgentRunAccount = (id) =>
  sendJson('PUT', '/api/agent-run/setting', id === null ? { account: null } : { account: String(id), confirmed: true })
/** Start the one job for the workspace (none = all). Answers the state, running. */
export const startAgentRun = (workspace) => sendJson('POST', '/api/agent-run', workspace ? { job: 'investigate', workspace } : { job: 'investigate' })
export const stopAgentRun = () => sendJson('POST', '/api/agent-run/stop', {})

/** Answer bridge (FR-113). The accounts with their bridge state and the exact entry text, or null when the server has no such route (404: the Python server). */
export const getHooks = () =>
  getJson('/api/hooks').catch((error) => {
    if (error.status === 404) return null
    throw error
  })
/** Turn the bridge on or off for one account (the caller has shown the entries and been confirmed). Answers that account's row. */
export const setHooks = (account, on) => sendJson('PUT', '/api/hooks', { account: String(account), on, confirmed: true })

/** Answer a prompt of a terminal session (FR-114): `fields` is `{answers: [{selected, other}]}` for a question or `{decision, message}` for a permission. */
export const answerPrompt = (id, fields) => sendJson('POST', '/api/answer', { ...fields, id, confirmed: true })
/** Hand a prompt back to the terminal. */
export const releasePrompt = (id) => sendJson('POST', '/api/answer/release', { id, confirmed: true })
