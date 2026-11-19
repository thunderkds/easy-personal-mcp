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

describe('Inter for text, Plex Mono for ids and labels, larger text (T120, T121, T123)', () => {
  const appsession = readFileSync(resolve(process.cwd(), 'src/appsession.css'), 'utf8')
  const watch = readFileSync(resolve(process.cwd(), 'src/watch.css'), 'utf8')
  const sheets = { 'app.css': css, 'appsession.css': appsession, 'watch.css': watch }
  const rem32 = (name) => Math.round(parseFloat(tokens.match(new RegExp(`--fs-${name}:\\s*([\\d.]+)rem`))[1]) * 32)
  const SIZES = { label: 28, small: 28, note: 32, cell: 32, control: 32, body: 32, 'card-title': 34, section: 36, brand: 38, h2: 40, kpi: 60, h1: 64 } // in 1/32 rem (round 2: reading text is 16 px, labels 14 px)

  it('sets reading text (note, cell, control, body) to 16 px and labels to 14 px, headings above body, sizes never decreasing in scale order (T121)', () => {
    expect(tokens).toMatch(/--fs-body:\s*1rem;/)
    for (const [name, size] of Object.entries(SIZES)) expect(rem32(name), name).toBe(size)
    const order = Object.keys(SIZES).map(rem32)
    expect(order).toEqual([...order].sort((a, b) => a - b))
    for (const name of ['card-title', 'section', 'brand', 'h2', 'kpi', 'h1']) expect(rem32(name), name).toBeGreaterThan(rem32('body'))
    expect(Math.min(...order)).toBeGreaterThanOrEqual(24) // nothing under 12 px (24/32 rem), labels included
  })

  it('draws every page font from the two family tokens: no family literal; text is Inter, ids/labels/code stay Plex Mono (T123)', () => {
    expect(tokens).toMatch(/--font-sans:\s*'Inter', system-ui, sans-serif;/)
    expect(tokens).toMatch(/--font-mono:\s*'IBM Plex Mono', ui-monospace/)
    for (const [file, text] of Object.entries(sheets)) expect(text, file).not.toMatch(/font-family:\s*(?!var\(--font-(?:mono|sans)\))\S/)
    expect(rule('body')).toContain('font-family: var(--font-sans)')
    expect(rule('.sidebar nav button')).toContain('font-family: var(--font-sans)')
    expect(rule('.scope select, .board-filter select, .actions select, .field input, .field select, .field textarea, .repo-row input')).toContain('font-family: var(--font-sans)')
    for (const sel of ['.mono', '.badge', '.kpi-value', '.count', '.sort', '.ring-text', '.trend']) expect(rule(sel), sel).toContain('font-family: var(--font-mono)') // mono before T120, mono now
    expect(rule('table')).toContain('font-family: var(--font-mono)')
    expect(css).toMatch(/\ncode, pre, kbd, samp \{ font-family: var\(--font-mono\); \}/) // no browser-default monospace face
  })

  it('sets tabular figures once, on body, so every number in Inter text lines up and does not jiggle when it changes (T123)', () => {
    expect(rule('body')).toContain('font-variant-numeric: tabular-nums')
  })
  it('hard-codes no font size in any sheet except tokens.css (SVG text included)', () => {
    for (const [file, text] of Object.entries(sheets)) expect(text, file).not.toMatch(/font-size:\s*[\d.]+(?:px|rem)/)
  })

  it('ships real Inter and Plex Mono 400, 500 and 600 faces and no Plex Sans, so 600 is never a faked bold (T123)', () => {
    for (const [family, file] of [['Inter', 'inter'], ['IBM Plex Mono', 'ibm-plex-mono']]) {
      for (const w of [400, 500, 600]) {
        expect(tokens).toMatch(new RegExp(`'${family}'[^}]*font-weight: ${w};[^}]*${file}-latin-${w}-normal\\.woff2`))
        const bytes = readFileSync(resolve(process.cwd(), `src/fonts/${file}-latin-${w}-normal.woff2`))
        expect(bytes.subarray(0, 4).toString('latin1'), `${file} ${w}`).toBe('wOF2') // a real woff2, not a stub or an HTML error page
        expect(bytes.length, `${file} ${w}`).toBeGreaterThan(10000)
      }
    }
    expect(tokens).not.toContain('IBM Plex Sans')
  })

  it('self-hosts Inter with its OFL notice and no runtime font dependency (T123)', () => {
    expect(readFileSync(resolve(process.cwd(), 'src/fonts/OFL-Inter.txt'), 'utf8')).toMatch(/Inter Project Authors[\s\S]*SIL OPEN FONT LICENSE Version 1\.1/)
    expect(readFileSync(resolve(process.cwd(), 'package.json'), 'utf8')).not.toMatch(/fontsource/)
    expect(tokens).not.toMatch(/url\(['"]?https?:/)
  })
})

describe('round 2 layout rules (T121)', () => {
  const at = (re) => expect(css).toMatch(re)

  it('orders the status chips by urgency at narrow widths: red, then amber, the rest in their order, one row, no scrollbar', () => {
    const narrow = css.slice(css.indexOf('@media (max-width: 63.999rem) { /* T121 F2'), css.indexOf('@media (max-width: 39.999rem) { /* T121 F4'))
    expect(narrow).toMatch(/\.status-chip\.tone-red \{ order: -2; \}/)
    expect(narrow).toMatch(/\.status-chip\.tone-amber \{ order: -1; \}/)
    expect(narrow).toMatch(/scrollbar-width: none/)
    expect(narrow).toMatch(/::-webkit-scrollbar \{ display: none; \}/)
    expect(narrow).toMatch(/overflow-x: auto/)
    expect(narrow).toMatch(/linear-gradient\(to left/) // the edge fade
  })

  it('keeps the disclosure triangle on a summary at phone width (list-item, not flex)', () => {
    const phone = css.slice(css.indexOf('/* T121 F4, F17'), css.indexOf('/* Top bar status chips'))
    expect(phone).toMatch(/main summary \{ display: list-item; min-height: 44px;/)
    expect(phone).not.toMatch(/main summary \{ display: flex/)
  })

  it('makes a checkbox row and a drawer family link 44 px at phone width, the box itself 1.25 rem', () => {
    const phone = css.slice(css.indexOf('/* T121 F4, F17'), css.indexOf('/* Top bar status chips'))
    expect(phone).toMatch(/main label:has\(> input\[type='checkbox'\]\)[^}]*min-height: 44px/)
    expect(phone).toMatch(/main input\[type='checkbox'\][^}]*width: 1\.25rem; height: 1\.25rem; accent-color: var\(--color-accent\)/)
    expect(phone).toMatch(/\.family a \{[^}]*min-height: 44px/)
    at(/\.workflow-gate span \{[^}]*-webkit-line-clamp: 3/)
  })
})
