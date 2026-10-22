// Guarded API client: the per-launch token comes from the URL fragment (#<token>) and is
// sent as X-Dashboard-Token. Only same-origin /api/ paths are requested.
function readToken() {
  const token = window.location.hash.replace(/^#/, '')
  return token
}

export async function getJson(path) {
  const response = await fetch(path, { headers: { 'X-Dashboard-Token': readToken() } })
  if (!response.ok) throw new Error(`${path} failed with status ${response.status}`)
  return response.json()
}

/** Guarded JSON write (POST/DELETE). A non-2xx answer throws with the server's `error` text. */
export async function sendJson(method, path, body) {
  const response = await fetch(path, {
    method,
    headers: { 'X-Dashboard-Token': readToken(), 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  })
  const data = await response.json().catch(() => ({}))
  if (!response.ok) throw new Error(data.error || `${path} failed with status ${response.status}`)
  return data
}
