import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { renderToStaticMarkup } from 'react-dom/server'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import MemoryView, { MAX_DRAWN, RingsGraph, layout } from './MemoryView.jsx'

const node = (id, kind, ring, name, extra = {}) => ({
  id, kind, ring, name, stem: name, description: '', path: kind === 'dangling' ? null : `memory/${name}.md`,
  account: kind === 'claude' ? 'Claude work' : null, abstained: false, links_in: 0, links_out: 0, ...extra,
})
const nodes = [
  node('kit:memory/MEMORY.md', 'kit', 0, 'memory-index', { path: 'memory/MEMORY.md', description: 'Hot index', links_out: 2 }),
  node('kit:memory/decisions.md', 'kit', 1, 'decisions', { description: 'Why we chose things', links_in: 3, links_out: 2 }),
  node('kit:memory/glossary.md', 'kit', 1, 'glossary', { links_in: 1, links_out: 1 }),
  node('kit:memory/learning-records/LR-0001.md', 'kit', 2, 'LR-0001', { path: 'memory/learning-records/LR-0001.md' }),
  node('claude:1:memory/feedback.md', 'claude', 2, 'feedback-tone', { description: 'How the owner likes answers' }),
  node('dangling:ghost', 'dangling', 3, 'ghost', { links_in: 1 }),
]
const edges = [
  { source: 'kit:memory/MEMORY.md', target: 'kit:memory/decisions.md' },
  { source: 'kit:memory/decisions.md', target: 'kit:memory/glossary.md' },
  { source: 'kit:memory/decisions.md', target: 'dangling:ghost' },
]
const graph = (over = {}) => ({
  project: '/work/alpha', name: 'alpha', nodes, edges,
  counts: { notes: 5, links: 3, dangling: 1, abstained: 0, duplicates: 0, capped: false }, ...over,
})
const list = { projects: [{ project: '/work/alpha', name: 'alpha', kit_notes: 4, claude_notes: 1 }, { project: '/work/beta', name: 'beta', kit_notes: 1, claude_notes: 0 }] }

function stub({ projects = list, graphs = {}, fail = false } = {}) {
  const calls = []
  vi.stubGlobal('fetch', async (url) => {
    calls.push(String(url))
    const u = new URL(String(url), 'http://127.0.0.1')
    if (fail) return { ok: false, status: 500, json: async () => ({}) }
    const project = u.searchParams.get('project')
    const body = project ? (graphs[project] ?? graph({ project, name: projects.projects.find((p) => p.project === project)?.name })) : projects
    return { ok: true, status: 200, json: async () => body }
  })
  return calls
}
beforeEach(() => {
  window.location.hash = '#secret-token'
})
afterEach(() => vi.unstubAllGlobals())

describe('rings layout (AC4)', () => {
  it('puts MEMORY.md at the centre and each ring at its own radius', () => {
    const placed = layout(nodes)
    const at = (id) => placed.find((p) => p.id === id)
    expect(at('kit:memory/MEMORY.md')).toMatchObject({ x: 320, y: 320 })
    const radius = (p) => Math.round(Math.hypot(p.x - 320, p.y - 320))
    expect(new Set(placed.filter((p) => p.ring === 1).map(radius)).size).toBe(1)
    expect(radius(at('kit:memory/decisions.md'))).toBeLessThan(radius(at('claude:1:memory/feedback.md')))
    expect(radius(at('dangling:ghost'))).toBeGreaterThan(radius(at('claude:1:memory/feedback.md')))
  })

  it('is the same SVG, byte for byte, for the same input and as the golden file', () => {
    const html = () => renderToStaticMarkup(<RingsGraph nodes={nodes} edges={edges} selectedId={null} onSelect={() => {}} />)
    expect(html()).toBe(html())
    const golden = readFileSync(resolve(process.cwd(), 'src/test/memory-graph.golden.svg'), 'utf8').trimEnd()
    expect(html()).toBe(golden)
  })

  it('does not depend on the order the nodes arrive in', () => {
    const a = renderToStaticMarkup(<RingsGraph nodes={nodes} edges={edges} selectedId={null} onSelect={() => {}} />)
    const b = renderToStaticMarkup(<RingsGraph nodes={[...nodes].reverse()} edges={edges} selectedId={null} onSelect={() => {}} />)
    expect(a).toBe(b)
  })

  it('has no NaN for one node, for none, and for a ring with a single node', () => {
    for (const set of [[], [nodes[0]], [nodes[5]]]) {
      expect(renderToStaticMarkup(<RingsGraph nodes={set} edges={[]} selectedId={null} onSelect={() => {}} />)).not.toMatch(/NaN|Infinity/)
    }
  })
})

