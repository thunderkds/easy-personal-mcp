import { FitAddon } from '@xterm/addon-fit'
import { Terminal } from '@xterm/xterm'
import '@xterm/xterm/css/xterm.css'
import { useEffect, useRef, useState } from 'react'
import { accountLabel, endedWords, isExited, readToken, stopTermSession } from './api.js'
import { why } from './Composer.jsx'
import { folderName } from './SessionPanel.jsx'

// The terminal panel of the Live view (T134, FR-136..FR-138, DDR-0009): the owner's own `claude` in a PTY, drawn with xterm. One WebSocket per open panel
// carries the bytes (T133's contract: the first frame is `{t:'auth', token, cols, rows}`, the token is never in the URL; binary frames in are the
// keys typed, binary frames out are the screen; text frames are `resize`, `exit`, `pages`, `notice`). Every attached page can type; the last resize
// wins. The screen is shown here and never kept: nothing goes to storage, a URL or the console, and the server drops its copy when the session ended
// and the last page left. Esc is a key of the terminal, so this panel does not close on Esc as the Chat panel does.
const KEYS = [
  ['Esc', 'Esc', '\x1b'],
  ['Tab', 'Tab', '\t'],
  ['Ctrl-C', 'Ctrl-C', '\x03'],
  ['Enter', 'Enter', '\r'],
  ['Up', '↑', '\x1b[A'],
  ['Down', '↓', '\x1b[B'],
  ['Left', '←', '\x1b[D'],
  ['Right', '→', '\x1b[C'],
]
const RESIZE_MS = 150
const OPEN = 1
const encoder = new TextEncoder()
const bytesOf = (text) => encoder.encode(text)
const css = (name) => getComputedStyle(document.documentElement).getPropertyValue(name).trim()

const wsUrl = (id) => `${window.location.protocol === 'https:' ? 'wss:' : 'ws:'}//${window.location.host}/api/term/${encodeURIComponent(id)}`

/**
 * `term`: the session as `GET /api/term-sessions` lists it (`id`, `repo`, `account`, `state`, `code`). `onBack()` closes the panel. `onChanged()` asks Live to
 * read the list again (the session ended, or Stop was sent). `autoFocus`: move the focus to the heading when the panel opens.
 */
