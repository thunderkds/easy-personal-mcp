import { fireEvent, render, screen, within } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import Timeline from './Timeline.jsx'
import { sessions, emptySessions } from './test/fixtures.js'

const fmt = (n) => n.toLocaleString('en-US')

describe('session timeline (AC2)', () => {
  it('renders one item per session, in API order', () => {
    render(<Timeline data={sessions} />)
    const items = screen.getAllByRole('listitem').filter((li) => li.dataset.session)
    expect(items.map((li) => li.dataset.session)).toEqual(['abc12345-aaaa', 'single-point'])
  })

  it('shows project, start to end, duration, models, messages and token total', () => {
    render(<Timeline data={sessions} />)
    const item = screen.getByTestId('session-abc12345-aaaa')
    expect(within(item).getByText('demo-app')).toBeInTheDocument()
    expect(item).toHaveTextContent('claude-haiku-5-5, claude-opus-5-5')
    expect(item).toHaveTextContent('2h 0m')
    expect(item).toHaveTextContent('3 messages')
    expect(item).toHaveTextContent(`${fmt(300)} tokens`)
    const start = new Date('2026-10-20T08:00:00.000000Z').toLocaleString()
    const end = new Date('2026-10-20T10:00:00.000000Z').toLocaleString()
    expect(item).toHaveTextContent(`${start} to ${end}`)
  })

  it('draws token use over time: one mark per point, one band per gap', () => {
    render(<Timeline data={sessions} />)
    const chart = screen.getByRole('img', { name: /abc12345.*3 messages.*1 gap/ })
    expect(chart.querySelectorAll('[data-point]')).toHaveLength(3)
    expect(chart.querySelectorAll('[data-gap]')).toHaveLength(1)
  })

  it('lists each gap with its minutes', () => {
    render(<Timeline data={sessions} />)
    const item = screen.getByTestId('session-abc12345-aaaa')
    expect(item).toHaveTextContent(/Gap of 110 min/)
    expect(within(screen.getByTestId('session-single-point')).queryByText(/Gap of/)).toBeNull()
  })

  it('handles a one-point session (zero length) without NaN', () => {
    render(<Timeline data={sessions} />)
    const chart = screen.getByRole('img', { name: /single-point/ })
    expect(chart.outerHTML).not.toMatch(/NaN|Infinity/)
    expect(screen.getByTestId('session-single-point')).toHaveTextContent('0m')
  })

  it('states the gap threshold', () => {
    render(<Timeline data={sessions} />)
    expect(screen.getByText(/pause of 30 min or more/)).toBeInTheDocument()
  })

  it('shows an empty state', () => {
    render(<Timeline data={emptySessions} />)
    expect(screen.getByText(/No sessions in this period/)).toBeInTheDocument()
  })

  describe('top 10 and Show N more (HUN-191, FR-058)', () => {
    const make = (n, extra = {}) => ({
      ...sessions,
      ...extra,
      sessions: Array.from({ length: n }, (_, i) => ({ ...sessions.sessions[0], session_id: `s${i}` })),
    })
    const cards = () => screen.getAllByRole('listitem').filter((li) => li.dataset.session)

    it('AC1: 60 sessions give 10 cards and "Show 50 more"; click lists all 60 and "Show fewer"', () => {
      render(<Timeline data={make(60)} />)
      expect(cards()).toHaveLength(10)
      expect(cards().map((li) => li.dataset.session)).toEqual(Array.from({ length: 10 }, (_, i) => `s${i}`))
      expect(screen.getByRole('heading', { level: 2, name: 'Session timeline (60)' })).toBeInTheDocument()
      const button = screen.getByRole('button', { name: 'Show 50 more' })
      expect(button).toHaveAttribute('aria-expanded', 'false')
      fireEvent.click(button)
      expect(cards()).toHaveLength(60)
      expect(screen.getByRole('button', { name: 'Show fewer' })).toHaveAttribute('aria-expanded', 'true')
      expect(screen.getByRole('heading', { level: 2, name: 'Session timeline (60)' })).toBeInTheDocument()
      fireEvent.click(screen.getByRole('button', { name: 'Show fewer' }))
      expect(cards()).toHaveLength(10)
    })

    it('AC1: the toggle is a native button, so Enter and Space work', () => {
      render(<Timeline data={make(12)} />)
      expect(screen.getByRole('button', { name: 'Show 2 more' })).toHaveAttribute('type', 'button')
    })

    it('AC2: 10 sessions have no button, 11 have one', () => {
      const { unmount } = render(<Timeline data={make(10)} />)
      expect(cards()).toHaveLength(10)
      expect(screen.queryByRole('button')).toBeNull()
      unmount()
      render(<Timeline data={make(11)} />)
      expect(screen.getByRole('button', { name: 'Show 1 more' })).toBeInTheDocument()
    })

    it('AC2: no sessions give the empty text and no button', () => {
      render(<Timeline data={emptySessions} />)
      expect(screen.getByText('No sessions in this period.')).toBeInTheDocument()
      expect(screen.queryByRole('button')).toBeNull()
    })

    it('AC3: with detail_sessions 50, card 51 and later have totals but no chart, no gap count, no gap list', () => {
      render(<Timeline data={make(60, { detail_sessions: 50 })} />)
      fireEvent.click(screen.getByRole('button', { name: 'Show 50 more' }))
      const all = cards()
      for (const li of all.slice(0, 50)) {
        expect(li.querySelector('svg.chart')).not.toBeNull()
        expect(li).toHaveTextContent('1 gap')
        expect(li).toHaveTextContent(/Gap of 110 min/)
        expect(li).not.toHaveTextContent('Chart kept for the newest 50 sessions only.')
      }
      for (const li of all.slice(50)) {
        expect(li.querySelector('svg.chart')).toBeNull()
        expect(li.querySelector('footer')).not.toHaveTextContent(/gap/)
        expect(li).not.toHaveTextContent(/Gap of/)
        expect(li.querySelector('footer')).toHaveTextContent('3 messages')
        expect(li.querySelector('footer')).toHaveTextContent('300 tokens')
        expect(within(li).getByRole('progressbar')).toBeInTheDocument()
        expect(within(li).getByText('Chart kept for the newest 50 sessions only.')).toBeInTheDocument()
      }
    })

    it('AC3: the number in the line comes from detail_sessions', () => {
      render(<Timeline data={make(14, { detail_sessions: 12 })} />)
      fireEvent.click(screen.getByRole('button', { name: 'Show 4 more' }))
      expect(screen.getAllByText('Chart kept for the newest 12 sessions only.')).toHaveLength(2)
    })

    it('AC3: without detail_sessions every card has its chart', () => {
      render(<Timeline data={make(60)} />)
      fireEvent.click(screen.getByRole('button', { name: 'Show 50 more' }))
      expect(document.querySelectorAll('svg.chart')).toHaveLength(60)
      expect(screen.queryByText(/Chart kept for/)).toBeNull()
    })

    it('AC5: the "Showing 50 of N sessions" line is gone', () => {
      render(<Timeline data={make(60, { detail_sessions: 50 })} />)
      expect(screen.queryByText(/Showing \d+ of/)).toBeNull()
    })
  })

  describe('feed cards (HUN-121)', () => {
    const two = {
      ...sessions,
      sessions: [
        { ...sessions.sessions[0], input_tokens: 100, output_tokens: 100, cache_creation_input_tokens: 50, cache_read_input_tokens: 50 },
        { ...sessions.sessions[1], project: 'kit-core', input_tokens: 25, output_tokens: 25, cache_creation_input_tokens: 25, cache_read_input_tokens: 25 },
      ],
    }

    it('SC5: 300 and 100 tokens give two feed cards with token bars at 75 % and 25 %', () => {
      render(<Timeline data={two} />)
      const cards = screen.getAllByRole('article')
      expect(cards).toHaveLength(2)
      const bar = (i) => within(cards[i]).getByRole('progressbar')
      expect(bar(0)).toHaveAttribute('aria-valuenow', '75')
      expect(bar(1)).toHaveAttribute('aria-valuenow', '25')
      expect(bar(0)).toHaveAccessibleName(/share of the period's tokens/i)
    })

    it('has an icon with the project initial, an accent title, a one-line summary and the local start time', () => {
      render(<Timeline data={sessions} />)
      const card = within(screen.getByTestId('session-abc12345-aaaa')).getByRole('article')
      expect(card.querySelector('.feed-icon')).toHaveTextContent('D')
      expect(within(card).getByRole('heading', { name: 'demo-app' })).toBeInTheDocument()
      expect(within(card).getByText('2 models · 3 messages')).toBeInTheDocument()
      expect(card.querySelector('.feed-time')).toHaveTextContent(new Date('2026-10-20T08:00:00.000000Z').toLocaleString())
    })

    it('footer counts messages, tokens and gaps; tool calls only when the data has them', () => {
      render(<Timeline data={sessions} />)
      const foot = within(screen.getByTestId('session-abc12345-aaaa')).getByRole('article').querySelector('footer')
      expect(foot).toHaveTextContent('3 messages')
      expect(foot).toHaveTextContent('300 tokens')
      expect(foot).toHaveTextContent('1 gap')
      expect(foot).not.toHaveTextContent(/tool call/)
    })

    it('shows the tool-call count when the session carries one', () => {
      const withTools = { ...sessions, sessions: [{ ...sessions.sessions[0], tool_calls: 7 }] }
      render(<Timeline data={withTools} />)
      expect(screen.getByRole('article').querySelector('footer')).toHaveTextContent('7 tool calls')
    })

    it('the section header carries the count: "Session timeline (2)"', () => {
      render(<Timeline data={sessions} />)
      expect(screen.getByRole('heading', { level: 2, name: 'Session timeline (2)' })).toBeInTheDocument()
    })

    it('shares are of all sessions of the period, also those folded away', () => {
      const many = { ...sessions, sessions: Array.from({ length: 60 }, (_, i) => ({ ...sessions.sessions[1], session_id: `s${i}` })) }
      render(<Timeline data={many} />)
      expect(within(screen.getAllByRole('article')[0]).getByRole('progressbar')).toHaveAttribute('aria-valuenow', '1.67')
    })
  })
})
