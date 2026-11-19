import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { fireEvent, render, screen, within } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import PromptCard, { APP_NO_DETAIL, APP_TOO_LONG, PLAN_NO_DETAIL, PLAN_TOO_LONG, TOO_LONG, readPrompt } from './PromptCard.jsx'

// The app session's card, as the server sends it (T103 fix round): a permission card carries `input` and `complete`.
const toolCard = (over = {}) => ({ kind: 'prompt', prompt_id: 'p1', status: 'pending', type: 'tool', tool: 'Bash', input: 'rm -rf /tmp/x', complete: true, ...over })
const questionCard = (over = {}) => ({
  kind: 'prompt', prompt_id: 'p2', status: 'pending', type: 'question', tool: 'AskUserQuestion',
  questions: [{ question: 'Which colour?', header: 'Colour', multiSelect: false, options: [{ label: 'Red', description: 'warm' }, { label: 'Blue', description: 'cold' }] }],
  ...over,
})
const planCard = (over = {}) => ({ kind: 'prompt', prompt_id: 'p3', status: 'pending', type: 'plan', tool: 'ExitPlanMode', plan: '1. do it\n2. test it', complete: true, ...over })
// The bridge's `pending_answer` (T106): the shapes the Live view reads.
const terminalPermission = (over = {}) => ({ id: 'a1', kind: 'permission', tool: 'Bash', input: 'ls -la', complete: true, ...over })
const terminalQuestion = { id: 'a2', kind: 'question', questions: [{ question: 'Pick', header: 'H', multi_select: true, options: [{ label: 'One', description: '' }, { label: 'Two', description: '' }] }] }

describe('readPrompt: one internal shape for both sources', () => {
  it('maps an app session permission card and a bridge permission to {id, kind, tool, input, complete}', () => {
    expect(readPrompt(toolCard())).toEqual({ id: 'p1', kind: 'permission', tool: 'Bash', input: 'rm -rf /tmp/x', complete: true })
    expect(readPrompt(terminalPermission())).toEqual({ id: 'a1', kind: 'permission', tool: 'Bash', input: 'ls -la', complete: true })
  })

  it('a permission item with no `complete` key is not complete, and neither is one that says false', () => {
    expect(readPrompt(terminalPermission({ complete: undefined })).complete).toBe(false)
    expect(readPrompt(toolCard({ complete: undefined })).complete).toBe(false)
    expect(readPrompt(toolCard({ complete: false })).complete).toBe(false)
    expect(readPrompt(toolCard({ complete: 'true' })).complete).toBe(false)
  })

  it('a card of the earlier T103 shape (a cut `summary`, no `input`) is shown but never complete', () => {
    const old = readPrompt({ kind: 'prompt', prompt_id: 'p9', status: 'pending', type: 'tool', tool: 'Bash', summary: 'ls' })
    expect(old).toEqual({ id: 'p9', kind: 'permission', tool: 'Bash', input: 'ls', complete: false })
  })

  it('maps questions of both sources (multiSelect and multi_select) and a plan', () => {
    expect(readPrompt(questionCard())).toEqual({
      id: 'p2', kind: 'question',
      questions: [{ question: 'Which colour?', header: 'Colour', multi: false, options: [{ label: 'Red', description: 'warm' }, { label: 'Blue', description: 'cold' }] }],
    })
    expect(readPrompt(terminalQuestion).questions[0].multi).toBe(true)
    expect(readPrompt(planCard())).toEqual({ id: 'p3', kind: 'plan', plan: '1. do it\n2. test it', complete: true })
    expect(readPrompt(planCard({ complete: undefined })).complete).toBe(false)
    expect(readPrompt(planCard({ complete: 'true' })).complete).toBe(false)
  })

  it('gives null for anything it cannot draw: no id, an unknown type, a question without options', () => {
    for (const bad of [null, undefined, 'x', 7, {}, { kind: 'permission' }, { kind: 'prompt', type: 'tool' }, { kind: 'prompt', prompt_id: 'x', type: 'other' }, { kind: 'prompt', prompt_id: 'x', type: 'question', questions: [{ question: 'q', options: [] }] }, { id: 'a', kind: 'question', questions: [] }])
      expect(readPrompt(bad)).toBeNull()
  })
})

