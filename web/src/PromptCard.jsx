import { useId, useState } from 'react'

// The prompt card (T107, FR-111, FR-114; T104 adds the app session shapes and the plan card): what a waiting Claude Code prompt shows
// and how it is answered. The Live view uses it for a terminal session's `pending_answer`, the app session page for its own prompts.
// It sends nothing itself: `onAnswer(fields)` and `onRelease()` do. With no `onRelease` the card is an app session's (no terminal to
// hand back to). All text is React text, never HTML (FR-115).
const HIDDEN = '[hidden: looks like a secret]' // what the server sends for a question that looks like a secret; it is still answerable by position
export const OTHER_MAX = 2000
export const NOTE_MAX = 500

// A permission prompt carries the tool's whole input as text, and `complete` (T106 security review, P1). Allow is offered only for an
// input that is shown in full: `complete: true` and a non-empty input. A missing `complete` is not complete. Anything else offers Deny only.
export const TOO_LONG = 'Too long to show in full \u2014 answer in the terminal, or deny'
// The app session's permission card (T103 fix round) is the bridge's shape: `input` (the whole tool input as text) and `complete`. A card
// of the earlier T103 shape carries only a cut `summary` and no `complete`: it is shown, and counts as incomplete (no Allow).
export const APP_TOO_LONG = 'Too long to show in full \u2014 deny it, or stop the session and start one in a mode that does not ask'
export const APP_NO_DETAIL = 'The app shows no detail for this call \u2014 deny it, or stop the session and start one in a mode that does not ask'

const text = (v) => (typeof v === 'string' ? v : '')

const readOptions = (list) => (Array.isArray(list) ? list.map((o) => ({ label: text(o?.label), description: text(o?.description) })) : [])
function readQuestions(list, multiOf) {
  if (!Array.isArray(list) || list.length === 0) return null
  const questions = list.map((q) => ({ question: text(q?.question), header: text(q?.header), multi: multiOf(q), options: readOptions(q?.options) }))
  return questions.every((q) => q.options.length > 0) ? questions : null
}

/** The card of an app session (`{kind: 'prompt', prompt_id, type: 'tool' | 'question' | 'plan', ...}`), in the shape `readPrompt` gives. */
function readAppPrompt(raw) {
  if (typeof raw.prompt_id !== 'string' || raw.prompt_id === '') return null
  const id = raw.prompt_id
  if (raw.type === 'plan') return { id, kind: 'plan', plan: text(raw.plan) }
  if (raw.type === 'question') {
    const questions = readQuestions(raw.questions, (q) => q?.multiSelect === true)
    return questions ? { id, kind: 'question', questions } : null
  }
  if (raw.type !== 'tool') return null
  const full = typeof raw.input === 'string'
  return { id, kind: 'permission', tool: text(raw.tool), input: full ? raw.input : text(raw.summary), complete: full && raw.complete === true }
}

/**
 * The prompt of a `pending_answer` or of an app session's card, checked: `{id, kind: 'question', questions: [{question, header, multi,
 * options: [{label, description}]}]}`, `{id, kind: 'permission', tool, input, complete}` or `{id, kind: 'plan', plan}`. Anything else
 * gives null, and no card is drawn.
 */
export function readPrompt(raw) {
  if (!raw || typeof raw !== 'object') return null
  if (raw.kind === 'prompt') return readAppPrompt(raw)
  if (typeof raw.id !== 'string' || raw.id === '') return null
  if (raw.kind === 'permission') return { id: raw.id, kind: 'permission', tool: text(raw.tool), input: text(raw.input), complete: raw.complete === true }
  if (raw.kind !== 'question') return null
  const questions = readQuestions(raw.questions, (q) => q?.multi_select === true)
  return questions ? { id: raw.id, kind: 'question', questions } : null
}

const canAllow = (prompt) => prompt.complete === true && typeof prompt.input === 'string' && prompt.input !== ''

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
 * `onAnswer(fields)`: `{answers: [{selected, other?}]}` (positions of the chosen options) or `{decision: 'allow' | 'deny', message?}`
 * (a plan: Approve is allow, Keep planning is deny). `onRelease()`: hand the prompt back to the terminal; a card with none is an app
 * session's. Give the card `key={prompt.id}` so a new prompt starts empty.
 */
export default function PromptCard({ prompt, busy = false, error = '', onAnswer, onRelease }) {
  const noteId = useId()
  const [choices, setChoices] = useState(() => (prompt.kind === 'question' ? prompt.questions.map(() => EMPTY_CHOICE) : []))
  const [note, setNote] = useState('')
  const where = onRelease ? 'a terminal session' : 'the session'
  const send = () =>
    onAnswer({
      answers: choices.map((c) => ({ selected: [...c.picked].sort((a, b) => a - b), ...(c.otherOn && c.other.trim() ? { other: c.other.trim() } : {}) })),
    })
  const decide = (decision) => onAnswer(decision === 'deny' && note.trim() ? { decision, message: note.trim() } : { decision })
  const noteField = (label) => (
    <label htmlFor={noteId} className="field prompt-note">
      {label}
      <input id={noteId} type="text" maxLength={NOTE_MAX} value={note} disabled={busy} onChange={(e) => setNote(e.target.value)} />
    </label>
  )
  const release = onRelease && (
    <button type="button" disabled={busy} onClick={onRelease}>
      Answer in terminal
    </button>
  )
  const allow = prompt.kind === 'permission' && canAllow(prompt)
  const label = { question: `Question from ${where}`, permission: `Permission request from ${where}`, plan: `Plan from ${where}` }[prompt.kind]
  return (
    <section className="prompt-card" aria-label={label}>
      {prompt.kind === 'question' && (
        <>
          {prompt.questions.map((q, i) => (
            <Question key={i} q={q} n={i + 1} choice={choices[i]} disabled={busy} onChange={(c) => setChoices((all) => all.map((x, j) => (j === i ? c : x)))} />
          ))}
          <div className="actions">
            <button type="button" className="primary" disabled={busy || !choices.every(done)} onClick={send}>
              Send answer
            </button>
            {release}
          </div>
        </>
      )}
      {prompt.kind === 'permission' && (
        <>
          <p className="prompt-text">{prompt.tool || 'A tool'} asks for permission</p>
          {prompt.input && <pre className="prompt-summary">{prompt.input}</pre>}
          {!allow && (
            <p role="note" className="prompt-cut">
              {onRelease ? TOO_LONG : prompt.input ? APP_TOO_LONG : APP_NO_DETAIL}
            </p>
          )}
          {noteField('Note to Claude (sent with Deny)')}
          <div className="actions">
            {allow && (
              <button type="button" className="primary" disabled={busy} onClick={() => decide('allow')}>
                Allow
              </button>
            )}
            <button type="button" className={allow ? undefined : 'primary'} disabled={busy} onClick={() => decide('deny')}>
              Deny
            </button>
            {release}
          </div>
        </>
      )}
      {prompt.kind === 'plan' && (
        <>
          <p className="prompt-text">Claude has a plan and asks to leave plan mode</p>
          {prompt.plan ? <pre className="prompt-summary">{prompt.plan}</pre> : <p className="muted">No plan text came with this request.</p>}
          {noteField('Note to Claude (sent with Keep planning)')}
          <div className="actions">
            <button type="button" className="primary" disabled={busy} onClick={() => decide('allow')}>
              Approve
            </button>
            <button type="button" disabled={busy} onClick={() => decide('deny')}>
              Keep planning
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
