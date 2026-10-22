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