describe('permission card', () => {
  const card = (raw, props = {}) => {
    const onAnswer = vi.fn()
    render(<PromptCard prompt={readPrompt(raw)} onAnswer={onAnswer} {...props} />)
    return onAnswer
  }

  it('Allow and Deny with a note: allow sends the decision alone, deny sends the note', () => {
    const onAnswer = card(toolCard())
    expect(screen.getByText('Bash asks for permission')).toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: 'Allow' }))
    expect(onAnswer).toHaveBeenLastCalledWith({ decision: 'allow' })
    fireEvent.change(screen.getByLabelText(/Note to Claude/), { target: { value: ' not now ' } })
    fireEvent.click(screen.getByRole('button', { name: 'Deny' }))
    expect(onAnswer).toHaveBeenLastCalledWith({ decision: 'deny', message: 'not now' })
  })

  it('offers Allow only for a complete card with a non-empty input; otherwise Deny alone, with the reason', () => {
    card(toolCard({ complete: false, input: 'x'.repeat(50) }))
    expect(screen.queryByRole('button', { name: 'Allow' })).toBeNull()
    expect(screen.getByRole('button', { name: 'Deny' })).toHaveClass('primary')
    expect(screen.getByRole('note')).toHaveTextContent(APP_TOO_LONG)
  })

  it('complete with an empty input is not allowed either, and says there is no detail', () => {
    card(toolCard({ input: '' }))
    expect(screen.queryByRole('button', { name: 'Allow' })).toBeNull()
    expect(screen.getByRole('note')).toHaveTextContent(APP_NO_DETAIL)
  })

  it('the card itself fails closed when it is handed a prompt that is not complete', () => {
    for (const prompt of [{ id: 'x', kind: 'permission', tool: 'Bash', input: 'ls' }, { id: 'x', kind: 'permission', tool: 'Bash', input: '', complete: true }, { id: 'x', kind: 'permission', tool: 'Bash', input: 7, complete: true }]) {
      const { unmount } = render(<PromptCard prompt={prompt} onAnswer={() => {}} />)
      expect(screen.queryByRole('button', { name: 'Allow' })).toBeNull()
      unmount()
    }
  })

  it('shows the whole input as plain text: markup stays text, line breaks stay, and the block is in page flow', () => {
    const input = '<img src=x onerror="window.__pwned=1">\n  echo "a   b"'
    card(toolCard({ input }))
    const block = document.querySelector('.prompt-summary')
    expect(block.tagName).toBe('PRE')
    expect(block.textContent).toBe(input)
    expect(block.querySelector('img')).toBeNull()
    expect(window.__pwned).toBeUndefined()
    const css = readFileSync(resolve(process.cwd(), 'src/appsession.css'), 'utf8')
    const rule = css.match(/\.prompt-summary\s*\{([^}]*)\}/)[1]
    expect(rule).toContain('white-space: pre-wrap')
    expect(rule).toContain('var(--font-mono)')
    expect(rule).not.toMatch(/max-height|overflow-y|overflow:\s*(auto|scroll)/)
  })

  it('a card with no onRelease is an app session card: no Answer in terminal; with one it is the terminal wording', () => {
    card(toolCard())
    expect(screen.queryByRole('button', { name: 'Answer in terminal' })).toBeNull()
    expect(screen.getByRole('region', { name: 'Permission request from the session' })).toBeInTheDocument()
  })

  it('the terminal card keeps its wording and its release button', () => {
    const onRelease = vi.fn()
    card(terminalPermission({ complete: false }), { onRelease })
    expect(screen.getByRole('region', { name: 'Permission request from a terminal session' })).toBeInTheDocument()
    expect(screen.getByRole('note')).toHaveTextContent(TOO_LONG)
    fireEvent.click(screen.getByRole('button', { name: 'Answer in terminal' }))
    expect(onRelease).toHaveBeenCalledOnce()
  })

  it('busy turns every control off and an error is announced', () => {
    card(toolCard(), { busy: true, error: 'nope' })
    for (const b of screen.getAllByRole('button')) expect(b).toBeDisabled()
    expect(screen.getByRole('alert')).toHaveTextContent('nope')
  })
})

