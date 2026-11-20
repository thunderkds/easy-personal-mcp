import { useEffect, useId, useRef, useState } from 'react'

/** The commands the block offers. Held in code, never typed by the user (FR-092). */
export const COMMANDS = [
  { label: 'Start a session', command: 'claude "/wake"' },
  { label: 'Continue the last session', command: 'claude --continue' },
  { label: 'Verify', command: 'claude "/verify"' },
]

// C0 and C1 controls, DEL, line and paragraph separators, zero-width and LRM/RLM marks, bidi embeddings and isolates, BOM:
// each can make the shown path differ from the copied bytes.
const HIDDEN = /[\u0000-\u001f\u007f-\u009f\u200b-\u200f\u2028\u2029\u202a-\u202e\u2066-\u2069\ufeff]/

/**
 * The one line to paste into a POSIX shell: `cd -- '<repo>' && <command>`.
 * The `--` ends the options, so a path that starts with `-` is not read as one.
 * The repo is in single quotes and each `'` in it is written `'\''`, so nothing in it reaches the shell as syntax.
 * Returns null for a path the line cannot carry safely: empty, or holding a hidden or control character (see HIDDEN).
 */
export function shellLine(repo, command) {
  if (typeof repo !== 'string' || repo === '' || HIDDEN.test(repo)) return null
  return `cd -- '${repo.replaceAll("'", "'\\''")}' && ${command}`
}

const refusal = (repo) => (repo === '' ? 'This repo has no path; copy is off.' : 'This repo path has a hidden or control character; copy is off.')

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
      {outcome === 'manual' && line !== null && (
        <input ref={manual} className="mono" type="text" readOnly aria-label="Command to copy" value={line} />
      )}
      <p className="muted" data-testid="copy-status" aria-live="polite">
        {line === null ? refusal(repo) : outcome === 'copied' ? 'Copied' : outcome === 'manual' ? 'Copy it with Ctrl+C.' : ''}
      </p>
    </div>
  )
}

const PLAIN = /^[A-Za-z0-9_./:@%+=,-]+$/

/**
 * T132: the line that signs in to a new account folder: `CLAUDE_CONFIG_DIR=<folder> claude`.
 * The folder is written as is when it holds only plain characters, else in single quotes (each `'` as `'\''`).
 * Returns null for a folder the line cannot carry safely: empty, or holding a hidden or control character (see HIDDEN).
 */
export function signInLine(folder) {
  if (typeof folder !== 'string' || folder === '' || HIDDEN.test(folder)) return null
  return `CLAUDE_CONFIG_DIR=${PLAIN.test(folder) ? folder : `'${folder.replaceAll("'", "'\\''")}'`} claude`
}

/** The sign-in line of a new account with a Copy button, shown the way CopyCommand shows its line. It runs nothing and sends nothing. */
export function SignInCommand({ folder }) {
  const [outcome, setOutcome] = useState(null) // null | 'copied' | 'manual'
  const manual = useRef(null)
  useEffect(() => {
    if (outcome === 'manual') {
      manual.current?.focus()
      manual.current?.select()
    }
  }, [outcome])
  const line = signInLine(folder)
  const copy = async () => {
    try {
      await navigator.clipboard.writeText(line)
      setOutcome('copied')
    } catch {
      setOutcome('manual')
    }
  }
  return (
    <div className="copy-command" data-testid="signin-command">
      <p className="copy-command-title">Sign in to the new account</p>
      {line === null ? (
        <p className="muted">This folder path has a hidden or control character; copy is off.</p>
      ) : (
        <>
          <p className="muted">Run this in a terminal, then type /login in Claude.</p>
          <div className="copy-command-row">
            <code>{line}</code>
            <button type="button" onClick={copy}>
              Copy
            </button>
          </div>
          {outcome === 'manual' && <input ref={manual} className="mono" type="text" readOnly aria-label="Command to copy" value={line} />}
          <p className="muted" aria-live="polite">
            {outcome === 'copied' ? 'Copied' : outcome === 'manual' ? 'Copy it with Ctrl+C.' : ''}
          </p>
        </>
      )}
    </div>
  )
}
