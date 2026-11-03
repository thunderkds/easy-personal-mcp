import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { act, fireEvent, render, screen, within } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { ToastProvider, useNotify } from './Toast.jsx'

const css = readFileSync(resolve(process.cwd(), 'src/app.css'), 'utf8')

function Buttons() {
  const notify = useNotify()
  return (
    <>
      <button onClick={() => notify('success', 'Saved one')}>ok</button>
      <button onClick={() => notify('error', 'Could not complete: nope')}>fail</button>
      <button onClick={() => notify('success', '<b>bold</b>')}>html</button>
      <button
        onClick={() => {
          for (const n of [1, 2, 3, 4]) notify('error', `Error ${n}`)
        }}
      >
        four
      </button>
    </>
  )
}
const setup = () =>
  render(
    <ToastProvider>
      <Buttons />
    </ToastProvider>,
  )
const host = () => screen.getByTestId('toasts')

beforeEach(() => vi.useFakeTimers())
afterEach(() => vi.useRealTimers())

describe('toast host (AC5, AC6)', () => {
  it('shows a success toast as role=status inside the host and hides it after 5 s', () => {
    setup()
    fireEvent.click(screen.getByText('ok'))
    const toast = within(host()).getByRole('status')
    expect(toast).toHaveTextContent('Saved one')
    act(() => vi.advanceTimersByTime(4999))
    expect(within(host()).getByRole('status')).toBeInTheDocument()
    act(() => vi.advanceTimersByTime(1))
    expect(within(host()).queryByRole('status')).toBeNull()
  })

  it('keeps an error toast (role=alert) until its close button is pressed', () => {
    setup()
    fireEvent.click(screen.getByText('fail'))
    expect(within(host()).getByRole('alert')).toHaveTextContent('Could not complete: nope')
    act(() => vi.advanceTimersByTime(120000))
    const toast = within(host()).getByRole('alert')
    fireEvent.click(within(toast).getByRole('button', { name: 'Close notification' }))
    expect(within(host()).queryByRole('alert')).toBeNull()
  })

  it('has a close button on a success toast too', () => {
    setup()
    fireEvent.click(screen.getByText('ok'))
    fireEvent.click(within(host()).getByRole('button', { name: 'Close notification' }))
    expect(within(host()).queryByRole('status')).toBeNull()
  })

  it('keeps at most 3 toasts and drops the oldest, newest last', () => {
    setup()
    fireEvent.click(screen.getByText('four'))
    const alerts = within(host()).getAllByRole('alert')
    expect(alerts.map((a) => a.textContent)).toEqual(['Error 2×', 'Error 3×', 'Error 4×'])
  })

  it('shows two quick actions as two toasts', () => {
    setup()
    fireEvent.click(screen.getByText('ok'))
    fireEvent.click(screen.getByText('fail'))
    expect(within(host()).getByRole('status')).toBeInTheDocument()
    expect(within(host()).getByRole('alert')).toBeInTheDocument()
  })

  it('renders the message as plain text, never as HTML', () => {
    setup()
    fireEvent.click(screen.getByText('html'))
    expect(within(host()).getByRole('status')).toHaveTextContent('<b>bold</b>')
    expect(host().querySelector('b')).toBeNull()
  })

  it('does not warn or throw when the host unmounts with a timer running', () => {
    const spy = vi.spyOn(console, 'error').mockImplementation(() => {})
    const view = setup()
    fireEvent.click(screen.getByText('ok'))
    view.unmount()
    act(() => vi.advanceTimersByTime(10000))
    expect(spy).not.toHaveBeenCalled()
    spy.mockRestore()
  })

  it('refuses useNotify outside a ToastProvider', () => {
    const spy = vi.spyOn(console, 'error').mockImplementation(() => {})
    expect(() => render(<Buttons />)).toThrow(/ToastProvider/)
    spy.mockRestore()
  })
})

describe('toast and loading styles (design system)', () => {
  const block = (selector) => css.match(new RegExp(`(?:^|\\n)${selector.replace(/[.[\]]/g, '\\$&')}\\s*\\{([^}]*)\\}`))?.[1] ?? ''

  it('fixes the host bottom-right with a 16 px (--space-4) gutter', () => {
    const rule = block('.toast-host')
    expect(rule).toContain('position: fixed')
    expect(rule).toContain('right: var(--space-4)')
    expect(rule).toContain('bottom: var(--space-4)')
  })

  it('makes the host full width at the bottom on a phone', () => {
    expect(css).toMatch(/@media \(max-width: 39\.999rem\)\s*\{[^@]*\.toast-host\s*\{[^}]*left: var\(--space-4\)/)
  })

  it('fixes the loading pill and uses tokens only', () => {
    const rule = block('.loading-pill')
    expect(rule).toContain('position: fixed')
    const own = css.split('\n').filter((l) => /toast|loading-pill|spinner/.test(l)).join('\n')
    expect(own).not.toMatch(/#[0-9a-fA-F]{3,8}\b|rgb\(/)
  })

  it('turns the motion off under prefers-reduced-motion', () => {
    expect(css).toMatch(/@media \(prefers-reduced-motion: reduce\)\s*\{[^@]*\.spinner[^}]*animation: none/)
    expect(css).toMatch(/@media \(prefers-reduced-motion: reduce\)\s*\{[^@]*\.toast[^}]*animation: none/)
  })
})
