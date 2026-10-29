import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { render, screen, within } from '@testing-library/react'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import App from './App.jsx'
import { fakeFetch } from './test/fixtures.js'

// HUN-131: the neon pass. The values come from the design's renderVals().
const tokens = readFileSync(resolve(process.cwd(), 'src/tokens.css'), 'utf8')
const css = readFileSync(resolve(process.cwd(), 'src/app.css'), 'utf8')
const escape = (s) => s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
const token = (name) => tokens.match(new RegExp(`${escape(name)}:\\s*([^;]+);`))?.[1].trim()
const rule = (selector) => css.match(new RegExp(`(?:^|\\n)${escape(selector)}\\s*\\{([^}]*)\\}`))?.[1] ?? ''

beforeEach(() => {
  window.location.hash = '#secret-token'
  window.localStorage.clear()
  vi.stubGlobal('fetch', fakeFetch())
})
afterEach(() => {
  vi.unstubAllGlobals()
})

it('uses the neon accent and its tints', () => {
  expect(token('--color-accent')).toBe('#3cf5a8')
  expect(token('--color-accent-soft')).toBe('#3cf5a814')
  expect(token('--color-accent-line')).toBe('#3cf5a859')
  expect(token('--color-accent-bar')).toBe('#3cf5a84d')
})

it('keeps the link colors as their own tokens', () => {
  expect(token('--color-link')).toBe('#7eddb0')
  expect(token('--color-link-hover')).toBe('#b2edcf')
  expect(rule('a')).toContain('var(--color-link)')
  expect(rule('a:hover')).toContain('var(--color-link-hover)')
})

it('defines the four glow tokens with the values of the design', () => {
  expect(token('--glow-text')).toBe('0 0 12px #3cf5a859')
  expect(token('--glow-box')).toBe('0 0 0 1px #3cf5a840, 0 0 18px #3cf5a840')
  expect(token('--glow-dot')).toBe('0 0 8px #3cf5a8')
  expect(token('--glow-line')).toBe('drop-shadow(0 0 3px #3cf5a899)')
})

it('turns the glow off for prefers-contrast: more', () => {
  const block = tokens.match(/@media \(prefers-contrast: more\)\s*\{\s*:root\s*\{([^}]*)\}/)?.[1] ?? ''
  for (const name of ['--glow-text', '--glow-box', '--glow-dot', '--glow-line']) {
    expect(block).toMatch(new RegExp(`${escape(name)}:\\s*none`))
  }
})

it('puts the glow on the elements of the design, each with the matching token', () => {
  const nav = rule(".sidebar nav button[aria-current='page']")
  expect(nav).toContain('text-shadow: var(--glow-text)')
  expect(nav).toContain('inset 0 0 0 1px var(--color-accent-line)')
  expect(rule("[aria-current='page'] > .dot")).toContain('box-shadow: var(--glow-dot)')
  expect(rule('.kpi .spark-line')).toContain('filter: var(--glow-line)')
  expect(rule('.kpi-cache .kpi-value')).toMatch(/color: var\(--color-accent\)[^}]*text-shadow: var\(--glow-text\)/)
  expect(rule('.kpi-cache .progress-fill')).toContain('box-shadow: var(--glow-dot)')
  expect(rule('.bar-latest')).toContain('box-shadow: var(--glow-box)')
  expect(rule('.bar-row-top .bar-fill')).toContain('box-shadow: var(--glow-dot)')
  expect(rule('.ring-value')).toContain('filter: var(--glow-line)')
  expect(rule('.ring-text')).toContain('fill: var(--color-accent)')
  expect(rule('ol.rail-list > li::before')).toContain('text-shadow: var(--glow-text)')
})

it('styles the active period button with the soft bg, accent text and accent-line ring', () => {
  const active = rule(".periods button[aria-pressed='true']")
  expect(active).toContain('background: var(--color-accent-soft)')
  expect(active).toContain('color: var(--color-accent)')
  expect(active).toContain('inset 0 0 0 1px var(--color-accent-line)')
  expect(active).not.toContain('--color-selected')
})

it('gives the shared widgets no glow of their own', () => {
  // Progress bars of limits, agents and the timeline are not in the design's glow list.
  expect(rule('.progress-fill')).not.toMatch(/shadow|filter/)
  expect(rule('.bar')).not.toMatch(/shadow|filter/)
  expect(rule('.bar-fill')).not.toMatch(/shadow|filter/)
})

it('marks the active nav item and the active period button at the default load', async () => {
  render(<App />)
  await screen.findByRole('heading', { name: 'Totals' })
  const nav = screen.getByRole('navigation', { name: 'Views' })
  expect(within(nav).getByRole('button', { name: 'Usage' })).toHaveAttribute('aria-current', 'page')
  const period = screen.getByRole('group', { name: 'Period' })
  expect(within(period).getByRole('button', { name: 'Last 7 days' })).toHaveAttribute('aria-pressed', 'true')
  expect(within(period).getByRole('button', { name: 'Last 30 days' })).toHaveAttribute('aria-pressed', 'false')
  // The cache hit ratio KPI has its own class to hook the accent value and the glow bar.
  expect(screen.getByRole('region', { name: 'Cache hit ratio' })).toHaveClass('kpi-cache')
})
