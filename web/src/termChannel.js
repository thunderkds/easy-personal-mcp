// Live and the terminals popped out of it talk over one same-origin BroadcastChannel (T135). Nothing on it is a secret or a screen byte: only
// session ids and four words. `open` (a window shows this terminal; also the answer to `who`), `closed` (that window went), `who` (Live asks which
// are open) and `dock` (the window's Dock back: Live opens the terminal in its panel again).
const NAME = 'easy-personal-term'
const KINDS = new Set(['open', 'closed', 'who', 'dock'])

/** One end of the channel, or null where the browser has none (then Live shows no marks and Dock back only closes the window). */
export function termChannel(onMessage) {
  if (typeof BroadcastChannel === 'undefined') return null
  const channel = new BroadcastChannel(NAME)
  channel.onmessage = (event) => {
    const m = event.data
    if (!m || typeof m !== 'object' || !KINDS.has(m.t)) return
    if (m.t !== 'who' && typeof m.id !== 'string') return
    onMessage(m)
  }
  return channel
}
