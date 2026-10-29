import { render, screen, within } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import FeedCard from './FeedCard.jsx'

describe('FeedCard (HUN-121, HUN-126)', () => {
  it('has a round icon with the initial, a title, a summary, a right-aligned time, a body and a footer', () => {
    render(
      <FeedCard id="x" icon="demo-app" title="demo-app" summary="3 models · 42 messages" time="10:00" footer={<span>42 messages</span>}>
        <p>chart</p>
      </FeedCard>,
    )
    const card = screen.getByRole('article', { name: 'demo-app' })
    expect(within(card).getByRole('heading', { name: 'demo-app' })).toBeInTheDocument()
    expect(card.querySelector('.feed-icon')).toHaveTextContent('D')
    expect(card.querySelector('.feed-icon')).toHaveAttribute('aria-hidden', 'true')
    expect(within(card).getByText('3 models · 42 messages')).toBeInTheDocument()
    expect(card.querySelector('.feed-time')).toHaveTextContent('10:00')
    expect(within(card).getByText('chart')).toBeInTheDocument()
    expect(card.querySelector('footer.feed-foot')).toHaveTextContent('42 messages')
  })

  it('omits the time and the footer when not given', () => {
    render(<FeedCard id="y" icon="a" title="alpha" />)
    const card = screen.getByRole('article', { name: 'alpha' })
    expect(card.querySelector('.feed-time')).toBeNull()
    expect(card.querySelector('footer')).toBeNull()
  })

  it('the icon tile is mint by default and amber with tone="amber"', () => {
    const { container } = render(
      <>
        <FeedCard id="a" icon="1" title="a" />
        <FeedCard id="b" icon="2" title="b" tone="amber" />
      </>,
    )
    const [one, two] = [...container.querySelectorAll('.feed-icon')]
    expect(one).not.toHaveClass('tone-amber')
    expect(two).toHaveClass('tone-amber')
  })
})
