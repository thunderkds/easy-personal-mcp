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

  it('styles the "N left" badge with tokens only', () => {
    const badge = rule('.badge')
    expect(badge).toContain('var(--color-accent')
    expect(badge).not.toMatch(/#[0-9a-fA-F]{3,8}\b|rgb\(/)
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
