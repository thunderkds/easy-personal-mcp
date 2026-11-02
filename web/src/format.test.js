import { describe, expect, it } from 'vitest'
import { abstainedText, compact, costFigure } from './format.js'

describe('compact (HUN-188 AC1)', () => {
  it('keeps figures below 10,000 exact', () => {
    expect(compact(0)).toBe('0')
    expect(compact(9999)).toBe('9,999')
  })
  it('uses 3 significant digits with K, M, B', () => {
    expect(compact(10000)).toBe('10 K')
    expect(compact(21286)).toBe('21.3 K')
    expect(compact(412345678)).toBe('412 M')
    expect(compact(3107421801)).toBe('3.11 B')
    expect(compact(3111681853)).toBe('3.11 B')
    expect(compact(1_250_000_000_000)).toBe('1250 B')
  })
  it('moves up a unit instead of printing 1000 K', () => {
    expect(compact(999600)).toBe('1 M')
    expect(compact(999600000)).toBe('1 B')
  })
})

describe('cost figure and abstained line (HUN-188 AC2)', () => {
  const cost = (usd, abstained_models = []) => ({ usd, price_table_date: '2026-10-01', abstained_models })
  it('the figure is the dollar amount only', () => {
    expect(costFigure(cost(2138.6))).toBe('$2138.60')
    expect(costFigure(cost(2138.61, ['(unknown)', 'claude-opus-5']))).toBe('$2138.61')
  })
  it('lists abstained models in their own text; empty when none', () => {
    expect(abstainedText(cost(1, ['(unknown)', 'claude-opus-5']))).toBe('abstained: (unknown), claude-opus-5')
    expect(abstainedText(cost(1))).toBe('')
  })
  it('never prints $0 for a missing total', () => {
    expect(costFigure(cost(null, ['m']))).toBe('no total')
  })
})