describe('Memory view', () => {
  it('loads the project list, then the first project, and draws one labelled node per note', async () => {
    const calls = stub()
    render(<MemoryView />)
    const svg = await screen.findByRole('group', { name: /Memory graph of alpha/ })
    expect(calls[0]).toBe('/api/memory-graph')
    expect(calls[1]).toBe('/api/memory-graph?project=%2Fwork%2Falpha')
    expect(within(svg).getAllByRole('button')).toHaveLength(6)
    expect(screen.getByText(/5 notes/)).toBeInTheDocument()
    expect(screen.getByText(/3 links/)).toBeInTheDocument()
  })

  it('has a project picker that loads the chosen project', async () => {
    const calls = stub()
    render(<MemoryView />)
    const picker = await screen.findByRole('combobox', { name: 'Project' })
    expect(within(picker).getAllByRole('option').map((o) => o.textContent)).toEqual(['alpha', 'beta'])
    fireEvent.change(picker, { target: { value: '/work/beta' } })
    await waitFor(() => expect(calls.at(-1)).toBe('/api/memory-graph?project=%2Fwork%2Fbeta'))
  })

  it('shows name, description, path and links in/out for the selected node, and nothing else (AC5)', async () => {
    stub()
    render(<MemoryView />)
    const svg = await screen.findByRole('group', { name: /Memory graph/ })
    fireEvent.click(within(svg).getByRole('button', { name: /^decisions/ }))
    const panel = screen.getByRole('complementary', { name: 'Selected note' })
    expect(panel).toHaveTextContent('decisions')
    expect(panel).toHaveTextContent('Why we chose things')
    expect(panel).toHaveTextContent('memory/decisions.md')
    expect(panel).toHaveTextContent('links in 3')
    expect(panel).toHaveTextContent('out 2')
  })

  it('selects with the keyboard', async () => {
    stub()
    render(<MemoryView />)
    const svg = await screen.findByRole('group', { name: /Memory graph/ })
    const button = within(svg).getByRole('button', { name: /^glossary/ })
    expect(button).toHaveAttribute('tabindex', '0')
    fireEvent.keyDown(button, { key: 'Enter' })
    expect(screen.getByRole('complementary', { name: 'Selected note' })).toHaveTextContent('memory/glossary.md')
    fireEvent.keyDown(within(svg).getByRole('button', { name: /^LR-0001/ }), { key: ' ' })
    expect(screen.getByRole('complementary', { name: 'Selected note' })).toHaveTextContent('learning-records/LR-0001.md')
  })

  it('shows a dangling link as an unresolved node with no path', async () => {
    stub()
    render(<MemoryView />)
    const svg = await screen.findByRole('group', { name: /Memory graph/ })
    fireEvent.click(within(svg).getByRole('button', { name: /^ghost.*unresolved/ }))
    const panel = screen.getByRole('complementary', { name: 'Selected note' })
    expect(panel).toHaveTextContent('unresolved link')
    expect(panel).not.toHaveTextContent('memory/')
  })

  it('has a legend that names kit memory, Claude memory and unresolved links', async () => {
    stub()
    render(<MemoryView />)
    const legend = await screen.findByRole('list', { name: 'Legend' })
    expect(legend).toHaveTextContent('kit memory')
    expect(legend).toHaveTextContent('Claude memory')
    expect(legend).toHaveTextContent('unresolved link')
  })

  it('draws at most MAX_DRAWN nodes and says how many more there are', async () => {
    const many = Array.from({ length: MAX_DRAWN + 12 }, (_, i) => node(`kit:memory/n${String(i).padStart(4, '0')}.md`, 'kit', 2, `n${i}`))
    stub({ graphs: { '/work/alpha': graph({ nodes: many, edges: [], counts: { notes: many.length, links: 0, dangling: 0, abstained: 0, duplicates: 0, capped: false } }) } })
    render(<MemoryView />)
    const svg = await screen.findByRole('group', { name: /Memory graph/ })
    expect(within(svg).getAllByRole('button')).toHaveLength(MAX_DRAWN)
    expect(screen.getByText('12 more notes not drawn')).toBeInTheDocument()
  })

  it('says so when there are no memory notes at all', async () => {
    stub({ projects: { projects: [] } })
    render(<MemoryView />)
    expect(await screen.findByText('No memory notes found.')).toBeInTheDocument()
  })

  it('shows an error when the request fails', async () => {
    stub({ fail: true })
    render(<MemoryView />)
    expect(await screen.findByRole('alert')).toHaveTextContent('Could not load the memory graph.')
  })

  it('never renders a note body (AC1): only the allowed fields are drawn', async () => {
    const leaky = nodes.map((n) => ({ ...n, body: 'SECRET_Y' }))
    stub({ graphs: { '/work/alpha': graph({ nodes: leaky }) } })
    const { container } = render(<MemoryView />)
    const svg = await screen.findByRole('group', { name: /Memory graph/ })
    fireEvent.click(within(svg).getByRole('button', { name: /^decisions/ }))
    expect(container.innerHTML).not.toContain('SECRET_Y')
  })
})

