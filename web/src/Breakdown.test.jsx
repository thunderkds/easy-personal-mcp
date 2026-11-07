import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { render, screen, fireEvent, within } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import Breakdown from './Breakdown.jsx'

// HUN-207 (FR-069): the Scratch group of By project opens to list its folders, from data already in the group.
const group = (key, tokens, extra = {}) => ({
  key,
  messages: 1,
  input_tokens: tokens,
  output_tokens: 0,
  cache_creation_input_tokens: 0,
  cache_read_input_tokens: 0,
  api_equivalent_cost: { usd: 1, price_table_date: '2026-10-01', abstained_models: [] },
  ...extra,
})
const scratch = (folders) => group('Scratch', folders.reduce((n, f) => n + f.input_tokens, 0), { scratch: true, folders })
const data = (groups) => ({ by: 'project', period: {}, groups })
const section = () => screen.getByRole('region', { name: 'By project' })
const bars = () => within(section()).getAllByRole('list')[0]

afterEach(() => vi.unstubAllGlobals())

describe('Scratch group in By project (AC1-AC3)', () => {
  const folders = [group('live-trace-1', 300), group('proj', 200), group('proj', 100)]
  const render3 = () => render(<Breakdown by="project" data={data([group('app', 5000), scratch(folders), group('tool', 50)])} />)

  it('shows one closed "Scratch (3 folders)" row at its sorted place; click opens 3 indented rows; click again closes (AC1)', () => {
    render3()
    const toggle = within(bars()).getByRole('button', { name: /Scratch \(3 folders\)/ })
    expect(toggle).toHaveAttribute('aria-expanded', 'false')
    expect(within(bars()).getAllByRole('listitem')).toHaveLength(3)
    expect(within(bars()).getAllByRole('listitem').map((li) => li.textContent.split(/[\d]/)[0])).toEqual(['app', 'Scratch (', 'tool'])
    expect(screen.queryByText('live-trace-1')).toBeNull()

    fireEvent.click(toggle)
    expect(toggle).toHaveAttribute('aria-expanded', 'true')
    const nested = within(screen.getByRole('list', { name: 'Scratch folders' })).getAllByRole('listitem')
    expect(nested).toHaveLength(3)
    expect(nested.map((li) => li.textContent)).toEqual(['live-trace-1300', 'proj200', 'proj100'])
    expect(within(bars()).getAllByRole('listitem')).toHaveLength(6)

    fireEvent.click(toggle)
    expect(toggle).toHaveAttribute('aria-expanded', 'false')
    expect(screen.queryByText('live-trace-1')).toBeNull()
  })

  it('folder bars are a share of the period total, like the other rows', () => {
    render3()
    fireEvent.click(within(bars()).getByRole('button', { name: /Scratch/ }))
    const fill = (name) => screen.getByText(name).closest('li').querySelector('.bar-fill')
    // total = 5000 + 600 + 50 = 5650; 300 / 5650 = 5.31 %
    expect(fill('live-trace-1').style.width).toBe(`${(300 / 5650) * 100}%`)
  })

  it('a folder row has a title with its full name', () => {
    render3()
    fireEvent.click(within(bars()).getByRole('button', { name: /Scratch/ }))
    expect(screen.getByText('live-trace-1')).toHaveAttribute('title', 'live-trace-1')
  })

  it('one folder reads "1 folder"', () => {
    render(<Breakdown by="project" data={data([scratch([group('only', 10)])])} />)
    expect(screen.getByRole('button', { name: /Scratch \(1 folder\)/ })).toBeInTheDocument()
  })

  it('opening Scratch sends no request (AC2)', () => {
    const fetchSpy = vi.fn()
    vi.stubGlobal('fetch', fetchSpy)
    render3()
    fireEvent.click(within(bars()).getByRole('button', { name: /Scratch/ }))
    expect(fetchSpy).not.toHaveBeenCalled()
  })

  it('without a Scratch group By project renders as before: no buttons, one row per group (AC3)', () => {
    render(<Breakdown by="project" data={data([group('app', 5000), group('app', 40)])} />)
    expect(within(bars()).queryByRole('button')).toBeNull()
    expect(within(bars()).getAllByRole('listitem')).toHaveLength(2)
  })

  it('a Scratch group without a folders list stays a plain row', () => {
    render(<Breakdown by="project" data={data([group('Scratch', 10, { scratch: true })])} />)
    expect(within(bars()).queryByRole('button')).toBeNull()
  })

  it('the disclosure works from the keyboard (a real button)', () => {
    render3()
    const toggle = within(bars()).getByRole('button', { name: /Scratch/ })
    toggle.focus()
    expect(toggle).toHaveFocus()
    expect(toggle.tagName).toBe('BUTTON')
    expect(toggle).toHaveAttribute('type', 'button')
  })

  it('CSS: the HUN-207 block uses tokens only, no hard-coded colour', () => {
    const css = readFileSync(resolve(process.cwd(), 'src/app.css'), 'utf8')
    const block = css.slice(css.indexOf('/* Scratch group in By project'))
    expect(block).toMatch(/\.bar-row-toggle/)
    expect(block).not.toMatch(/#[0-9a-fA-F]{3,8}\b|rgb\(|hsl\(/)
  })
})