export default function TermPanel({ term, onBack, onChanged, autoFocus = false }) {
  const { id } = term
  const box = useRef(null)
  const heading = useRef(null)
  const screen = useRef(null) // the xterm Terminal
  const socket = useRef(null)
  const last = useRef(null) // the size the server was last told
  const [link, setLink] = useState('connecting') // connecting | open | lost
  const [attempt, setAttempt] = useState(0)
  const [exit, setExit] = useState(null) // {code} once an exit frame came
  const [pages, setPages] = useState(1)
  const [dropped, setDropped] = useState(false)
  const [stopSent, setStopSent] = useState(false)
  const [stopBusy, setStopBusy] = useState(false)
  const [error, setError] = useState('')
  const ended = exit !== null || isExited(term)
  const code = exit ? exit.code : term.code
  const endedRef = useRef(ended)
  endedRef.current = ended
  const onChangedRef = useRef(onChanged)
  onChangedRef.current = onChanged

  useEffect(() => {
    if (autoFocus) heading.current?.focus()
  }, [autoFocus])

  // The screen: one xterm per panel, fitted to its box; a size change is told to the server after it settles.
  useEffect(() => {
    const theme = {}
    for (const [key, name] of [['background', '--color-bg'], ['foreground', '--color-text'], ['cursor', '--color-accent']]) if (css(name)) theme[key] = css(name)
    const xterm = new Terminal({ cursorBlink: true, fontSize: 13, scrollback: 5000, fontFamily: css('--font-mono') || 'monospace', theme })
    const fit = new FitAddon()
    xterm.loadAddon(fit)
    xterm.open(box.current)
    fit.fit()
    screen.current = xterm
    const typed = xterm.onData((text) => send(bytesOf(text)))
    const binary = xterm.onBinary?.((text) => send(Uint8Array.from(text, (c) => c.charCodeAt(0)))) // a mouse report: one byte per character
    let timer = null
    const refit = () => {
      clearTimeout(timer)
      timer = setTimeout(() => {
        fit.fit()
        const ws = socket.current
        if (!ws || ws.readyState !== OPEN || (last.current?.cols === xterm.cols && last.current?.rows === xterm.rows)) return
        last.current = { cols: xterm.cols, rows: xterm.rows }
        ws.send(JSON.stringify({ t: 'resize', cols: xterm.cols, rows: xterm.rows }))
      }, RESIZE_MS)
    }
    const observer = typeof ResizeObserver === 'undefined' ? null : new ResizeObserver(refit)
    observer?.observe(box.current)
    document.fonts?.ready?.then(refit) // the font changes the cell size once it has loaded
    return () => {
      clearTimeout(timer)
      observer?.disconnect()
      typed.dispose()
      binary?.dispose()
      xterm.dispose()
      screen.current = null
    }
  }, [id])

  useEffect(() => {
    if (screen.current) screen.current.options.disableStdin = ended
  }, [ended])

  // The connection: re-run by Reconnect (a new attach replays the screen).
  useEffect(() => {
    const xterm = screen.current
    const ws = new WebSocket(wsUrl(id))
    ws.binaryType = 'arraybuffer'
    socket.current = ws
    ws.onopen = () => {
      last.current = { cols: xterm.cols, rows: xterm.rows }
      ws.send(JSON.stringify({ t: 'auth', token: readToken(), cols: xterm.cols, rows: xterm.rows }))
      setLink('open')
    }
    ws.onmessage = (event) => {
      if (typeof event.data !== 'string') {
        if (event.data?.byteLength >= 0) xterm.write(new Uint8Array(event.data)) // an ArrayBuffer (`binaryType`)
        return
      }
      let frame
      try {
        frame = JSON.parse(event.data)
      } catch {
        return // a frame that is not JSON is skipped
      }
      if (!frame || typeof frame !== 'object') return
      if (frame.t === 'exit') {
        endedRef.current = true
        setExit({ code: Number.isInteger(frame.code) ? frame.code : null })
        onChangedRef.current?.()
      } else if (frame.t === 'pages' && Number.isInteger(frame.n)) setPages(frame.n)
      else if (frame.t === 'notice') setDropped(true)
    }
    ws.onclose = () => {
      if (!endedRef.current) setLink('lost') // after an exit frame the close is the normal end
    }
    return () => {
      ws.onopen = ws.onmessage = ws.onclose = null
      ws.close()
      if (socket.current === ws) socket.current = null
    }
  }, [id, attempt])

  function send(data) {
    const ws = socket.current
    if (ws && ws.readyState === OPEN && !endedRef.current) ws.send(data)
  }
  const reconnect = () => {
    screen.current?.reset() // the replay draws the whole screen again
    setLink('connecting')
    setAttempt((n) => n + 1)
  }
  const press = (sequence) => {
    send(bytesOf(sequence))
    screen.current?.focus()
  }
  const stop = async () => {
    if (stopBusy) return
    setStopBusy(true)
    setError('')
    try {
      await stopTermSession(id)
      setStopSent(true)
      onChanged?.()
    } catch (e) {
      setError(why(e, 'Could not stop the session. Try again.'))
    } finally {
      setStopBusy(false)
    }
  }

  const project = folderName(term.repo) || 'terminal'
  const live = link === 'open' && !ended
  const status = ended ? endedWords(code) : stopSent ? 'Stopping' : link === 'connecting' ? 'Connecting' : link === 'lost' ? 'Not attached' : 'Running'
  return (
    <section id="live-panel" className="card session-panel term-panel" aria-label={`Terminal session in ${project}`}>
      <div className="session-head">
        <button type="button" className="session-back" onClick={onBack}>
          Back to sessions
        </button>
        <h2 className="session-title" ref={heading} tabIndex={-1}>
          {`${project} · ${String(id).slice(0, 8)}`}
        </h2>
      </div>
      <dl className="session-facts">
        <dt>Project</dt>
        <dd>{project}</dd>
        <dt>Account</dt>
        <dd>{accountLabel(term.account)}</dd>
        <dt>Folder</dt>
        <dd className="mono">{term.repo}</dd>
      </dl>
      <p role="status" className="session-state" data-state={ended || link === 'lost' ? 'ended' : 'working'}>
        {status}
      </p>
      {pages > 1 && <p className="muted">{`${pages} pages attached`}</p>}
      {dropped && <p className="muted">Some input was dropped: too much was typed too fast.</p>}
      {link === 'lost' && !ended && (
        <div role="alert" className="prompt-cut">
          <p>Disconnected from the terminal. The session keeps running; reconnect to see it again.</p>
          <button type="button" onClick={reconnect}>
            Reconnect
          </button>
        </div>
      )}
      <div role="group" aria-label="Terminal" className="term-box" ref={box} />
      <div role="group" aria-label="Terminal keys" className="term-keys">
        {KEYS.map(([name, label, sequence]) => (
          <button key={name} type="button" aria-label={name} disabled={!live} onClick={() => press(sequence)}>
            {label}
          </button>
        ))}
      </div>
      {!ended && (
        <div className="actions">
          <button type="button" className="danger" disabled={stopBusy || stopSent} onClick={stop}>
            Stop
          </button>
        </div>
      )}
      {error && (
        <p role="alert" className="field-error">
          {error}
        </p>
      )}
    </section>
  )
}
