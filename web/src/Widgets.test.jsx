import { fireEvent, render, screen } from '@testing-library/react'
import { expect, it, describe } from 'vitest'
import { Progress, Ring, Sparkline, Trend, useTopN } from './Widgets.jsx'

describe('Trend', () => {
  it('shows an up arrow and the percent change', () => {
    render(<Trend current={1500} previous={1000} />)
    expect(screen.getByText('↑ 50.0%')).toBeInTheDocument()
  })
  it('shows a down arrow when the value fell', () => {
    render(<Trend current={750} previous={1000} />)
    expect(screen.getByText('↓ 25.0%')).toBeInTheDocument()
  })
  it.each([[0], [null], [undefined]])('says no previous data when previous is %s, never Infinity or NaN', (previous) => {
    const { container } = render(<Trend current={5} previous={previous} />)
    expect(screen.getByText('no previous data')).toBeInTheDocument()
    expect(container.textContent).not.toMatch(/Infinity|NaN/)
  })
  it('says no previous data when the current value is unknown', () => {
    render(<Trend current={null} previous={10} />)
    expect(screen.getByText('no previous data')).toBeInTheDocument()
  })
})

describe('Sparkline', () => {
  it('is an image with the label and a finite path', () => {
    render(<Sparkline values={[1, 5, 3]} label="Tokens per day, 3 days, peak 5" />)
    const svg = screen.getByRole('img', { name: 'Tokens per day, 3 days, peak 5' })
    for (const p of svg.querySelectorAll('path')) expect(p.getAttribute('d')).not.toMatch(/NaN|Infinity/)
  })
  it('draws a flat line for one value', () => {
    const { container } = render(<Sparkline values={[7]} label="one day" />)
    expect(container.querySelector('path').getAttribute('d')).not.toMatch(/NaN|Infinity/)
  })
  it('draws a flat line for all-zero values', () => {
    const { container } = render(<Sparkline values={[0, 0]} label="zeros" />)
    expect(container.querySelector('path').getAttribute('d')).not.toMatch(/NaN|Infinity/)
  })
  it('renders nothing for no values', () => {
    const { container } = render(<Sparkline values={[]} label="none" />)
    expect(container).toBeEmptyDOMElement()
  })
})

describe('Ring and Progress', () => {
  it('ring shows the percent and is labelled', () => {
    render(<Ring ratio={0.9184} label="Cache hit ratio 91.8%" />)
    expect(screen.getByRole('img', { name: 'Cache hit ratio 91.8%' })).toBeInTheDocument()
    expect(screen.getByText('91.8%')).toBeInTheDocument()
  })
  it('ring with a null ratio says no data, not 0%', () => {
    const { container } = render(<Ring ratio={null} label="Cache hit ratio: no data" />)
    expect(screen.getByText('no data')).toBeInTheDocument()
    expect(container.textContent).not.toMatch(/0\.0%|0%/)
  })
  it('progress exposes its value', () => {
    render(<Progress ratio={0.5} label="Cache hit ratio" />)
    const bar = screen.getByRole('progressbar', { name: 'Cache hit ratio' })
    expect(bar).toHaveAttribute('aria-valuenow', '50')
  })
  it('progress with a null ratio says no data and has no bar', () => {
    render(<Progress ratio={null} label="Cache hit ratio" />)
    expect(screen.getByText('no data')).toBeInTheDocument()
    expect(screen.queryByRole('progressbar')).toBeNull()
  })
})

describe('useTopN (HUN-190)', () => {
  function List({ n, limit }) {
    const { shown, toggle } = useTopN(Array.from({ length: n }, (_, i) => i + 1), limit)
    return (
      <div>
        <ul>{shown.map((x) => <li key={x}>row {x}</li>)}</ul>
        {toggle}
      </div>
    )
  }
  it('shows the first 10 and "Show N more"; click shows all, then "Show fewer"', () => {
    render(<List n={25} />)
    expect(screen.getAllByRole('listitem')).toHaveLength(10)
    expect(screen.getByText('row 10')).toBeInTheDocument()
    expect(screen.queryByText('row 11')).toBeNull()
    const btn = screen.getByRole('button', { name: 'Show 15 more' })
    expect(btn).toHaveAttribute('aria-expanded', 'false')
    expect(btn).toHaveClass('details-toggle')
    fireEvent.click(btn)
    expect(screen.getAllByRole('listitem')).toHaveLength(25)
    expect(screen.getByRole('button', { name: 'Show fewer' })).toHaveAttribute('aria-expanded', 'true')
    fireEvent.click(screen.getByRole('button', { name: 'Show fewer' }))
    expect(screen.getAllByRole('listitem')).toHaveLength(10)
  })
  it.each([[0], [1], [10]])('%s rows: all shown, no button', (n) => {
    render(<List n={n} />)
    expect(screen.queryAllByRole('listitem')).toHaveLength(n)
    expect(screen.queryByRole('button')).toBeNull()
  })
  it('11 rows: "Show 1 more"', () => {
    render(<List n={11} />)
    expect(screen.getByRole('button', { name: 'Show 1 more' })).toBeInTheDocument()
  })
  it('a limit of Infinity never folds', () => {
    render(<List n={25} limit={Infinity} />)
    expect(screen.getAllByRole('listitem')).toHaveLength(25)
    expect(screen.queryByRole('button')).toBeNull()
  })
  it('the button is a focusable <button type="button"> (Enter and Space toggle a native button)', () => {
    render(<List n={12} />)
    const btn = screen.getByRole('button', { name: 'Show 2 more' })
    expect(btn.tagName).toBe('BUTTON')
    expect(btn).toHaveAttribute('type', 'button')
    btn.focus()
    expect(btn).toHaveFocus()
  })
  it('formats a large count with separators', () => {
    render(<List n={1010} />)
    expect(screen.getByRole('button', { name: 'Show 1,000 more' })).toBeInTheDocument()
  })
})
