import { render, screen, within } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import FeedCard from './FeedCard.jsx'

describe('FeedCard (HUN-121)', () => {
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

  it('picks one of the five dot colors from the icon text, the same one each time', () => {
    const { container } = render(
      <>
        <FeedCard id="a" icon="kit-core" title="a" />
        <FeedCard id="b" icon="kit-core" title="b" />
      </>,
    )
    const [one, two] = [...container.querySelectorAll('.feed-icon')]
    expect(one.className).toMatch(/dot-[1-5]/)
    expect(one.className).toBe(two.className)
  })
})
