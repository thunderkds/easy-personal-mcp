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

/** The address of a terminal's own window (T135): the page alone with `?term=<id>`; the token rides in the hash, as on the dashboard's own address. */
export const popOutUrl = (id) => `/?term=${encodeURIComponent(id)}#${readToken()}`

/**
 * `term`: the session as `GET /api/term-sessions` lists it (`id`, `repo`, `account`, `state`, `code`). `onBack()` closes the panel. `onChanged()` asks Live to
 * read the list again (the session ended, or Stop was sent). `autoFocus`: move the focus to the heading when the panel opens. `onPoppedOut()`: the
 * terminal opened in its own window (T135), so Live closes this panel. `popped`: this panel is that window, so it has no Back and no Pop out.
 */
export default function TermPanel({ term, onBack, onChanged, onPoppedOut, autoFocus = false, popped = false }) {
  const { id } = term
  const panel = useRef(null)
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
  const [popError, setPopError] = useState('')
  const ended = exit !== null || isExited(term)
  const code = exit ? exit.code : term.code
  const endedRef = useRef(ended)
  endedRef.current = ended
  const onChangedRef = useRef(onChanged)
  onChangedRef.current = onChanged

  // In Live the screen is sized to the window (T135): bring the whole panel into view when it opens, under the sticky header.
  useEffect(() => {
    if (!popped) panel.current?.scrollIntoView?.({ block: 'start' })
  }, [popped])
  useEffect(() => {
    if (autoFocus) heading.current?.focus({ preventScroll: !popped })
  }, [autoFocus, popped])

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
    if (popped) xterm.focus() // its own window is only this terminal: the keys go to it at once
    const typed = xterm.onData((text) => send(bytesOf(text)))
    const binary = xterm.onBinary?.((text) => send(Uint8Array.from(text, (c) => c.charCodeAt(0)))) // a mouse report: one byte per character
    let timer = null
    let forceNext = false // a focus asked for a forced send: a plain refit in the same 150 ms keeps it
    // `force`: tell the size even when it is unchanged. With two pages attached the last resize wins (question 47), so the page whose screen
    // takes the focus sends its own size and the program fits the page being typed in (T135).
    const refit = (force = false) => {
      forceNext = forceNext || force === true
      clearTimeout(timer)
      timer = setTimeout(() => {
        const forced = forceNext
        forceNext = false
        fit.fit()
        const ws = socket.current
        if (!ws || ws.readyState !== OPEN || (!forced && last.current?.cols === xterm.cols && last.current?.rows === xterm.rows)) return
        last.current = { cols: xterm.cols, rows: xterm.rows }
        ws.send(JSON.stringify({ t: 'resize', cols: xterm.cols, rows: xterm.rows }))
      }, RESIZE_MS)
    }
    const observer = typeof ResizeObserver === 'undefined' ? null : new ResizeObserver(() => refit())
    observer?.observe(box.current)
    document.fonts?.ready?.then(() => refit()) // the font changes the cell size once it has loaded
    const focused = () => refit(true)
    xterm.textarea?.addEventListener('focus', focused)
    return () => {
      clearTimeout(timer)
      observer?.disconnect()
      xterm.textarea?.removeEventListener('focus', focused)
      typed.dispose()
      binary?.dispose()
      xterm.dispose()
      screen.current = null
    }
  }, [id]) // `popped` never changes for a mounted panel

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
  const popOut = () => {
    setPopError('')
    const win = window.open(popOutUrl(id), '_blank')
    if (win) win.opener = null
    // The desktop app window denies the webview's own new window and opens the pop-out as an app window itself (`window.rs`), so there
    // `window.open` answers null with the window opening; the app marks its windows with this flag. Anywhere else null is a pop-up blocker.
    else if (window.__EASY_PERSONAL_APP__ !== true) {
      setPopError('The browser blocked the new window. Allow pop-ups for this page, then try Pop out again.')
      return
    }
    onPoppedOut?.()
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
    <section ref={panel} id={popped ? 'term-window' : 'live-panel'} className="card session-panel term-panel" data-popped={popped ? 'true' : undefined} aria-label={`Terminal session in ${project}`}>
      <div className="term-head">
        {!popped && (
          <button type="button" className="session-back" onClick={onBack}>
            Back to sessions
          </button>
        )}
        <div className="term-name">
          <h2 className="session-title" ref={heading} tabIndex={-1}>
            {`${project} · ${String(id).slice(0, 8)}`}
          </h2>
          <p className="term-meta">
            <span>{accountLabel(term.account)}</span>
            <span className="mono" title={term.repo}>
              {term.repo}
            </span>
          </p>
        </div>
        <p role="status" className="session-state" data-state={ended || link === 'lost' ? 'ended' : 'working'}>
          {status}
        </p>
        <div className="term-actions">
          {!popped && (
            <button type="button" onClick={popOut} title="Open this terminal in its own window">
              Pop out
            </button>
          )}
          {!ended && (
            <button type="button" className="danger" disabled={stopBusy || stopSent} onClick={stop}>
              Stop
            </button>
          )}
        </div>
      </div>
      {pages > 1 && <p className="muted term-note">{`${pages} pages attached`}</p>}
      {dropped && <p className="muted term-note">Some input was dropped: too much was typed too fast.</p>}
      {link === 'lost' && !ended && (
        <div role="alert" className="prompt-cut">
          <p>Disconnected from the terminal. The session keeps running; reconnect to see it again.</p>
          <button type="button" onClick={reconnect}>
            Reconnect
          </button>
        </div>
      )}
      {(error || popError) && (
        <p role="alert" className="field-error">
          {error || popError}
        </p>
      )}
      <div role="group" aria-label="Terminal" className="term-box" ref={box} />
      <div role="group" aria-label="Terminal keys" className="term-keys">
        {KEYS.map(([name, label, sequence]) => (
          <button key={name} type="button" aria-label={name} disabled={!live} onClick={() => press(sequence)}>
            {label}
          </button>
        ))}
      </div>
    </section>
  )
}
