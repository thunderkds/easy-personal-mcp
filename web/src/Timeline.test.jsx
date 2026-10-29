import { render, screen, within } from '@testing-library/react'
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

  it('caps the list and says how many are hidden', () => {
    const many = { ...sessions, sessions: Array.from({ length: 60 }, (_, i) => ({ ...sessions.sessions[1], session_id: `s${i}` })) }
    render(<Timeline data={many} />)
    expect(screen.getAllByRole('listitem').filter((li) => li.dataset.session)).toHaveLength(50)
    expect(screen.getByText(/Showing 50 of 60 sessions/)).toBeInTheDocument()
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

    it('shares are of all sessions of the period, also those beyond the 50 shown', () => {
      const many = { ...sessions, sessions: Array.from({ length: 60 }, (_, i) => ({ ...sessions.sessions[1], session_id: `s${i}` })) }
      render(<Timeline data={many} />)
      expect(within(screen.getAllByRole('article')[0]).getByRole('progressbar')).toHaveAttribute('aria-valuenow', '1.67')
    })
  })
})