describe('question card', () => {
  it('a single-select question takes one option by position, and Other takes the owner\'s own text', () => {
    const onAnswer = vi.fn()
    render(<PromptCard prompt={readPrompt(questionCard())} onAnswer={onAnswer} />)
    const send = screen.getByRole('button', { name: 'Send answer' })
    expect(send).toBeDisabled()
    expect(screen.getAllByRole('radio')).toHaveLength(3) // Red, Blue, Other
    fireEvent.click(screen.getByRole('radio', { name: /Blue/ }))
    fireEvent.click(send)
    expect(onAnswer).toHaveBeenLastCalledWith({ answers: [{ selected: [1] }] })
    fireEvent.click(screen.getByRole('radio', { name: /Other/ }))
    expect(send).toBeDisabled() // Other with no text is not an answer
    fireEvent.change(screen.getByLabelText('Your own answer to question 1'), { target: { value: ' Green ' } })
    fireEvent.click(send)
    expect(onAnswer).toHaveBeenLastCalledWith({ answers: [{ selected: [], other: 'Green' }] })
  })

  it('a multi-select question uses check boxes and keeps positions in order, plus Other', () => {
    const onAnswer = vi.fn()
    render(<PromptCard prompt={readPrompt(terminalQuestion)} onAnswer={onAnswer} onRelease={() => {}} />)
    expect(screen.getAllByRole('checkbox')).toHaveLength(3)
    fireEvent.click(screen.getByRole('checkbox', { name: /Two/ }))
    fireEvent.click(screen.getByRole('checkbox', { name: /One/ }))
    fireEvent.click(screen.getByRole('button', { name: 'Send answer' }))
    expect(onAnswer).toHaveBeenLastCalledWith({ answers: [{ selected: [0, 1] }] })
  })

  it('every question must be answered before Send answer is on', () => {
    const two = questionCard({ questions: [...questionCard().questions, { question: 'Size?', header: '', multiSelect: false, options: [{ label: 'S', description: '' }] }] })
    render(<PromptCard prompt={readPrompt(two)} onAnswer={() => {}} />)
    fireEvent.click(within(screen.getByRole('group', { name: 'Which colour?' })).getByRole('radio', { name: /Red/ }))
    expect(screen.getByRole('button', { name: 'Send answer' })).toBeDisabled()
    fireEvent.click(within(screen.getByRole('group', { name: 'Size?' })).getByRole('radio', { name: /S/ }))
    expect(screen.getByRole('button', { name: 'Send answer' })).toBeEnabled()
  })

  it('question and option text is plain text', () => {
    const raw = questionCard({ questions: [{ question: '<b>bold?</b>', header: '', multiSelect: false, options: [{ label: '<i>x</i>', description: '<u>y</u>' }] }] })
    render(<PromptCard prompt={readPrompt(raw)} onAnswer={() => {}} />)
    expect(screen.getByText('<b>bold?</b>')).toBeInTheDocument()
    expect(document.querySelector('b, i, u')).toBeNull()
  })
})

describe('plan card', () => {
  it('shows the plan as plain text; Approve allows, Keep planning denies with the note', () => {
    const onAnswer = vi.fn()
    render(<PromptCard prompt={readPrompt(planCard({ plan: '<h1>plan</h1>\n1. go' }))} onAnswer={onAnswer} />)
    expect(document.querySelector('.prompt-summary').textContent).toBe('<h1>plan</h1>\n1. go')
    expect(document.querySelector('h1')).toBeNull()
    fireEvent.click(screen.getByRole('button', { name: 'Approve' }))
    expect(onAnswer).toHaveBeenLastCalledWith({ decision: 'allow' })
    fireEvent.change(screen.getByLabelText(/Note to Claude/), { target: { value: 'add tests' } })
    fireEvent.click(screen.getByRole('button', { name: 'Keep planning' }))
    expect(onAnswer).toHaveBeenLastCalledWith({ decision: 'deny', message: 'add tests' })
    expect(screen.getByRole('region', { name: 'Plan from the session' })).toBeInTheDocument()
  })

  it('a plan with no text says so; Approve is not offered (nothing to approve), Keep planning still is', () => {
    const onAnswer = vi.fn()
    render(<PromptCard prompt={readPrompt(planCard({ plan: '' }))} onAnswer={onAnswer} />)
    expect(screen.getByText('No plan text came with this request.')).toBeInTheDocument()
    expect(screen.getByRole('note')).toHaveTextContent(PLAN_NO_DETAIL)
    expect(screen.queryByRole('button', { name: 'Approve' })).toBeNull()
    fireEvent.click(screen.getByRole('button', { name: 'Keep planning' }))
    expect(onAnswer).toHaveBeenLastCalledWith({ decision: 'deny' })
  })

  it('a plan cut at the server (no `complete`, or false) has no Approve and says why; Keep planning is the primary button', () => {
    for (const complete of [undefined, false, 'true']) {
      const { unmount } = render(<PromptCard prompt={readPrompt(planCard({ complete }))} onAnswer={() => {}} />)
      expect(screen.queryByRole('button', { name: 'Approve' })).toBeNull()
      expect(screen.getByRole('note')).toHaveTextContent(PLAN_TOO_LONG)
      expect(screen.getByRole('button', { name: 'Keep planning' })).toHaveClass('primary')
      unmount()
    }
  })

  it('the card itself fails closed when it is handed a plan that is not complete', () => {
    render(<PromptCard prompt={{ id: 'p3', kind: 'plan', plan: 'a plan' }} onAnswer={() => {}} />)
    expect(screen.queryByRole('button', { name: 'Approve' })).toBeNull()
  })
})

