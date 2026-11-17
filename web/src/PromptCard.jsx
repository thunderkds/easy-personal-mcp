import { useId, useState } from 'react'

// The prompt card (T107, FR-111, FR-114): what a waiting Claude Code prompt shows and how it is answered. Reusable: the Live view
// uses it for a terminal session's `pending_answer`, the app session page can use it for its own prompts. It sends nothing itself:
// `onAnswer(fields)` and `onRelease()` do. All text is React text, never HTML (FR-115).
const HIDDEN = '[hidden: looks like a secret]' // what the server sends for a question that looks like a secret; it is still answerable by position
export const OTHER_MAX = 2000
export const NOTE_MAX = 500

// A permission prompt carries the tool's whole input as text, and `complete` (T106 security review, P1). Read here and nowhere else:
// `complete: false` means the input was cut, so the card offers only Deny and Answer in terminal.
export const TOO_LONG = 'Too long to show in full \u2014 answer in the terminal, or deny'

const text = (v) => (typeof v === 'string' ? v : '')

/**
 * The prompt of a `pending_answer`, checked: `{id, kind: 'question', questions: [{question, header, multi, options: [{label, description}]}]}`
 * or `{id, kind: 'permission', tool, input, complete}`. Anything else gives null, and no card is drawn.
 */
export function readPrompt(raw) {
  if (!raw || typeof raw !== 'object' || typeof raw.id !== 'string' || raw.id === '') return null
  if (raw.kind === 'permission')
    return { id: raw.id, kind: 'permission', tool: text(raw.tool), input: text(raw.input), complete: raw.complete !== false }
  if (raw.kind !== 'question' || !Array.isArray(raw.questions) || raw.questions.length === 0) return null
  const questions = raw.questions.map((q) => ({
    question: text(q?.question),
    header: text(q?.header),
    multi: q?.multi_select === true,
    options: Array.isArray(q?.options) ? q.options.map((o) => ({ label: text(o?.label), description: text(o?.description) })) : [],
  }))
  return questions.every((q) => q.options.length > 0) ? { id: raw.id, kind: 'question', questions } : null
}

const EMPTY_CHOICE = { picked: [], otherOn: false, other: '' }
const done = (c) => c.picked.length > 0 || (c.otherOn && c.other.trim() !== '')

/** One question: its options by position, plus an Other entry that takes the owner's own text. One choice for a single-select, several for a multi-select. */
function Question({ q, n, choice, disabled, onChange }) {
  const id = useId()
  const kind = q.multi ? 'checkbox' : 'radio'
  const toggle = (i) => {
    if (!q.multi) return onChange({ picked: [i], otherOn: false, other: choice.other })
    const picked = choice.picked.includes(i) ? choice.picked.filter((p) => p !== i) : [...choice.picked, i]
    return onChange({ ...choice, picked })
  }
  const toggleOther = () => onChange(q.multi ? { ...choice, otherOn: !choice.otherOn } : { picked: [], otherOn: true, other: choice.other })
  return (
    <div role="group" aria-labelledby={`${id}-q`} className="prompt-question">
      {q.header && <span className="prompt-header">{q.header}</span>}
      <p id={`${id}-q`} className="prompt-text">
        {q.question}
      </p>
      {q.question === HIDDEN && <p className="muted">The question looks like a secret, so it is hidden here. Pick an option, or answer in the terminal.</p>}
      {q.multi && <p className="muted">Pick any that apply.</p>}
      <div className="prompt-options">
        {q.options.map((o, i) => (
          <label key={i} className="prompt-option">
            <input type={kind} name={`${id}-o`} checked={choice.picked.includes(i)} disabled={disabled} onChange={() => toggle(i)} />
            <span className="prompt-option-text">
              <strong>{o.label}</strong>
              {o.description && <span className="muted">{o.description}</span>}
            </span>
          </label>
        ))}
        <label className="prompt-option">
          <input type={kind} name={`${id}-o`} checked={choice.otherOn} disabled={disabled} onChange={toggleOther} />
          <span className="prompt-option-text">
            <strong>Other</strong>
          </span>
        </label>
        {choice.otherOn && (
          <textarea
            className="prompt-other"
            rows={2}
            maxLength={OTHER_MAX}
            aria-label={`Your own answer to question ${n}`}
            value={choice.other}
            disabled={disabled}
            onChange={(e) => onChange({ ...choice, other: e.target.value })}
          />
        )}
      </div>
    </div>
  )
}

/**
 * `prompt`: from `readPrompt`. `busy`: a request is out, every control is off. `error`: the text of the last failure.
 * `onAnswer(fields)`: `{answers: [{selected, other?}]}` (positions of the chosen options) or `{decision: 'allow' | 'deny', message?}`.
 * `onRelease()`: hand the prompt back to the terminal. Give the card `key={prompt.id}` so a new prompt starts empty.
 */
export default function PromptCard({ prompt, busy = false, error = '', onAnswer, onRelease }) {
  const noteId = useId()
  const [choices, setChoices] = useState(() => (prompt.kind === 'question' ? prompt.questions.map(() => EMPTY_CHOICE) : []))
  const [note, setNote] = useState('')
  const send = () =>
    onAnswer({
      answers: choices.map((c) => ({ selected: [...c.picked].sort((a, b) => a - b), ...(c.otherOn && c.other.trim() ? { other: c.other.trim() } : {}) })),
    })
  const decide = (decision) => onAnswer(decision === 'deny' && note.trim() ? { decision, message: note.trim() } : { decision })
  return (
    <section className="prompt-card" aria-label={prompt.kind === 'question' ? 'Question from a terminal session' : 'Permission request from a terminal session'}>
      {prompt.kind === 'question' ? (
        <>
          {prompt.questions.map((q, i) => (
            <Question key={i} q={q} n={i + 1} choice={choices[i]} disabled={busy} onChange={(c) => setChoices((all) => all.map((x, j) => (j === i ? c : x)))} />
          ))}
          <div className="actions">
            <button type="button" className="primary" disabled={busy || !choices.every(done)} onClick={send}>
              Send answer
            </button>
            <button type="button" disabled={busy} onClick={onRelease}>
              Answer in terminal
            </button>
          </div>
        </>
      ) : (
        <>
          <p className="prompt-text">
            {prompt.tool || 'A tool'} asks for permission
          </p>
          {prompt.input && <pre className="prompt-summary">{prompt.input}</pre>}
          {!prompt.complete && (
            <p role="note" className="prompt-cut">
              {TOO_LONG}
            </p>
          )}
          <label htmlFor={noteId} className="field prompt-note">
            Note to Claude (sent with Deny)
            <input id={noteId} type="text" maxLength={NOTE_MAX} value={note} disabled={busy} onChange={(e) => setNote(e.target.value)} />
          </label>
          <div className="actions">
            {prompt.complete && (
              <button type="button" className="primary" disabled={busy} onClick={() => decide('allow')}>
                Allow
              </button>
            )}
            <button type="button" className={prompt.complete ? undefined : 'primary'} disabled={busy} onClick={() => decide('deny')}>
              Deny
            </button>
            <button type="button" disabled={busy} onClick={onRelease}>
              Answer in terminal
            </button>
          </div>
        </>
      )}
      {error && (
        <p role="alert" className="field-error">
          {error}
        </p>
      )}
    </section>
  )
}
