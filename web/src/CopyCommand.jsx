import { useEffect, useId, useRef, useState } from 'react'

/** The commands the block offers. Held in code, never typed by the user (FR-092). */
export const COMMANDS = [
  { label: 'Start a session', command: 'claude "/wake"' },
  { label: 'Continue the last session', command: 'claude --continue' },
  { label: 'Verify', command: 'claude "/verify"' },
]

const CONTROL = /[\u0000-\u001f\u007f]/

/**
 * The one line to paste into a POSIX shell: `cd '<repo>' && <command>`.
 * The repo is in single quotes and each `'` in it is written `'\''`, so nothing in it reaches the shell as syntax.
 * Returns null for a path the line cannot carry safely: empty, or holding a C0 control character or U+007F.
 */
export function shellLine(repo, command) {
  if (typeof repo !== 'string' || repo === '' || CONTROL.test(repo)) return null
  return `cd '${repo.replaceAll("'", "'\\''")}' && ${command}`
}

/** A small block on a workspace card: pick a command (and a repo), then copy one line. It runs nothing and sends nothing. */
export default function CopyCommand({ repos }) {
  const id = useId()
  const [commandIndex, setCommandIndex] = useState(0)
  const [repoIndex, setRepoIndex] = useState(0)
  const [outcome, setOutcome] = useState(null) // null | 'copied' | 'manual'
  const manual = useRef(null)
  useEffect(() => {
    if (outcome === 'manual') {
      manual.current?.focus()
      manual.current?.select()
    }
  }, [outcome])
  if (repos.length === 0) return null

  const repo = repos[repoIndex] ?? repos[0]
  const line = shellLine(repo, COMMANDS[commandIndex].command)
  const choose = (set) => (e) => {
    set(Number(e.target.value))
    setOutcome(null)
  }
  const copy = async () => {
    try {
      await navigator.clipboard.writeText(line)
      setOutcome('copied')
    } catch {
      setOutcome('manual')
    }
  }

  return (
    <div className="copy-command" data-testid="copy-command">
      <p className="copy-command-title">Copy a Claude command</p>
      <div className="copy-command-row">
        <label htmlFor={`${id}-command`}>
          <span>Command</span>
          <select id={`${id}-command`} value={commandIndex} onChange={choose(setCommandIndex)}>
            {COMMANDS.map((c, i) => (
              <option key={c.label} value={i}>
                {c.label}
              </option>
            ))}
          </select>
        </label>
        {repos.length > 1 && (
          <label htmlFor={`${id}-repo`}>
            <span>Repo</span>
            <select id={`${id}-repo`} value={repoIndex} onChange={choose(setRepoIndex)}>
              {repos.map((r, i) => (
                <option key={i} value={i}>
                  {r}
                </option>
              ))}
            </select>
          </label>
        )}
        {line !== null && (
          <button type="button" onClick={copy}>
            Copy
          </button>
        )}
      </div>
      {line === null && <p className="muted">This repo path has a control character; copy is off.</p>}
      {outcome === 'manual' && line !== null && (
        <input ref={manual} className="mono" type="text" readOnly aria-label="Command to copy" value={line} />
      )}
      <p className="muted" data-testid="copy-status" aria-live="polite">
        {outcome === 'copied' ? 'Copied' : outcome === 'manual' ? 'Copy it with Ctrl+C.' : ''}
      </p>
    </div>
  )
}