describe('design tokens (UI AC 2)', () => {
  it('styles the memory view with tokens only, no raw colour literals', () => {
    const css = readFileSync(resolve(process.cwd(), 'src/app.css'), 'utf8')
    const rules = css.split('\n').filter((line) => /\.memory/.test(line))
    expect(rules.length).toBeGreaterThan(5)
    for (const line of rules) expect(line).not.toMatch(/#[0-9a-fA-F]{3,8}\b|rgba?\(|hsla?\(/)
  })
})

describe('default project and gone line (HUN-188 AC4, AC6)', () => {
  const many = {
    projects: [
      { project: '-home-me-claude-folder', name: '-home-me-claude-folder', kit_notes: 0, claude_notes: 2 },
      { project: '/work/zeta', name: 'zeta', kit_notes: 3, claude_notes: 1, easy_memory_notes: 6 },
      { project: '/work/alpha', name: 'alpha', kit_notes: 9, claude_notes: 1 },
      { project: '/work/beta', name: 'beta', kit_notes: 5, claude_notes: 0, easy_memory_notes: 5 },
    ],
  }

  it('opens on the project with the most notes, counting easy-memory notes; ties go to the lower name', async () => {
    const calls = stub({ projects: many })
    render(<MemoryView />)
    await screen.findByRole('group', { name: /^Memory graph of / })
    // alpha 10, beta 10, zeta 10: all tie, alpha wins by name
    expect(screen.getByLabelText('Project')).toHaveValue('/work/alpha')
    expect(calls.at(-1)).toBe('/api/memory-graph?project=%2Fwork%2Falpha')
  })

  it('never opens on a Claude-folder-only key while a real project has more notes', async () => {
    stub({ projects: { projects: [many.projects[0], { project: '/work/real', name: 'real', kit_notes: 4, claude_notes: 0 }] } })
    render(<MemoryView />)
    await screen.findByRole('group', { name: /^Memory graph of / })
    expect(screen.getByLabelText('Project')).toHaveValue('/work/real')
  })

  it('treats a missing note count as 0 and still opens on a project', async () => {
    stub({ projects: { projects: [{ project: '/work/x', name: 'x' }] } })
    render(<MemoryView />)
    await screen.findByRole('group', { name: /^Memory graph of / })
    expect(screen.getByLabelText('Project')).toHaveValue('/work/x')
  })

  it('says how many gone folders were hidden, and nothing when `gone` is 0 or missing', async () => {
    stub({ projects: { ...list, gone: 4 } })
    const first = render(<MemoryView />)
    expect(await screen.findByText('4 gone folders hidden')).toBeInTheDocument()
    first.unmount()
    stub({ projects: { ...list, gone: 0 } })
    render(<MemoryView />)
    await screen.findByLabelText('Project')
    expect(screen.queryByText(/gone folder/)).toBeNull()
  })

  it('keeps the empty text when the list is empty', async () => {
    stub({ projects: { projects: [] } })
    render(<MemoryView />)
    expect(await screen.findByText('No memory notes found.')).toBeInTheDocument()
  })
})
