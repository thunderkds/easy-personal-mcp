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
