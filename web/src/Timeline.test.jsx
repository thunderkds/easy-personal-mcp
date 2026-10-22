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
})
