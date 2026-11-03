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
    expect(within(picker).getAllByRole('option').map((o) => o.textContent)).toEqual(['alpha · kit 4 · Claude 1', 'beta · kit 1'])
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

  it('never opens on a Claude-folder-only key even when it has MORE notes than a real project', async () => {
    const folder = { project: '-home-me-claude-folder', name: '-home-me-claude-folder', kit_notes: 0, claude_notes: 40 }
    stub({ projects: { projects: [folder, { project: '/work/real', name: 'real', kit_notes: 1, claude_notes: 0 }] } })
    render(<MemoryView />)
    await screen.findByRole('group', { name: /^Memory graph of / })
    expect(screen.getByLabelText('Project')).toHaveValue('/work/real')
  })

  it('opens on the folder-only key with the most notes when no real project exists', async () => {
    const folder = (key, n) => ({ project: key, name: key, kit_notes: 0, claude_notes: n })
    stub({ projects: { projects: [folder('-a-folder', 2), folder('-b-folder', 5)] } })
    render(<MemoryView />)
    await screen.findByRole('group', { name: /^Memory graph of / })
    expect(screen.getByLabelText('Project')).toHaveValue('-b-folder')
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

// ---- HUN-185: the easy-memory layer (FR-052, FR-053) ----
const em = (path, cited, stale, newest = '2026-11-01') => ({
  id: `easymem:${path}`, kind: 'easy_memory', ring: 4, name: path.split('/').at(-1), stem: path.split('/').at(-1), description: '',
  path, account: null, abstained: false, links_in: 0, links_out: 0, cited, newest, stale,
})
const days = (counts = []) => Array.from({ length: 30 }, (_, i) => ({ date: `2026-10-${String(i + 1).padStart(2, '0')}`, count: counts[i] ?? 0 }))
const block = (over = {}) => ({ notes: 98, abstained: 0, days: days([0, 3, 0, 5]), prompts_on: 12, injected: 5, files_cited: 40, files_not_drawn: 0, outside: 0, ...over })
const emNodes = [
  em('src/app.py', 9, 'fresh'),
  em('src/gone.py', 2, 'gone'),
  em('src/changed.py', 4, 'stale', '2026-10-30'),
  em('docs/unchecked.bin', 1, 'unchecked'),
]
const merged = nodes.map((n) => (n.id === 'kit:memory/decisions.md' ? { ...n, cited: 3, newest: '2026-10-31', stale: 'stale' } : n))
const withEm = (over = {}) => graph({ nodes: [...merged, ...emNodes], easy_memory: block(), ...over })
const open = async (g) => {
  stub({ graphs: { '/work/alpha': g } })
  render(<MemoryView />)
  return screen.findByRole('group', { name: /Memory graph of alpha/ })
}
const html = (set) => renderToStaticMarkup(<RingsGraph nodes={set} edges={[]} selectedId={null} onSelect={() => {}} />)

describe('ring 4 and the easy-memory kind (HUN-185 AC1, AC7)', () => {
  it('draws easy_memory nodes outside the unresolved-link ring, with one more dotted ring', () => {
    const placed = layout([...nodes, ...emNodes])
    const centre = placed.find((p) => p.ring === 0)
    const r = (id) => Math.round(Math.hypot(placed.find((p) => p.id === id).x - centre.x, placed.find((p) => p.id === id).y - centre.y))
    expect(r('easymem:src/app.py')).toBeGreaterThan(r('dangling:ghost'))
    expect(html([...nodes, ...emNodes]).match(/class="memory-ring"/g)).toHaveLength(4)
  })

  it('keeps the old canvas when there is no ring 4', () => {
    expect(html(nodes)).toContain('viewBox="0 0 640 640"')
    expect(html([...nodes, ...emNodes])).not.toContain('viewBox="0 0 640 640"')
  })

  it('gives easy-memory nodes their own class, a name with the kind, and a radius that grows with cited inside a fixed range', () => {
    const radius = (cited) => Number(html([em('a/b.py', cited, 'fresh')]).match(/<circle cx="[^"]+" cy="[^"]+" r="([\d.]+)"/)[1])
    expect(html(emNodes)).toContain('memory-node memory-node-easy_memory')
    expect(html(emNodes)).toContain('aria-label="app.py, easy-memory file"')
    expect(radius(1)).toBeLessThan(radius(9))
    expect(radius(9)).toBeLessThan(radius(36))
    expect(radius(36)).toBe(radius(5000))
    expect(radius(1)).toBeGreaterThanOrEqual(5)
    expect(radius(5000)).toBeLessThanOrEqual(14)
  })

  it('is deterministic with ring 4: same nodes in any order give the same SVG and the same coordinates', () => {
    const set = [...nodes, ...emNodes]
    expect(html([...set].reverse())).toBe(html(set))
    expect(layout([...set].reverse())).toEqual(layout(set))
    expect(html(set)).not.toMatch(/NaN|Infinity/)
  })

  it('has no NaN for a ring 4 with a single node and for ring 4 alone (centre empty)', () => {
    expect(html([emNodes[0]])).not.toMatch(/NaN|Infinity/)
    expect(html(emNodes)).not.toMatch(/NaN|Infinity/)
  })

  it('shows "easy-memory file" in the legend only when the project has an easy-memory layer', async () => {
    await open(withEm())
    expect(screen.getByRole('list', { name: 'Legend' })).toHaveTextContent('easy-memory file')
  })
})

describe('stale mark (HUN-185 AC2)', () => {
  it('marks stale and gone nodes of any kind with an accessible name, and nothing else', async () => {
    const svg = await open(withEm())
    expect(within(svg).getAllByRole('img', { name: 'changed since the newest note' })).toHaveLength(2) // easy-memory + merged kit node
    expect(within(svg).getAllByRole('img', { name: 'file gone' })).toHaveLength(1)
    expect(within(svg).getAllByRole('img')).toHaveLength(3)
    expect(within(svg).getByRole('button', { name: 'gone.py, easy-memory file, file gone' })).toBeInTheDocument()
    expect(within(svg).getByRole('button', { name: 'decisions, kit memory, changed since the newest note' })).toBeInTheDocument()
    expect(within(svg).getByRole('button', { name: 'app.py, easy-memory file' })).toBeInTheDocument()
    expect(within(svg).getByRole('button', { name: 'unchecked.bin, easy-memory file' })).toBeInTheDocument()
  })

  it('does not tell the state by colour alone: the mark carries a symbol', () => {
    expect(html([emNodes[1]])).toContain('>×<')
    expect(html([emNodes[2]])).toContain('>!<')
    expect(html([emNodes[0], emNodes[3]])).not.toMatch(/memory-mark/)
  })

  it('draws a node without easy-memory fields exactly as before', () => {
    expect(html(nodes)).not.toMatch(/memory-mark/)
  })
})

describe('click panel (HUN-185 AC2, AC3)', () => {
  it('shows path, citation count, newest date and stale state for an easy-memory file, and no note fields', async () => {
    const svg = await open(withEm())
    fireEvent.click(within(svg).getByRole('button', { name: /^changed\.py/ }))
    const panel = screen.getByRole('complementary', { name: 'Selected note' })
    expect(panel).toHaveTextContent('easy-memory file')
    expect(panel).toHaveTextContent('src/changed.py')
    expect(panel).toHaveTextContent('cited by 4 notes')
    expect(panel).toHaveTextContent('newest note 2026-10-30')
    expect(panel).toHaveTextContent('changed since the newest note')
    expect(panel).not.toHaveTextContent('links in')
    expect(panel).not.toHaveTextContent('No description')
  })

  it('says "not checked" for unchecked, "file gone" for gone and "unchanged" for fresh', async () => {
    const svg = await open(withEm())
    fireEvent.click(within(svg).getByRole('button', { name: /^unchecked\.bin/ }))
    expect(screen.getByRole('complementary', { name: 'Selected note' })).toHaveTextContent('not checked')
    expect(screen.getByRole('complementary', { name: 'Selected note' })).toHaveTextContent('cited by 1 note')
    fireEvent.click(within(svg).getByRole('button', { name: /^gone\.py/ }))
    expect(screen.getByRole('complementary', { name: 'Selected note' })).toHaveTextContent('file gone')
    fireEvent.click(within(svg).getByRole('button', { name: /^app\.py/ }))
    expect(screen.getByRole('complementary', { name: 'Selected note' })).toHaveTextContent('unchanged since the newest note')
  })

  it('keeps the existing fields of a merged kit node and adds the same three', async () => {
    const svg = await open(withEm())
    fireEvent.click(within(svg).getByRole('button', { name: /^decisions/ }))
    const panel = screen.getByRole('complementary', { name: 'Selected note' })
    expect(panel).toHaveTextContent('Why we chose things')
    expect(panel).toHaveTextContent('memory/decisions.md')
    expect(panel).toHaveTextContent('links in 3')
    expect(panel).toHaveTextContent('cited by 3 notes')
    expect(panel).toHaveTextContent('newest note 2026-10-31')
    expect(panel).toHaveTextContent('changed since the newest note')
  })

  it('never renders text the API should not send', async () => {
    const leaky = emNodes.map((n) => ({ ...n, prompt: 'SECRET_P', answer: 'SECRET_A', session_id: 'SECRET_S' }))
    const svg = await open(withEm({ nodes: [...merged, ...leaky], easy_memory: { ...block(), prompt: 'SECRET_P' } }))
    fireEvent.click(within(svg).getByRole('button', { name: /^app\.py/ }))
    expect(document.body.innerHTML).not.toMatch(/SECRET_/)
  })
})

describe('easy-memory panel (HUN-185 AC4, AC5)', () => {
  it('shows a 30-bar sparkline with a text summary, the total, and the hit rate with the prompt count', async () => {
    await open(withEm())
    const panel = screen.getByRole('region', { name: 'easy-memory' })
    const spark = within(panel).getByRole('img', { name: /Notes saved per day, last 30 days: 8 in the window, 98 notes in total/ })
    expect(spark.querySelectorAll('rect')).toHaveLength(30)
    expect(panel).toHaveTextContent('98 notes')
    expect(panel).toHaveTextContent('41.7% of 12 prompts')
  })

  it('says "no prompts" when the prompt count is 0, never NaN or 0%', async () => {
    await open(withEm({ easy_memory: block({ prompts_on: 0, injected: 0 }) }))
    const panel = screen.getByRole('region', { name: 'easy-memory' })
    expect(panel).toHaveTextContent('no prompts')
    expect(panel).not.toHaveTextContent(/NaN|Infinity|0\.0%/)
  })

  it('says "of 1 prompt" for one prompt and keeps big counts short', async () => {
    await open(withEm({ easy_memory: block({ prompts_on: 1, injected: 1 }) }))
    expect(screen.getByRole('region', { name: 'easy-memory' })).toHaveTextContent('100.0% of 1 prompt')
  })

  it('shows files not drawn and abstained notes only when above 0', async () => {
    await open(withEm({ easy_memory: block({ files_not_drawn: 16, abstained: 2 }) }))
    const panel = screen.getByRole('region', { name: 'easy-memory' })
    expect(panel).toHaveTextContent('16 files not drawn')
    expect(panel).toHaveTextContent('2 notes could not be read')
  })

  it('leaves out the two lines when they are 0', async () => {
    await open(withEm())
    const panel = screen.getByRole('region', { name: 'easy-memory' })
    expect(panel).not.toHaveTextContent('not drawn')
    expect(panel).not.toHaveTextContent('could not be read')
  })

  it('shows a flat sparkline and total 0 for an empty layer that is present', async () => {
    await open(withEm({ nodes: merged, easy_memory: block({ notes: 0, days: days(), prompts_on: 0, injected: 0 }) }))
    const panel = screen.getByRole('region', { name: 'easy-memory' })
    expect(panel).toHaveTextContent('0 notes')
    const heights = [...panel.querySelectorAll('rect')].map((r) => r.getAttribute('height'))
    expect(heights).toHaveLength(30)
    expect(new Set(heights).size).toBe(1)
    expect(panel.outerHTML).not.toMatch(/NaN|Infinity/)
  })

  it('with easy_memory null: no panel, no ring 4, no easy-memory legend entry, and the old canvas', async () => {
    const svg = await open(graph({ easy_memory: null }))
    expect(screen.queryByRole('region', { name: 'easy-memory' })).toBeNull()
    expect(svg.querySelectorAll('.memory-ring')).toHaveLength(3)
    expect(svg).toHaveAttribute('viewBox', '0 0 640 640')
    expect(screen.getByRole('list', { name: 'Legend' })).not.toHaveTextContent('easy-memory')
  })

  it('with the field missing (older server): same as null', async () => {
    await open(graph())
    expect(screen.queryByRole('region', { name: 'easy-memory' })).toBeNull()
  })

  it('draws an easy-memory-only project: ring 4 only, empty centre, panel shown', async () => {
    const svg = await open(graph({ nodes: emNodes, edges: [], counts: { notes: 0, links: 0, dangling: 0, abstained: 0, duplicates: 0, capped: false }, easy_memory: block() }))
    expect(within(svg).getAllByRole('button')).toHaveLength(4)
    expect(screen.getByRole('region', { name: 'easy-memory' })).toBeInTheDocument()
  })

  it('still draws all ring 4 nodes when the other nodes hit the MAX_DRAWN cap', async () => {
    const many = Array.from({ length: MAX_DRAWN + 5 }, (_, i) => node(`kit:memory/n${String(i).padStart(4, '0')}.md`, 'kit', 2, `n${i}`))
    const svg = await open(withEm({ nodes: [...many, ...emNodes], edges: [], counts: { notes: many.length, links: 0, dangling: 0, abstained: 0, duplicates: 0, capped: false } }))
    expect(within(svg).getAllByRole('button')).toHaveLength(MAX_DRAWN + 4)
    expect(screen.getByText('5 more notes not drawn')).toBeInTheDocument()
  })

  it('labels at most the nodes that fit: with many nodes only the inner rings and the selection are labelled', () => {
    const outer = Array.from({ length: 30 }, (_, i) => em(`src/dir/a-very-long-file-name-${String(i).padStart(2, '0')}.py`, i + 1, 'fresh'))
    const many = Array.from({ length: 40 }, (_, i) => node(`kit:memory/n${i}.md`, 'kit', 2, `n${i}`))
    expect(html([...many, ...outer]).match(/<text/g) ?? []).toHaveLength(0)
    expect(html(outer).match(/<text/g)).toHaveLength(30)
  })
})

describe('ring 4 labels (HUN-185 edge case)', () => {
  const outer = Array.from({ length: 6 }, (_, i) => em(`src/dir/a-very-long-file-name-${i}.py`, 2, 'fresh'))

  it('cuts a long name to 10 characters and puts every second label above its node', () => {
    const svg = html(outer)
    expect(svg).toContain('>a-very-lo…<')
    const cy = [...svg.matchAll(/<circle cx="[^"]+" cy="([\d.]+)"/g)].map((m) => Number(m[1]))
    const ty = [...svg.matchAll(/<text x="[^"]+" y="([\d.]+)"/g)].map((m) => Number(m[1]))
    expect(ty).toHaveLength(6)
    expect(ty.map((y, i) => y < cy[i])).toEqual([false, true, false, true, false, true])
  })

  it('puts the panels under the graph only when ring 4 exists', async () => {
    const svg = await open(withEm())
    expect(svg.closest('.memory-body')).toHaveAttribute('data-wide', 'true')
  })

  it('keeps the side-by-side layout without ring 4', async () => {
    const svg = await open(graph({ easy_memory: null }))
    expect(svg.closest('.memory-body')).not.toHaveAttribute('data-wide')
  })
})

describe('project list count (HUN-185 AC6)', () => {
  const projects = {
    projects: [
      { project: '/work/alpha', name: 'alpha', kit_notes: 4, claude_notes: 1, easy_memory_notes: 6 },
      { project: '/work/only', name: 'only', kit_notes: 0, claude_notes: 0, easy_memory_notes: 3 },
    ],
  }

  it('shows "easy-memory N" beside the kit and Claude counts', async () => {
    stub({ projects })
    render(<MemoryView />)
    const picker = await screen.findByRole('combobox', { name: 'Project' })
    expect(within(picker).getAllByRole('option').map((o) => o.textContent)).toEqual(['alpha · kit 4 · Claude 1 · easy-memory 6', 'only · easy-memory 3'])
  })

  it('lists an easy-memory-only project and opens it', async () => {
    const calls = stub({ projects })
    render(<MemoryView />)
    const picker = await screen.findByRole('combobox', { name: 'Project' })
    fireEvent.change(picker, { target: { value: '/work/only' } })
    await waitFor(() => expect(calls.at(-1)).toBe('/api/memory-graph?project=%2Fwork%2Fonly'))
    expect(await screen.findByRole('group', { name: /Memory graph of only/ })).toBeInTheDocument()
  })

  it('compacts a big count', async () => {
    stub({ projects: { projects: [{ project: '/work/big', name: 'big', kit_notes: 12345, claude_notes: 0 }] } })
    render(<MemoryView />)
    const picker = await screen.findByRole('combobox', { name: 'Project' })
    expect(within(picker).getAllByRole('option')[0].textContent).toBe('big · kit 12.3 K')
  })
})

describe('design tokens and contrast for the easy-memory layer (HUN-185 UI AC2)', () => {
  const tokens = readFileSync(resolve(process.cwd(), 'src/tokens.css'), 'utf8')
  const css = readFileSync(resolve(process.cwd(), 'src/app.css'), 'utf8')
  const colour = (name) => tokens.match(new RegExp(`${name}:\\s*(#[0-9a-fA-F]{6})\\s*;`))[1]
  const channel = (hex, i) => parseInt(hex.slice(1 + i * 2, 3 + i * 2), 16) / 255
  const luminance = (hex) => {
    const [r, g, b] = [0, 1, 2].map((i) => channel(hex, i)).map((c) => (c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4))
    return 0.2126 * r + 0.7152 * g + 0.0722 * b
  }
  const contrast = (a, b) => {
    const [hi, lo] = [luminance(colour(a)), luminance(colour(b))].sort((x, y) => y - x)
    return (hi + 0.05) / (lo + 0.05)
  }
  const rule = (selector) => css.split('\n').find((line) => line.startsWith(`${selector} {`)) ?? ''

  it('takes the node, mark and sparkline colours from tokens that exist', () => {
    expect(rule('.memory-node-easy_memory circle')).toContain('var(--color-heading)')
    expect(rule('.memory-mark circle')).toContain('var(--color-red)')
    expect(rule('.memory-spark rect')).toContain('var(--color-accent)')
    expect(rule('.memory-easy')).toContain('var(--font-mono)')
    expect(rule('.memory-cited')).toContain('var(--font-mono)')
  })

  it('has a mark symbol readable on its disc (4.5:1) and shapes that stand out from the graph background (3:1)', () => {
    expect(contrast('--color-bg', '--color-red')).toBeGreaterThanOrEqual(4.5)
    for (const fill of ['--color-heading', '--color-accent', '--color-amber', '--color-red']) expect(contrast(fill, '--color-input')).toBeGreaterThanOrEqual(3)
  })

  it('has text in the panel at 4.5:1 on the page background', () => {
    for (const text of ['--color-heading', '--color-secondary', '--color-muted']) expect(contrast(text, '--color-bg')).toBeGreaterThanOrEqual(4.5)
  })
})

// ---- HUN-198: memory on vs off (FR-065) ----
describe('memory on vs off block (HUN-198 AC4..AC7)', () => {
  const NOW = new Date('2026-11-03T12:06:31Z').getTime()
  beforeEach(() => {
    vi.useFakeTimers({ toFake: ['Date'] })
    vi.setSystemTime(NOW)
  })
  afterEach(() => vi.useRealTimers())

  const report = (over = {}) => ({
    state: 'ok', generated: '2026-11-03T10:06:31Z', min_sessions: 10, too_few: true, on_vs_off_pct: null,
    on: { sessions: 3, tokens_all_median: 18658261 }, off: { sessions: 5, tokens_all_median: 16553412 }, ...over,
  })
  const shown = async (over) => {
    await open(withEm({ easy_memory: block({ report: over === undefined ? report() : over }) }))
    return screen.getByRole('group', { name: 'memory on vs off' })
  }

  it('too_few: both group rows, the age, and the too-few line; no "Saves" or "Uses" (AC4, AC7)', async () => {
    const group = await shown()
    expect(group).toHaveTextContent('Memory on vs off')
    expect(group).toHaveTextContent('updated 2 h ago')
    expect(group).toHaveTextContent('On 3 sessions 18.7 M tokens / session')
    expect(group).toHaveTextContent('Off 5 sessions 16.6 M tokens / session')
    expect(group).toHaveTextContent('too few sessions to compare (3 on / 5 off, need 10 each)')
    expect(group).not.toHaveTextContent(/Saves|Uses/)
  })

  it('sits inside the easy-memory panel under the hit-rate line', async () => {
    await shown()
    const panel = screen.getByRole('region', { name: 'easy-memory' })
    const group = within(panel).getByRole('group', { name: 'memory on vs off' })
    const rate = within(panel).getByText('Hit rate').closest('p')
    expect(rate.compareDocumentPosition(group) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
  })

  it.each([
    [-12, 'Saves 12%'],
    [8, 'Uses 8% more'],
    [0, 'No difference'],
  ])('enough sessions, pct %i: "%s" (AC5)', async (pct, text) => {
    const group = await shown(report({ too_few: false, on_vs_off_pct: pct }))
    expect(group).toHaveTextContent(text)
    expect(group).not.toHaveTextContent('too few sessions')
    expect(group).toHaveTextContent('On 3 sessions')
  })

  it('enough sessions, pct null: no saving line and no too-few line (AC5)', async () => {
    const group = await shown(report({ too_few: false, on_vs_off_pct: null }))
    expect(group).not.toHaveTextContent(/Saves|Uses|No difference|too few/)
  })

  it('a pct is never shown while too_few is true, even when the file carries one', async () => {
    const group = await shown(report({ too_few: true, on_vs_off_pct: -30 }))
    expect(group).not.toHaveTextContent(/Saves|30/)
  })

  it('unsupported: only the notice, no numbers (AC6)', async () => {
    const group = await shown({ state: 'unsupported' })
    expect(group).toHaveTextContent('report format not supported')
    expect(group).not.toHaveTextContent(/\d/)
  })

  it('none: "no report yet" (AC6)', async () => {
    const group = await shown({ state: 'none' })
    expect(group).toHaveTextContent('no report yet')
    expect(group).not.toHaveTextContent(/sessions|updated/)
  })

  it('an answer without a report key draws no block (older server)', async () => {
    await open(withEm())
    expect(screen.queryByRole('group', { name: 'memory on vs off' })).toBeNull()
  })

  it('an empty group reads "no sessions", never NaN or 0 tokens', async () => {
    const group = await shown(report({ off: { sessions: 0, tokens_all_median: null } }))
    expect(group).toHaveTextContent('Off 0 sessions no sessions')
    expect(group.innerHTML).not.toMatch(/NaN|null|undefined/)
  })

  it('one session is singular', async () => {
    const group = await shown(report({ on: { sessions: 1, tokens_all_median: 5000 } }))
    expect(group).toHaveTextContent('On 1 session 5,000 tokens / session')
  })

  it.each([
    ['a future time', '2026-11-04T10:00:00Z'],
    ['an unparseable time', 'last tuesday'],
  ])('%s: no "updated" text, the block still shows (edge case)', async (_name, generated) => {
    const group = await shown(report({ generated }))
    expect(group).not.toHaveTextContent('updated')
    expect(group).toHaveTextContent('On 3 sessions')
  })

  it('draws only the fields it is given: planted extra text never reaches the page', async () => {
    const group = await shown({ ...report(), prompt: 'SECRET_P', store: { notes: 'SECRET_S' } })
    expect(group.ownerDocument.body.innerHTML).not.toMatch(/SECRET_P|SECRET_S/)
  })
})