// FR-132: the CLI's own "don't ask again" suggestions, and a plan approved with edits on.
describe('allow and do not ask again (FR-132)', () => {
  const offered = [
    { index: 0, where: 'this project', summary: "don't ask again for Bash(git status:*)" },
    { index: 2, where: 'you, everywhere', summary: "don't ask again for Bash(git:*)" },
  ]

  it('names each suggestion and where its rule lands on the button, and sends only its index', () => {
    for (const raw of [toolCard({ suggestions: offered }), terminalPermission({ suggestions: offered })]) {
      const onAnswer = vi.fn()
      const { unmount } = render(<PromptCard prompt={readPrompt(raw)} onAnswer={onAnswer} />)
      fireEvent.click(screen.getByRole('button', { name: "Allow and don't ask again for Bash(git status:*) — this project" }))
      expect(onAnswer).toHaveBeenLastCalledWith({ decision: 'allow', suggestion: 0 })
      fireEvent.click(screen.getByRole('button', { name: "Allow and don't ask again for Bash(git:*) — you, everywhere" }))
      expect(onAnswer).toHaveBeenLastCalledWith({ decision: 'allow', suggestion: 2 })
      fireEvent.click(screen.getByRole('button', { name: 'Allow' }))
      expect(onAnswer).toHaveBeenLastCalledWith({ decision: 'allow' }) // the plain Allow is still there and carries no suggestion
      unmount()
    }
  })

  it('no suggestions, or ones it cannot draw, give no extra button and the shape stays as before', () => {
    expect(readPrompt(toolCard())).toEqual({ id: 'p1', kind: 'permission', tool: 'Bash', input: 'rm -rf /tmp/x', complete: true })
    for (const suggestions of [undefined, [], 'x', [{ index: -1, where: 'a', summary: 'b' }, { index: 1, where: '', summary: 'b' }, { index: 1, where: 'a' }, null]]) {
      const { unmount } = render(<PromptCard prompt={readPrompt(terminalPermission({ suggestions }))} onAnswer={() => {}} onRelease={() => {}} />)
      expect(screen.getAllByRole('button').map((b) => b.textContent)).toEqual(['Allow', 'Deny', 'Answer in terminal'])
      unmount()
    }
  })

  it('an incomplete card offers no always-allow either, and Deny sends no suggestion', () => {
    const onAnswer = vi.fn()
    render(<PromptCard prompt={readPrompt(toolCard({ complete: false, suggestions: offered }))} onAnswer={onAnswer} />)
    expect(screen.queryByRole('button', { name: /don't ask again/ })).toBeNull()
    fireEvent.click(screen.getByRole('button', { name: 'Deny' }))
    expect(onAnswer).toHaveBeenLastCalledWith({ decision: 'deny' })
  })

  it('a terminal plan is a plan card, not JSON; Approve and auto-accept edits only when the server said so', () => {
    const plan = { id: 'a5', kind: 'plan', plan: '1. do it\n2. test it', complete: true, accept_edits: true }
    expect(readPrompt(plan)).toEqual({ id: 'a5', kind: 'plan', plan: '1. do it\n2. test it', complete: true, acceptEdits: true })
    const onAnswer = vi.fn()
    const onRelease = vi.fn()
    const { unmount } = render(<PromptCard prompt={readPrompt(plan)} onAnswer={onAnswer} onRelease={onRelease} />)
    expect(screen.getByRole('region', { name: 'Plan from a terminal session' })).toBeInTheDocument()
    expect(document.querySelector('.prompt-summary').textContent).toBe('1. do it\n2. test it')
    fireEvent.click(screen.getByRole('button', { name: 'Approve and auto-accept edits (this session)' }))
    expect(onAnswer).toHaveBeenLastCalledWith({ decision: 'allow', accept_edits: true })
    fireEvent.click(screen.getByRole('button', { name: 'Approve' }))
    expect(onAnswer).toHaveBeenLastCalledWith({ decision: 'allow' })
    fireEvent.click(screen.getByRole('button', { name: 'Answer in terminal' }))
    expect(onRelease).toHaveBeenCalledOnce()
    unmount()
    render(<PromptCard prompt={readPrompt({ ...plan, accept_edits: false })} onAnswer={onAnswer} onRelease={onRelease} />)
    expect(screen.queryByRole('button', { name: /auto-accept/ })).toBeNull()
  })

  it('an app plan card offers it when the server says so, and never for a plan that cannot be approved', () => {
    const onAnswer = vi.fn()
    const { unmount } = render(<PromptCard prompt={readPrompt(planCard({ accept_edits: true }))} onAnswer={onAnswer} />)
    fireEvent.click(screen.getByRole('button', { name: 'Approve and auto-accept edits (this session)' }))
    expect(onAnswer).toHaveBeenLastCalledWith({ decision: 'allow', accept_edits: true })
    expect(screen.queryByRole('button', { name: 'Answer in terminal' })).toBeNull()
    unmount()
    render(<PromptCard prompt={readPrompt(planCard({ accept_edits: true, complete: false }))} onAnswer={onAnswer} />)
    expect(screen.queryByRole('button', { name: /auto-accept/ })).toBeNull()
  })
})
