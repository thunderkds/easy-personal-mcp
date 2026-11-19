import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { describe, expect, it } from 'vitest'

const tokens = readFileSync(resolve(process.cwd(), 'src/tokens.css'), 'utf8')
const css = readFileSync(resolve(process.cwd(), 'src/app.css'), 'utf8')
const rule = (selector) => css.match(new RegExp(`(?:^|\\n)${selector.replace('.', '\\.')}\\s*\\{([^}]*)\\}`))?.[1] ?? ''

describe('shell container (HUN-128)', () => {
  it('defines --shell-max as 90rem', () => {
    expect(tokens).toMatch(/--shell-max:\s*90rem;/)
  })

  it('caps .shell at --shell-max and centers it', () => {
    const shell = rule('.shell')
    expect(shell).toContain('max-width: var(--shell-max)')
    expect(shell).toContain('margin-inline: auto')
  })

  it('draws the side borders only above the container width, with the divider token', () => {
    expect(css).toMatch(/@media \(min-width: 90\.\d+rem\)\s*\{\s*\.shell\s*\{[^}]*border-inline:\s*1px solid var\(--color-divider\)/)
  })
})

describe('rail groups and badge (HUN-160)', () => {
  it('keeps a hidden group out of the layout even though .nav-group sets display', () => {
    expect(css).toMatch(/\.nav-group\[hidden\]\s*\{\s*display:\s*none;?\s*\}/)
  })

  it('styles the First run badge as plain mono text with tokens only', () => {
    const badge = rule('.badge')
    expect(badge).toContain('var(--font-mono)')
    expect(badge).toContain('var(--color-muted)')
    expect(badge).not.toMatch(/#[0-9a-fA-F]{3,8}\b|rgb\(/)
  })

  it('sets First run font sizes from the type tokens, not raw px (T070)', () => {
    const own = css.split('\n').filter((l) => /first-run/.test(l)).join('\n')
    expect(own).not.toMatch(/font-size:\s*[\d.]+px/)
  })

  it('uses the artboard problem red #E8968F, which holds 4.5:1 on the page (T070)', () => {
    const hex = (name) => tokens.match(new RegExp(`--${name}:\\s*#([0-9a-f]{6})`))[1]
    const lum = (h) => {
      const [r, g, b] = [0, 2, 4].map((i) => parseInt(h.slice(i, i + 2), 16) / 255).map((c) => (c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4))
      return 0.2126 * r + 0.7152 * g + 0.0722 * b
    }
    expect(hex('color-red')).toBe('e8968f')
    expect((lum(hex('color-red')) + 0.05) / (lum(hex('color-bg')) + 0.05)).toBeGreaterThanOrEqual(4.5)
  })

  it('styles the First run cards with tokens only', () => {
    const own = css.split('\n').filter((l) => /first-run/.test(l)).join('\n')
    expect(own).not.toMatch(/#[0-9a-fA-F]{3,8}\b|rgb\(/)
  })
})

describe('toast host and loading pill placement (HUN-202, FR-065)', () => {
  it('keeps the toast host inside the shell box: right edge is --space-4 inside the shell edge on a wide screen', () => {
    const host = rule('.toast-host')
    expect(host).toContain('position: fixed')
    expect(host).toMatch(/right:\s*max\(var\(--space-4\),\s*calc\(\(100% - var\(--shell-max\)\) \/ 2 \+ var\(--space-4\)\)\)/)
    expect(host).toContain('bottom: var(--space-4)')
  })

  it('keeps the phone rule: full width inside a --space-4 gutter', () => {
    expect(css).toMatch(/@media \(max-width: 39\.999rem\)\s*\{\s*\.toast-host\s*\{\s*left: var\(--space-4\);\s*width: auto;/)
  })

  it('centres the loading pill in the middle of the window, fixed, within the screen width', () => {
    const pill = rule('.loading-pill')
    expect(pill).toContain('position: fixed')
    expect(pill).toContain('top: 50%')
    expect(pill).toContain('left: 50%')
    expect(pill).toContain('transform: translate(-50%, -50%)')
    expect(pill).toMatch(/max-width:\s*calc\(100% - 2 \* var\(--space-4\)\)/)
  })

  it('moves the pill to the centre of the main column (right of the sidebar) from 64rem', () => {
    expect(css).toMatch(/@media \(min-width: 64rem\)\s*\{\s*\.loading-pill\s*\{\s*left: calc\(50% \+ var\(--sidebar-width\) \/ 2\);/)
  })
})

describe('Plex Mono everywhere and one step larger text (T120)', () => {
  const appsession = readFileSync(resolve(process.cwd(), 'src/appsession.css'), 'utf8')
  const watch = readFileSync(resolve(process.cwd(), 'src/watch.css'), 'utf8')
  const sheets = { 'app.css': css, 'appsession.css': appsession, 'watch.css': watch }
  const rem32 = (name) => Math.round(parseFloat(tokens.match(new RegExp(`--fs-${name}:\\s*([\\d.]+)rem`))[1]) * 32)
  const SIZES = { label: 23, small: 24, note: 25, cell: 26, control: 27, body: 28, 'card-title': 29, section: 30, brand: 32, h2: 34, kpi: 52, h1: 56 } // the 14 px scale, in 1/32 rem

  it('sets body text to 15 px and scales every size token by 15/14, rounded to 1/32 rem, keeping their order', () => {
    expect(tokens).toMatch(/--fs-body:\s*0\.9375rem;/)
    for (const [name, old] of Object.entries(SIZES)) expect(rem32(name)).toBe(Math.round((old * 15) / 14))
    const order = Object.keys(SIZES).map(rem32)
    expect(order).toEqual([...order].sort((a, b) => a - b))
    expect(new Set(order).size).toBe(order.length)
  })

  it('draws every page font from the mono token: no family literal, no --font-sans user', () => {
    expect(tokens).toMatch(/--font-sans:\s*var\(--font-mono\);/)
    for (const [file, text] of Object.entries(sheets)) {
      expect(text, file).not.toMatch(/font-family:\s*(?!var\(--font-mono\))\S/)
      expect(text, file).not.toContain('var(--font-sans)')
    }
    expect(rule('body')).toContain('font-family: var(--font-mono)')
    expect(css).toMatch(/\ncode, pre, kbd, samp \{ font-family: var\(--font-mono\); \}/) // no browser-default monospace face
  })

  it('hard-codes no font size in any sheet except tokens.css (SVG text included)', () => {
    for (const [file, text] of Object.entries(sheets)) expect(text, file).not.toMatch(/font-size:\s*[\d.]+(?:px|rem)/)
  })

  it('ships real Plex Mono 400, 500 and 600 faces and no Plex Sans, so 600 is never a faked bold', () => {
    for (const w of [400, 500, 600]) expect(tokens).toMatch(new RegExp(`'IBM Plex Mono'[^}]*font-weight: ${w};[^}]*ibm-plex-mono-latin-${w}-normal\\.woff2`))
    expect(tokens).not.toContain('IBM Plex Sans')
  })
})
